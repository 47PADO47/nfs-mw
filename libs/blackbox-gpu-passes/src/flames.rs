//! Bounded volumetric flames. The WGSL ports mattatz/THREE.Fire (MIT).
use std::collections::HashMap;

use blackbox_gfx::{FlameVolume, TextureHandle};
use wgpu::util::DeviceExt;

use crate::{SoftDraw, WorldBindings, world::write_mask};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    to_world: [[f32; 4]; 4],
    from_world: [[f32; 4]; 4],
    parameters: [f32; 4],
    root: [f32; 4],
    tail: [f32; 4],
    shape: [f32; 4],
    root_color: [f32; 4],
    tail_color: [f32; 4],
    contact_plane: [f32; 4],
}

pub struct Flames {
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    depth_layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
    capacity: usize,
    textures: Vec<TextureHandle>,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    format: wgpu::TextureFormat,
    cache: Option<wgpu::PipelineCache>,
}

impl Flames {
    pub fn new(device: &wgpu::Device, bindings: &WorldBindings, cache: Option<&wgpu::PipelineCache>) -> Self {
        let depth_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("flame scene depth"),
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
            label: Some("volumetric flames"),
            bind_group_layouts: &[Some(&bindings.globals_layout), Some(&bindings.texture_layout), Some(&depth_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("THREE.Fire WGSL port"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(include_str!("shaders/flame_noise.wgsl"), include_str!("shaders/flames.wgsl")).into(),
            ),
        });
        Self {
            shader,
            layout,
            depth_layout,
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("flame volumes"),
                contents: bytemuck::bytes_of(&Instance::zero()),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }),
            capacity: 1,
            textures: Vec::new(),
            pipelines: HashMap::new(),
            format: crate::HDR_FORMAT,
            cache: cache.cloned(),
        }
    }

    pub fn use_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        self.format = format;
        if self.pipelines.contains_key(&format) {
            return;
        }
        let attributes = wgpu::vertex_attr_array![
            0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4,
            4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4,
            9 => Float32x4, 10 => Float32x4, 11 => Float32x4, 12 => Float32x4, 13 => Float32x4,
            14 => Float32x4
        ];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("volumetric flames"),
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
            // The shader selects exit faces, including reflected source marker frames.
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

    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, volumes: &[FlameVolume]) {
        self.textures.clear();
        let mut instances = Vec::with_capacity(volumes.len());
        for v in volumes.iter().take(64) {
            let a = &v.appearance;
            if !v.to_world.is_finite()
                || v.to_world.determinant().abs() < 1e-8
                || !v.time.is_finite()
                || !v.seed.is_finite()
                || !v.intensity.is_finite()
                || v.intensity <= 0.0
                || !a.root_tint.is_finite()
                || !a.tail_tint.is_finite()
                || !a.bend.is_finite()
                || !a.contact_plane.is_finite()
                || !a.density.is_finite()
                || !a.turbulence.is_finite()
                || !a.hot_core.is_finite()
                || !a.radial_scale.is_finite()
                || a.root_color.iter().chain(a.tail_color.iter()).any(|c| !c.is_finite())
            {
                continue;
            }
            self.textures.push(v.texture);
            instances.push(Instance {
                to_world: v.to_world.to_cols_array_2d(),
                from_world: v.to_world.inverse().to_cols_array_2d(),
                parameters: [
                    v.time,
                    v.seed,
                    f32::from(v.blue),
                    v.intensity.clamp(0.0, 16.0) * v.to_world.determinant().signum(),
                ],
                root: a.root_tint.max(glam::Vec3::ZERO).extend(a.density.clamp(0.0, 4.0)).to_array(),
                tail: a.tail_tint.max(glam::Vec3::ZERO).extend(a.turbulence.clamp(0.0, 2.0)).to_array(),
                shape: [a.bend.x, a.bend.z, a.radial_scale.clamp(1.0, 16.0), a.hot_core.clamp(0.0, 1.0)],
                root_color: a.root_color,
                tail_color: a.tail_color,
                contact_plane: a.contact_plane.to_array(),
            });
        }
        if instances.is_empty() {
            return;
        }
        if instances.len() > self.capacity {
            self.capacity = instances.len().next_power_of_two();
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("flame volumes"),
                size: (self.capacity * std::mem::size_of::<Instance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&instances));
    }

    pub fn draw<'t>(
        &mut self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        draw: &SoftDraw<'_>,
        texture: impl Fn(TextureHandle) -> Option<&'t wgpu::BindGroup>,
    ) {
        if self.textures.is_empty() {
            return;
        }
        // Each view owns its inverse: a later mirror must not overwrite it before submission.
        let inverse = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("flame view inverse"),
            contents: bytemuck::cast_slice(&draw.inverse_view_proj.to_cols_array()),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let depth = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("flame scene depth"),
            layout: &self.depth_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(draw.depth) },
                wgpu::BindGroupEntry { binding: 1, resource: inverse.as_entire_binding() },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("volumetric flames"),
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
        pass.set_bind_group(2, &depth, &[]);
        pass.set_vertex_buffer(0, self.buffer.slice(..));
        for (i, handle) in self.textures.iter().enumerate() {
            let Some(binding) = texture(*handle) else { continue };
            pass.set_bind_group(1, binding, &[]);
            pass.draw(0..36, i as u32..i as u32 + 1);
        }
    }
}

impl Instance {
    fn zero() -> Self {
        bytemuck::Zeroable::zeroed()
    }
}
