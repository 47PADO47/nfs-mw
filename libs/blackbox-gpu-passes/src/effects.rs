//! The world effect layer: reused vertex buffers and pipelines for surface overlays, particles, additive
//! streaks and glows, plus the textured effects and the depth-aware (soft) particles.
//!
//! Call order for one frame, inside the scene's own render pass (colour plus reverse-Z depth, group 0 set
//! to [`WorldBindings::globals_bind_group`] with a current [`Globals`](crate::Globals) block):
//! 1. [`Effects::upload`] with the frame's [`EffectLayer`] (before the pass; it only writes buffers);
//! 2. [`Effects::draw`] and [`Effects::draw_textured`] inside the pass;
//! 3. after the pass has ended, [`Effects::draw_soft`], which records its own pass over the finished
//!    colour image and samples the finished depth.
//!
//! The pipelines are built per target colour format; call [`Effects::use_format`] before drawing into a
//! new one.

use std::collections::HashMap;

use blackbox_gfx::{DEFAULT_SOFT_DISTANCE, EffectLayer, EffectVertex, TextureHandle};

use crate::batch::Batch;
use crate::soft_particles::SoftParticles;
use crate::textured_effects::TexturedEffects;
use crate::world::{DEPTH_FORMAT, EFFECT_VERTEX_ATTRIBUTES, WorldBindings, write_mask};

/// What [`Effects::draw_soft`] draws into and reads.
pub struct SoftDraw<'a> {
    /// The finished colour image, loaded and written back.
    pub target: &'a wgpu::TextureView,
    /// The finished reverse-Z depth image, sampled.
    pub depth: &'a wgpu::TextureView,
    /// Group 0 of the world passes ([`WorldBindings::globals_bind_group`], or the caller's own).
    pub globals: &'a wgpu::BindGroup,
    /// The inverse of the frame's view-projection, to rebuild world positions from depth.
    pub inverse_view_proj: glam::Mat4,
}

pub struct Effects {
    batches: [Batch; 4],
    /// The plain and sRGB-target shader modules, built once and shared by every format's pipelines.
    shaders: HashMap<bool, wgpu::ShaderModule>,
    layout: wgpu::PipelineLayout,
    /// Per target format, built the first time the effects are drawn into it.
    pipelines: HashMap<wgpu::TextureFormat, [wgpu::RenderPipeline; 4]>,
    format: wgpu::TextureFormat,
    soft: SoftParticles,
    detailed: bool,
    soft_distance: f32,
    textured: TexturedEffects,
}

/// The shader module for `srgb`, compiled from [`crate::EFFECTS_WGSL`] or [`crate::EFFECTS_WGSL_SRGB`]
/// the first time that condition is needed.
fn shader_module<'a>(
    device: &wgpu::Device,
    shaders: &'a mut HashMap<bool, wgpu::ShaderModule>,
    srgb: bool,
) -> &'a wgpu::ShaderModule {
    shaders.entry(srgb).or_insert_with(|| {
        let source = match srgb {
            true => crate::EFFECTS_WGSL_SRGB,
            false => crate::EFFECTS_WGSL,
        };
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("world effects"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        })
    })
}

impl Effects {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, bindings: &WorldBindings) -> Self {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("world effects"),
            bind_group_layouts: &[Some(&bindings.globals_layout)],
            immediate_size: 0,
        });
        let mut effects = Self {
            batches: std::array::from_fn(|_| Batch::new(device)),
            shaders: HashMap::new(),
            layout,
            pipelines: HashMap::new(),
            format,
            soft: SoftParticles::new(device, format, bindings),
            detailed: false,
            soft_distance: DEFAULT_SOFT_DISTANCE,
            textured: TexturedEffects::new(device, format, bindings),
        };
        effects.use_format(device, format);
        effects
    }

    /// Draw into `format` from now on, building its pipelines the first time.
    pub fn use_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        self.format = format;
        self.soft.use_format(device, format);
        self.textured.use_format(device, format);
        if !self.pipelines.contains_key(&format) {
            let pipelines = self.build(device, format);
            self.pipelines.insert(format, pipelines);
        }
    }

    fn build(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) -> [wgpu::RenderPipeline; 4] {
        let srgb = crate::world::is_srgb(format);
        let shader = shader_module(device, &mut self.shaders, srgb);
        const ENTRIES: [&str; 4] = ["fs_surface", "fs_particle", "fs_streak", "fs_glow"];
        std::array::from_fn(|i| {
            let entry = ENTRIES[i];
            let bias = match i {
                0 => wgpu::DepthBiasState { constant: 2, slope_scale: 1.0, clamp: 0.0 },
                _ => wgpu::DepthBiasState::default(),
            };
            let blend = match i {
                2 | 3 => wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent::OVER,
                },
                _ => wgpu::BlendState::ALPHA_BLENDING,
            };
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&self.layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<EffectVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &EFFECT_VERTEX_ATTRIBUTES,
                    })],
                },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                    stencil: Default::default(),
                    bias,
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: write_mask(format),
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        })
    }

    /// Replace the layer drawn by every following [`Self::draw`], until replaced again.
    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, layer: &EffectLayer) {
        self.detailed = layer.detailed_particles;
        self.soft_distance = layer.soft_distance;
        self.textured.upload(device, queue, &layer.textured);
        for (batch, vertices) in
            self.batches.iter_mut().zip([&layer.surfaces, &layer.particles, &layer.streaks, &layer.glows])
        {
            batch.upload(device, queue, vertices);
        }
    }

    /// Draw the surface overlays, particles, streaks and glows into a pass that has the world's colour and
    /// depth attached and the globals bound to group 0. Detailed particles are left to [`Self::draw_soft`].
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        let Some(pipelines) = self.pipelines.get(&self.format) else { return };
        for (i, (batch, pipeline)) in self.batches.iter().zip(pipelines).enumerate() {
            if batch.count() == 0 || (i == 1 && self.detailed) {
                continue;
            }
            pass.set_pipeline(pipeline);
            pass.set_vertex_buffer(0, batch.buffer().slice(..));
            pass.draw(0..batch.count(), 0..1);
        }
    }

    /// Draw the textured effects into the same pass as [`Self::draw`]. `texture` resolves a handle to a
    /// bind group made with [`WorldBindings::texture_layout`]; unresolved handles are skipped.
    pub fn draw_textured<'t>(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        texture: impl Fn(TextureHandle) -> Option<&'t wgpu::BindGroup>,
    ) {
        self.textured.draw(pass, texture);
    }

    /// Draw the detailed particles over the finished colour image, fading them where they meet the finished
    /// depth. Records its own render pass; does nothing when the layer has no detailed particles. Call it
    /// after the pass of [`Self::draw`] has ended.
    pub fn draw_soft(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        draw: &SoftDraw<'_>,
    ) {
        if !self.detailed || self.batches[1].count() == 0 {
            return;
        }
        self.soft.prepare(device, queue, draw.depth, draw.inverse_view_proj, self.soft_distance);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("soft particles"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: draw.target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        self.soft.bind(&mut pass, draw.globals);
        let batch = &self.batches[1];
        pass.set_vertex_buffer(0, batch.buffer().slice(..));
        pass.draw(0..batch.count(), 0..1);
    }

    /// Allocated surface-overlay capacity in vertices.
    pub fn surface_capacity(&self) -> usize {
        self.batches[0].capacity()
    }

    /// Allocated particle capacity in vertices.
    pub fn particle_capacity(&self) -> usize {
        self.batches[1].capacity()
    }

    /// Allocated additive-streak capacity in vertices; retained when the streak layer is cleared.
    pub fn streak_capacity(&self) -> usize {
        self.batches[2].capacity()
    }
}
