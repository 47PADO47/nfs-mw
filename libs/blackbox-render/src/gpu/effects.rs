//! Reused dynamic buffers and pipelines for world-space effects.

use super::Renderer;
use super::resources::{DEPTH_FORMAT, Shared};
use crate::{EffectLayer, EffectVertex};

const ATTRIBUTES: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Unorm8x4, 2 => Float32x2];

struct Batch {
    buffer: wgpu::Buffer,
    capacity: usize,
    count: u32,
}

impl Batch {
    fn new(device: &wgpu::Device) -> Self {
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

    fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &[EffectVertex]) {
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
    batches: [Batch; 2],
    pipelines: [wgpu::RenderPipeline; 2],
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
        let pipelines = std::array::from_fn(|i| {
            let entry = ["fs_surface", "fs_particle"][i];
            let bias = match i {
                0 => wgpu::DepthBiasState { constant: 2, slope_scale: 1.0, clamp: 0.0 },
                _ => wgpu::DepthBiasState::default(),
            };
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
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
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        });
        Self { batches: std::array::from_fn(|_| Batch::new(device)), pipelines }
    }

    pub(super) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        for (batch, pipeline) in self.batches.iter().zip(&self.pipelines) {
            if batch.count == 0 {
                continue;
            }
            pass.set_pipeline(pipeline);
            pass.set_vertex_buffer(0, batch.buffer.slice(..));
            pass.draw(0..batch.count, 0..1);
        }
    }
}

impl Renderer {
    /// Replace the world effects drawn in every following scene/capture, until replaced again.
    pub fn set_effects(&mut self, layer: &EffectLayer) {
        for (batch, vertices) in self.effects.batches.iter_mut().zip([&layer.surfaces, &layer.particles]) {
            batch.upload(&self.device, &self.queue, vertices);
        }
    }

    /// Allocated capacities in vertices (surface, particle); counts may fall to zero while reused.
    pub fn effect_capacities(&self) -> [usize; 2] {
        self.effects.batches.each_ref().map(|b| b.capacity)
    }
}
