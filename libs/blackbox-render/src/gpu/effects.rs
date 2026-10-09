//! Reused dynamic buffers and pipelines for world-space effects.

use std::collections::HashMap;

use super::Renderer;
use super::resources::{DEPTH_FORMAT, Shared};
use super::soft_particles::SoftParticles;
use super::targets::write_mask;
use crate::{DEFAULT_SOFT_DISTANCE, EffectLayer, EffectVertex};

pub(super) const ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Unorm8x4, 2 => Float32x2, 3 => Float32x2];

pub(super) struct Batch {
    pub(super) buffer: wgpu::Buffer,
    capacity: usize,
    pub(super) count: u32,
}

impl Batch {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self { buffer: Self::allocate(device, 1), capacity: 1, count: 0 }
    }

    fn allocate(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dynamic effects"),
            size: (capacity * std::mem::size_of::<EffectVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub(super) fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &[EffectVertex]) {
        self.count = vertices.len() as u32;
        if vertices.len() > self.capacity {
            self.capacity = vertices.len().next_power_of_two();
            self.buffer = Self::allocate(device, self.capacity);
        }
        if !vertices.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(vertices));
        }
    }
}

pub(super) struct Effects {
    batches: [Batch; 4],
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    /// Per target format, built the first time the effects are drawn into it.
    pipelines: HashMap<wgpu::TextureFormat, [wgpu::RenderPipeline; 4]>,
    format: wgpu::TextureFormat,
    soft: SoftParticles,
    detailed: bool,
    soft_distance: f32,
    pub(super) textured: super::textured_effects::TexturedEffects,
}

impl Effects {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat, shared: &Shared) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("world effects"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/effects.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("world effects"),
            bind_group_layouts: &[Some(&shared.globals_layout)],
            immediate_size: 0,
        });
        let mut effects = Self {
            batches: std::array::from_fn(|_| Batch::new(device)),
            shader,
            layout,
            pipelines: HashMap::new(),
            format,
            soft: SoftParticles::new(device, format, shared),
            detailed: false,
            soft_distance: DEFAULT_SOFT_DISTANCE,
            textured: super::textured_effects::TexturedEffects::new(device, format, shared),
        };
        effects.use_format(device, format);
        effects
    }

    /// Draw into `format` from now on, building its pipelines the first time.
    pub(super) fn use_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        self.format = format;
        self.soft.use_format(device, format);
        self.textured.use_format(device, format);
        if !self.pipelines.contains_key(&format) {
            let pipelines = self.build(device, format);
            self.pipelines.insert(format, pipelines);
        }
    }

    fn build(&self, device: &wgpu::Device, format: wgpu::TextureFormat) -> [wgpu::RenderPipeline; 4] {
        std::array::from_fn(|i| {
            let entry = ["fs_surface", "fs_particle", "fs_streak", "fs_glow"][i];
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
                    module: &self.shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<EffectVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &ATTRIBUTES,
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
                    module: &self.shader,
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

    pub(super) fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, layer: &EffectLayer) {
        self.detailed = layer.detailed_particles;
        self.soft_distance = layer.soft_distance;
        self.textured.upload(device, queue, &layer.textured);
        for (batch, vertices) in
            self.batches.iter_mut().zip([&layer.surfaces, &layer.particles, &layer.streaks, &layer.glows])
        {
            batch.upload(device, queue, vertices);
        }
    }

    pub(super) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        let Some(pipelines) = self.pipelines.get(&self.format) else { return };
        for (i, (batch, pipeline)) in self.batches.iter().zip(pipelines).enumerate() {
            if batch.count == 0 || (i == 1 && self.detailed) {
                continue;
            }
            pass.set_pipeline(pipeline);
            pass.set_vertex_buffer(0, batch.buffer.slice(..));
            pass.draw(0..batch.count, 0..1);
        }
    }

    pub(super) fn draw_soft(
        &mut self,
        gpu: (&wgpu::Device, &wgpu::Queue),
        encoder: &mut wgpu::CommandEncoder,
        views: (&wgpu::TextureView, &wgpu::TextureView),
        shared: &Shared,
        frame: &crate::FrameParams,
    ) {
        if !self.detailed || self.batches[1].count == 0 {
            return;
        }
        let (device, queue) = gpu;
        let (target, depth) = views;
        self.soft.prepare(device, queue, depth, frame.view_proj().inverse(), self.soft_distance);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("soft particles"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        self.soft.bind(&mut pass, shared);
        let batch = &self.batches[1];
        pass.set_vertex_buffer(0, batch.buffer.slice(..));
        pass.draw(0..batch.count, 0..1);
    }
}

impl Renderer {
    /// Replace the world effects drawn in every following scene/capture, until replaced again.
    pub fn set_effects(&mut self, layer: &EffectLayer) {
        self.effects.upload(&self.device, &self.queue, layer);
    }

    /// Allocated capacities in vertices (surface, particle); counts may fall to zero while reused.
    pub fn effect_capacities(&self) -> [usize; 2] {
        [self.effects.batches[0].capacity, self.effects.batches[1].capacity]
    }

    /// Allocated additive-streak capacity in vertices; retained when the streak layer is cleared.
    pub fn streak_capacity(&self) -> usize {
        self.effects.batches[2].capacity
    }
}
