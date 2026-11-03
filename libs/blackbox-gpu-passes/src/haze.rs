//! Small, bounded local-light scattering volumes against the finished scene depth.
use std::collections::HashMap;

use blackbox_gfx::LightHaze;
use wgpu::util::DeviceExt;

use crate::{SoftDraw, WorldBindings, world::write_mask};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    to_world: [[f32; 4]; 4],
    from_world: [[f32; 4]; 4],
    color: [f32; 4],
    handedness: [f32; 4],
}

pub struct Haze {
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    depth_layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
    capacity: usize,
    count: u32,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    format: wgpu::TextureFormat,
    cache: Option<wgpu::PipelineCache>,
    /// Uniforms and bind groups of recently drawn views, the most recent last.
    views: Vec<DepthView>,
}

impl Haze {
    pub fn new(device: &wgpu::Device, bindings: &WorldBindings, cache: Option<&wgpu::PipelineCache>) -> Self {
        let depth_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("light haze depth"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("light haze"),
            bind_group_layouts: &[Some(&bindings.globals_layout), Some(&depth_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("light haze"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/haze.wgsl").into()),
        });
        Self {
            shader,
            layout,
            depth_layout,
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("light haze instances"),
                contents: bytemuck::bytes_of(&<Instance as bytemuck::Zeroable>::zeroed()),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }),
            capacity: 1,
            count: 0,
            pipelines: HashMap::new(),
            format: crate::HDR_FORMAT,
            cache: cache.cloned(),
            views: Vec::new(),
        }
    }

    pub fn use_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        self.format = format;
        if self.pipelines.contains_key(&format) {
            return;
        }
        let attributes = wgpu::vertex_attr_array![
            0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4,
            4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4,
            8 => Float32x4, 9 => Float32x4
        ];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("light haze"),
            layout: Some(&self.layout),
            vertex: wgpu::VertexState {
                module: &self.shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Instance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &attributes,
                })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &self.shader,
                entry_point: Some(match crate::world::is_srgb(format) {
                    true => "fs_main_srgb",
                    false => "fs_main",
                }),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: write_mask(format),
                })],
            }),
            multiview_mask: None,
            cache: self.cache.as_ref(),
        });
        self.pipelines.insert(format, pipeline);
    }

    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, volumes: &[LightHaze]) {
        let mut instances = Vec::new();
        for volume in volumes.iter().take(8) {
            let det = volume.to_world.determinant();
            if !volume.to_world.is_finite()
                || det.abs() < 1e-8
                || !volume.intensity.is_finite()
                || volume.intensity <= 0.0
                || volume.color.iter().any(|v| !v.is_finite())
            {
                continue;
            }
            instances.push(Instance {
                to_world: volume.to_world.to_cols_array_2d(),
                from_world: volume.to_world.inverse().to_cols_array_2d(),
                color: [
                    volume.color[0].max(0.0),
                    volume.color[1].max(0.0),
                    volume.color[2].max(0.0),
                    volume.intensity.clamp(0.0, 4.0),
                ],
                handedness: [det.signum(), 0.0, 0.0, 0.0],
            });
        }
        self.count = instances.len() as u32;
        if instances.is_empty() {
            return;
        }
        if instances.len() > self.capacity {
            self.capacity = instances.len().next_power_of_two();
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("light haze instances"),
                size: (self.capacity * std::mem::size_of::<Instance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&instances));
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        draw: &SoftDraw<'_>,
    ) {
        if self.count == 0 {
            return;
        }
        let index = self.view(device, draw.depth);
        let view = &self.views[index];
        queue.write_buffer(&view.inverse, 0, bytemuck::cast_slice(&draw.inverse_view_proj.to_cols_array()));
        let depth = &view.group;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("light haze"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: draw.target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipelines[&self.format]);
        pass.set_bind_group(0, draw.globals, &[]);
        pass.set_bind_group(1, depth, &[]);
        pass.set_vertex_buffer(0, self.buffer.slice(..));
        pass.draw(0..36, 0..self.count);
    }

    /// The uniform and bind group of the view whose depth is `depth`. Each view (each depth image) has its own
    /// uniform, so views recorded before one submission never overwrite each other's matrix.
    fn view(&mut self, device: &wgpu::Device, depth: &wgpu::TextureView) -> usize {
        if let Some(index) = self.views.iter().position(|view| view.depth == *depth) {
            let view = self.views.remove(index);
            self.views.push(view);
            return self.views.len() - 1;
        }
        let inverse = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("light haze view inverse"),
            size: std::mem::size_of::<[f32; 16]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("light haze depth"),
            layout: &self.depth_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(depth) },
                wgpu::BindGroupEntry { binding: 1, resource: inverse.as_entire_binding() },
            ],
        });
        if self.views.len() >= VIEWS {
            // The least recently drawn view goes, releasing a depth image a resize replaced.
            self.views.remove(0);
        }
        self.views.push(DepthView { depth: depth.clone(), inverse, group });
        self.views.len() - 1
    }
}

/// Views (depth images) whose haze bind groups are kept.
const VIEWS: usize = 4;

struct DepthView {
    depth: wgpu::TextureView,
    inverse: wgpu::Buffer,
    group: wgpu::BindGroup,
}
