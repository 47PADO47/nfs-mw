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
    /// Per target format, indexed by `index(blend, shading)`. A format gets its pipelines the first time
    /// a scene is drawn into it, and a glossy pipeline only once glossy shading is in use.
    sets: HashMap<wgpu::TextureFormat, Vec<Option<wgpu::RenderPipeline>>>,
    format: wgpu::TextureFormat,
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

const ADDITIVE: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent::OVER,
};

const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4, 3 => Float32x2];

impl Pipelines {
    /// The pipelines for scenes drawn into `format`, except the glossy ones (see [`Self::use_format`]).
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat, shared: &Shared) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/scene.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene"),
            bind_group_layouts: &[Some(&shared.globals_layout), Some(&shared.texture_layout)],
            immediate_size: 0,
        });
        let mut pipelines = Self {
            scene: Module { shader, layout },
            glossy: None,
            glossy_layouts: shared.glossy.clone(),
            globals_layout: shared.globals_layout.clone(),
            texture_layout: shared.texture_layout.clone(),
            sets: HashMap::new(),
            format,
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
        let mut set = self.sets.remove(&format).unwrap_or_else(|| vec![None; BLEND_ORDER.len() * SHADINGS.len()]);
        for (blend, shading) in BLEND_ORDER.iter().flat_map(|&b| SHADINGS.iter().map(move |&s| (b, s))) {
            let slot = &mut set[index(blend, shading)];
            let wanted = glossy || !matches!(shading, Shading::Glossy(_));
            if slot.is_none() && wanted {
                *slot = Some(self.make(device, format, blend, shading));
            }
        }
        self.sets.insert(format, set);
    }

    /// How many pipelines exist for `format`.
    #[cfg(test)]
    pub(super) fn count(&self, format: wgpu::TextureFormat) -> usize {
        self.sets.get(&format).map_or(0, |set| set.iter().flatten().count())
    }

    /// Whether the glossy shader has been compiled.
    #[cfg(test)]
    pub(super) fn has_glossy(&self) -> bool {
        self.glossy.is_some()
    }

    fn glossy_module(&self, device: &wgpu::Device) -> Module {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("glossy shader"),
            source: wgpu::ShaderSource::Wgsl(super::glossy::SHADER.into()),
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

    fn make(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        blend_mode: BlendMode,
        shading: Shading,
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
        let label = format!("{blend_mode:?} {shading:?} {format:?}");
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
                // Reverse Z: near = 1, far = 0.
                depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
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
            cache: None,
        })
    }

    /// The pipeline for the current format, or `None` for glossy shading before it is in use.
    pub(super) fn get(&self, blend: BlendMode, shading: Shading) -> Option<&wgpu::RenderPipeline> {
        self.sets.get(&self.format)?.get(index(blend, shading))?.as_ref()
    }
}
