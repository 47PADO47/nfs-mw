//! The scene pipelines, one per target format × blend mode × shading model, built when first needed,
//! and the vertex layouts.

use std::collections::HashMap;

use super::glossy::Layouts;
use super::instances::INSTANCE_LAYOUT;
use super::resources::{DEPTH_FORMAT, Shared};
use super::targets::write_mask;
use crate::{BlendMode, GlossyMaterialHandle, Shading, Vertex};

/// Draw order: opaque and alpha-tested first, blended last.
pub(super) const BLEND_ORDER: [BlendMode; 4] =
    [BlendMode::Opaque, BlendMode::AlphaTest, BlendMode::AlphaBlend, BlendMode::Additive];
pub(super) const SHADINGS: [Shading; 4] =
    [Shading::Lit, Shading::Prelit, Shading::Sky, Shading::Glossy(GlossyMaterialHandle::ANY)];

pub(super) struct Pipelines {
    scene: Module,
    /// The glossy shader and layout, built when the first glossy pipeline is asked for.
    glossy: Option<Module>,
    glossy_layouts: Layouts,
    globals_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    /// Per target format and depth test (`(format, depth_prepared)`), indexed by `index(blend, shading)`. A format
    /// gets its pipelines the first time a scene is drawn into it, and a glossy pipeline only once glossy shading
    /// is in use. The depth-prepared set holds only the draws [`equal_depth`] allows; the rest use the plain set.
    sets: HashMap<(wgpu::TextureFormat, bool), Vec<Option<wgpu::RenderPipeline>>>,
    format: wgpu::TextureFormat,
    /// Used by every pipeline built later, as the pipelines are created per format and draw state.
    cache: Option<wgpu::PipelineCache>,
}

/// A shader module and the pipeline layout it is used with.
struct Module {
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
}

fn index(blend: BlendMode, shading: Shading) -> usize {
    let b = BLEND_ORDER.iter().position(|&m| m == blend).unwrap_or(0);
    let s = SHADINGS.iter().position(|&m| m.same_pipeline(shading)).unwrap_or(0);
    b * SHADINGS.len() + s
}

/// Whether an opaque lit draw can test `Equal` against the camera prepass's depth. The prepass draws every opaque
/// draw at the same clip position (`@invariant` in both shaders, the same model and view-projection), so a visible
/// fragment has exactly the depth the prepass wrote. Glossy draws (the prepass skips those without their material),
/// sky, blended and alpha-tested draws keep the plain test: the prepass does not always cover them, or it samples
/// its cutout with another LOD bias than the colour pass.
fn equal_depth(blend: BlendMode, shading: Shading) -> bool {
    blend == BlendMode::Opaque && matches!(shading, Shading::Lit | Shading::Prelit)
}

const ADDITIVE: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent::OVER,
};

pub(super) const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4, 3 => Float32x2];

impl Pipelines {
    /// The pipelines for scenes drawn into `format`, except the glossy ones (see [`Self::use_format`]).
    pub(super) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        shared: &Shared,
        cache: Option<&wgpu::PipelineCache>,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene shader"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    include_str!("../shaders/scene.wgsl"),
                    include_str!("../shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "2")
                )
                .into(),
            ),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene"),
            bind_group_layouts: &[
                Some(&shared.bindings.globals_layout),
                Some(&shared.bindings.texture_layout),
                Some(&shared.glossy.material),
            ],
            immediate_size: 0,
        });
        let mut pipelines = Self {
            scene: Module { shader, layout },
            glossy: None,
            glossy_layouts: shared.glossy.clone(),
            globals_layout: shared.bindings.globals_layout.clone(),
            texture_layout: shared.bindings.texture_layout.clone(),
            sets: HashMap::new(),
            format,
            cache: cache.cloned(),
        };
        pipelines.use_format(device, format, false);
        pipelines
    }

    /// Draw into `format` from now on, building what is missing for it: its pipelines the first time,
    /// and the glossy ones the first time `glossy` is set. Everything built stays for later switches.
    pub(super) fn use_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat, glossy: bool) {
        self.format = format;
        if glossy && self.glossy.is_none() {
            self.glossy = Some(self.glossy_module(device));
        }
        for prepared in [false, true] {
            let mut set =
                self.sets.remove(&(format, prepared)).unwrap_or_else(|| vec![None; BLEND_ORDER.len() * SHADINGS.len()]);
            for (blend, shading) in BLEND_ORDER.iter().flat_map(|&b| SHADINGS.iter().map(move |&s| (b, s))) {
                let slot = &mut set[index(blend, shading)];
                let wanted = match prepared {
                    true => equal_depth(blend, shading),
                    false => glossy || !matches!(shading, Shading::Glossy(_)),
                };
                if slot.is_none() && wanted {
                    *slot = Some(self.make(device, format, blend, shading, prepared));
                }
            }
            self.sets.insert((format, prepared), set);
        }
    }

    /// How many plain pipelines exist for `format` (the depth-prepared variants are not counted).
    #[cfg(test)]
    pub(super) fn count(&self, format: wgpu::TextureFormat) -> usize {
        self.sets.get(&(format, false)).map_or(0, |set| set.iter().flatten().count())
    }

    /// Whether the glossy shader has been compiled.
    #[cfg(test)]
    pub(super) fn has_glossy(&self) -> bool {
        self.glossy.is_some()
    }

    fn glossy_module(&self, device: &wgpu::Device) -> Module {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("glossy shader"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    super::glossy::SHADER,
                    include_str!("../shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "3")
                )
                .into(),
            ),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("glossy"),
            bind_group_layouts: &[
                Some(&self.globals_layout),
                Some(&self.texture_layout),
                Some(&self.glossy_layouts.scene),
                Some(&self.glossy_layouts.material),
            ],
            immediate_size: 0,
        });
        Module { shader, layout }
    }

    /// `prepared` builds the depth-prepared variant: `Equal` with no depth write. Only [`equal_depth`] draws
    /// may be built that way, and only in a pass whose depth the camera prepass already filled.
    fn make(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        blend_mode: BlendMode,
        shading: Shading,
        prepared: bool,
    ) -> wgpu::RenderPipeline {
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        };
        let (entry, blend, depth_write) = match blend_mode {
            BlendMode::Opaque => ("fs_opaque", None, true),
            BlendMode::AlphaTest => ("fs_alpha_test", None, true),
            BlendMode::AlphaBlend => ("fs_blend", Some(wgpu::BlendState::ALPHA_BLENDING), false),
            BlendMode::Additive => ("fs_blend", Some(ADDITIVE), false),
        };
        let label = match prepared {
            true => format!("{blend_mode:?} {shading:?} {format:?} depth prepared"),
            false => format!("{blend_mode:?} {shading:?} {format:?}"),
        };
        // The prepass wrote this depth with the same clip position, so the draw keeps it: no write needed.
        let (depth_compare, depth_write) = match prepared {
            true => (wgpu::CompareFunction::Equal, false),
            false => (wgpu::CompareFunction::GreaterEqual, depth_write),
        };
        let prelit = matches!(shading, Shading::Prelit | Shading::Sky);
        let constants =
            [("PRELIT", f64::from(u8::from(prelit))), ("FOG", f64::from(u8::from(shading != Shading::Sky)))];
        // The glossy shader has its own module, layout and no overrides.
        let glossy = self.glossy.as_ref().filter(|_| matches!(shading, Shading::Glossy(_)));
        let (module, constants): (_, &[(&str, f64)]) = match glossy {
            Some(glossy) => (glossy, &[]),
            None => (&self.scene, &constants),
        };
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&label),
            layout: Some(&module.layout),
            vertex: wgpu::VertexState {
                module: &module.shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(vertex_layout), Some(INSTANCE_LAYOUT)],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                // The games' culling per effect is not mapped yet; draw both sides.
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(depth_write),
                // Reverse Z: near = 1, far = 0. A prepared draw passes only where the prepass depth is its own.
                depth_compare: Some(depth_compare),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module.shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions { constants, ..Default::default() },
                targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: write_mask(format) })],
            }),
            multiview_mask: None,
            cache: self.cache.as_ref(),
        })
    }

    /// The pipeline for the current format, or `None` for glossy shading before it is in use. With
    /// `depth_prepared` (the camera prepass filled this pass's depth) an [`equal_depth`] draw takes its
    /// depth-prepared variant; every other draw keeps the plain one.
    pub(super) fn get(
        &self,
        blend: BlendMode,
        shading: Shading,
        depth_prepared: bool,
    ) -> Option<&wgpu::RenderPipeline> {
        let i = index(blend, shading);
        match depth_prepared {
            true => self.slot(true, i).or_else(|| self.slot(false, i)),
            false => self.slot(false, i),
        }
    }

    fn slot(&self, prepared: bool, i: usize) -> Option<&wgpu::RenderPipeline> {
        self.sets.get(&(self.format, prepared))?.get(i)?.as_ref()
    }
}
