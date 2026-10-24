//! The scene pipelines, one per target format × blend mode × shading model, built when first needed,
//! and the vertex layouts.

use std::collections::HashMap;

/// Pipeline tables are looked up per draw bucket: a fast hash, as the keys are small and not attacker-chosen.
type FastMap<K, V> = HashMap<K, V, foldhash::fast::FixedState>;

use super::cull::{FACES, Face};
use super::draw_list::{instance_store, instance_view};
use super::glossy::Layouts;
use super::instances::INSTANCE_LAYOUT;
use super::parallel;
use super::resources::{DEPTH_FORMAT, Shared};
use super::targets::write_mask;
use crate::{BlendMode, GlossyMaterialHandle, Shading, Vertex};

/// Draw order: opaque and alpha-tested first, blended last.
pub(super) const BLEND_ORDER: [BlendMode; 4] =
    [BlendMode::Opaque, BlendMode::AlphaTest, BlendMode::AlphaBlend, BlendMode::Additive];
pub(super) const SHADINGS: [Shading; 8] = [
    Shading::Lit,
    Shading::Prelit,
    Shading::Sky,
    Shading::World,
    Shading::Tree,
    Shading::WorldNoFog,
    Shading::WorldPrelit,
    Shading::Glossy(GlossyMaterialHandle::ANY),
];

pub(super) struct Pipelines {
    scene: Module,
    /// The glossy shader and layout, built when the first glossy pipeline is asked for.
    glossy: Option<Module>,
    glossy_layouts: Layouts,
    globals_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    /// Per target format, depth test and clip plane (`(format, depth_prepared, clip)`), indexed by
    /// `index(blend, shading, face)`. A format gets its pipelines the first time a scene is drawn into it, a glossy
    /// pipeline only once glossy shading is in use, and the culling variants only once a single-sided draw exists.
    /// The depth-prepared set holds only the draws [`equal_depth`] allows; the rest use the plain set. The clip sets
    /// exist only once a planar capture (or the prewarm for one) asks for them.
    sets: FastMap<(wgpu::TextureFormat, bool, bool), Vec<Option<wgpu::RenderPipeline>>>,
    /// The `(format, glossy, culled, clip)` requests of [`Self::use_format`] that left nothing missing, so a repeated
    /// request (every view of a frame makes one) returns without walking the sets.
    complete: Vec<(wgpu::TextureFormat, bool, bool, bool)>,
    /// Draws discard fragments behind the view's clip plane (a planar capture): the pipelines of the clip sets are
    /// picked and built from [`Self::use_format`] on. The main view leaves it off, keeping early depth testing.
    clip: bool,
    /// A mesh with a single-sided draw was created: the culling variants are built from then on.
    culled: bool,
    format: wgpu::TextureFormat,
    /// Used by every pipeline built later, as the pipelines are created per format and draw state.
    cache: Option<wgpu::PipelineCache>,
    /// The scene shader reads its instances from the instance store (bind group 3) through a slot per instance.
    /// The glossy shader keeps reading matrices.
    store: bool,
}

/// A shader module and the pipeline layout it is used with.
struct Module {
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
}

fn index(blend: BlendMode, shading: Shading, face: Face) -> usize {
    let b = BLEND_ORDER.iter().position(|&m| m == blend).unwrap_or(0);
    let s = SHADINGS.iter().position(|&m| m.same_pipeline(shading)).unwrap_or(0);
    (face.index() * BLEND_ORDER.len() + b) * SHADINGS.len() + s
}

/// Pipelines in one set: every blend mode and shading, for every face setting.
const SET_LEN: usize = FACES.len() * BLEND_ORDER.len() * SHADINGS.len();

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
        let store = instance_store::supported(device);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene shader"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}\n{}",
                    instance_view::instance_source(include_str!("../shaders/scene.wgsl"), store.then_some(3)),
                    include_str!("../shaders/world_surface.wgsl"),
                    include_str!("../shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "2")
                )
                .into(),
            ),
        });
        let instances = store.then(|| instance_store::layout(device));
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene"),
            bind_group_layouts: &[
                Some(&shared.bindings.globals_layout),
                Some(&shared.bindings.texture_layout),
                Some(&shared.glossy.material),
                instances.as_ref(),
            ][..3 + usize::from(store)],
            immediate_size: 0,
        });
        let mut pipelines = Self {
            scene: Module { shader, layout },
            glossy: None,
            glossy_layouts: shared.glossy.clone(),
            globals_layout: shared.bindings.globals_layout.clone(),
            texture_layout: shared.bindings.texture_layout.clone(),
            sets: FastMap::default(),
            complete: Vec::new(),
            culled: false,
            clip: false,
            format,
            cache: cache.cloned(),
            store,
        };
        pipelines.use_format(device, format, false);
        pipelines
    }

    /// Draw into `format` from now on, building what is missing for it: its pipelines the first time, the glossy
    /// ones the first time `glossy` is set, and the culling variants once [`Self::enable_culling`] was called.
    /// Everything built stays for later switches.
    pub(super) fn use_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat, glossy: bool) {
        self.format = format;
        let request = (format, glossy, self.culled, self.clip);
        if self.complete.contains(&request) {
            return;
        }
        if glossy && self.glossy.is_none() {
            self.glossy = Some(self.glossy_module(device));
        }
        let faces = match self.culled {
            true => &FACES[..],
            false => &FACES[..1],
        };
        for prepared in [false, true] {
            self.sets.entry((format, prepared, self.clip)).or_insert_with(|| vec![None; SET_LEN]);
        }
        let clip = self.clip;
        let missing: Vec<(BlendMode, Shading, Face, bool)> = [false, true]
            .into_iter()
            .flat_map(|prepared| faces.iter().map(move |&f| (f, prepared)))
            .flat_map(|(f, prepared)| {
                BLEND_ORDER.iter().flat_map(move |&b| SHADINGS.iter().map(move |&s| (b, s, f, prepared)))
            })
            .filter(|&(blend, shading, face, prepared)| {
                let wanted = match prepared {
                    // A clipped capture never has a camera prepass, so it needs no depth-prepared set.
                    true => !clip && equal_depth(blend, shading),
                    false => glossy || !matches!(shading, Shading::Glossy(_)),
                };
                wanted && self.sets[&(format, prepared, clip)][index(blend, shading, face)].is_none()
            })
            .collect();
        // The variants are independent, and a cold driver cache spends up to half a second on each: build them
        // all at once.
        let built = parallel::build(&missing, |&(blend, shading, face, prepared)| {
            self.make(device, format, (blend, shading, face), (prepared, clip))
        });
        for ((blend, shading, face, prepared), pipeline) in missing.into_iter().zip(built) {
            let Some(set) = self.sets.get_mut(&(format, prepared, clip)) else { continue };
            set[index(blend, shading, face)] = Some(pipeline);
        }
        self.complete.push(request);
    }

    /// Pick (and from the next [`Self::use_format`] on, build) the variants that discard fragments behind the clip
    /// plane. Only a planar-reflection capture sets it: the discard would disable early depth for the main view.
    pub(super) fn use_clip(&mut self, clip: bool) {
        self.clip = clip;
    }

    /// Build the culling variants from the next [`Self::use_format`] on: a single-sided draw exists.
    pub(super) fn enable_culling(&mut self) {
        self.culled = true;
    }

    /// Whether [`Self::enable_culling`] was called.
    pub(super) fn culled(&self) -> bool {
        self.culled
    }

    /// How many plain pipelines exist for `format` (the depth-prepared variants are not counted).
    #[cfg(test)]
    pub(super) fn count(&self, format: wgpu::TextureFormat) -> usize {
        self.sets.get(&(format, false, false)).map_or(0, |set| set.iter().flatten().count())
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
    /// may be built that way, and only in a pass whose depth the camera prepass already filled. `clip` builds the
    /// variant that discards fragments behind the view's clip plane.
    fn make(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        (blend_mode, shading, face): (BlendMode, Shading, Face),
        (prepared, clip): (bool, bool),
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
        let label = match (prepared, clip) {
            (true, true) => format!("{blend_mode:?} {shading:?} {face:?} {format:?} depth prepared clipped"),
            (true, false) => format!("{blend_mode:?} {shading:?} {face:?} {format:?} depth prepared"),
            (false, true) => format!("{blend_mode:?} {shading:?} {face:?} {format:?} clipped"),
            (false, false) => format!("{blend_mode:?} {shading:?} {face:?} {format:?}"),
        };
        // The prepass wrote this depth with the same clip position, so the draw keeps it: no write needed.
        let (depth_compare, depth_write) = match prepared {
            true => (wgpu::CompareFunction::Equal, false),
            false => (wgpu::CompareFunction::GreaterEqual, depth_write),
        };
        let prelit = !matches!(shading, Shading::Lit | Shading::Glossy(_));
        let world_kind = match shading {
            Shading::World => 1.0,
            Shading::Tree => 2.0,
            Shading::WorldPrelit => 3.0,
            Shading::WorldNoFog => 4.0,
            _ => 0.0,
        };
        let clip_plane = f64::from(u8::from(clip));
        let constants = [
            ("PRELIT", f64::from(u8::from(prelit))),
            ("FOG", f64::from(u8::from(shading != Shading::Sky))),
            ("WORLD_KIND", world_kind),
            ("CLIP_PLANE", clip_plane),
        ];
        // The glossy shader has its own module and layout, and only the clip override.
        let glossy_constants = [("CLIP_PLANE", clip_plane)];
        let glossy = self.glossy.as_ref().filter(|_| matches!(shading, Shading::Glossy(_)));
        let (module, constants, instances): (_, &[(&str, f64)], _) = match glossy {
            Some(glossy) => (glossy, &glossy_constants, INSTANCE_LAYOUT),
            None => (&self.scene, &constants, instance_view::instance_layout(self.store)),
        };
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&label),
            layout: Some(&module.layout),
            vertex: wgpu::VertexState {
                module: &module.shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions { constants, ..Default::default() },
                buffers: &[Some(vertex_layout), Some(instances)],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: face.cull_mode(),
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
    /// depth-prepared variant; every other draw keeps the plain one. A culling `face` whose variant is not built
    /// yet (before [`Self::enable_culling`]) takes the two-sided pipeline, which draws a superset.
    pub(super) fn get(
        &self,
        blend: BlendMode,
        shading: Shading,
        depth_prepared: bool,
        face: Face,
    ) -> Option<&wgpu::RenderPipeline> {
        self.get_face(blend, shading, depth_prepared, face)
            .or_else(|| self.get_face(blend, shading, depth_prepared, Face::Both))
    }

    fn get_face(
        &self,
        blend: BlendMode,
        shading: Shading,
        depth_prepared: bool,
        face: Face,
    ) -> Option<&wgpu::RenderPipeline> {
        let i = index(blend, shading, face);
        match depth_prepared {
            true => self.slot(true, i).or_else(|| self.slot(false, i)),
            false => self.slot(false, i),
        }
    }

    fn slot(&self, prepared: bool, i: usize) -> Option<&wgpu::RenderPipeline> {
        self.sets.get(&(self.format, prepared, self.clip))?.get(i)?.as_ref()
    }
}
