//! Spatial contact visibility from a dedicated depth/normal prepass; consumed by direct lighting.
use super::surface::targets::Targets;

use super::Renderer;
use blackbox_gfx::{FrameParams, Projection};
use blackbox_gpu_passes::BindCache;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    inverse: [f32; 16],
    view_proj: [f32; 16],
    sun: [f32; 4],
    screen: [f32; 4],
}

/// What one ray march needs of the camera and the sun: the matrix that takes the world to the clip space the depth was
/// drawn in (jittered, when a temporal upscaler runs) and how far the march reaches.
#[derive(Clone, Copy)]
struct March {
    view_proj: glam::Mat4,
    length: f32,
}

pub(super) struct Contact {
    buffer: wgpu::Buffer,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    mask: wgpu::TextureView,
    size: (u32, u32),
    /// Inputs keyed by the depth and normal views, which only change with the surface targets.
    groups: BindCache<wgpu::TextureView, wgpu::BindGroup>,
}

impl Contact {
    fn new(device: &wgpu::Device, size: (u32, u32), cache: Option<&wgpu::PipelineCache>) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("contact inputs"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("contact parameters"),
            size: size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("contact visibility"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/contact.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("contact visibility"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("contact visibility"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::RED,
                })],
            }),
            multiview_mask: None,
            cache,
        });
        Self {
            buffer,
            layout,
            pipeline,
            mask: super::surface::mask(device, size, "contact visibility"),
            size,
            groups: BindCache::default(),
        }
    }

    pub(super) fn size(&self) -> (u32, u32) {
        self.size
    }

    fn visibility(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &FrameParams,
        march: March,
        surfaces: &Targets,
    ) {
        let March { view_proj, length } = march;
        let Projection::PerspectiveInfiniteReverse { near, fov_y, .. } = frame.projection else { return };
        let size = self.size;
        let params = Params {
            inverse: view_proj.inverse().to_cols_array(),
            view_proj: view_proj.to_cols_array(),
            sun: (-frame.light_dir).normalize_or_zero().extend(length).to_array(),
            screen: [size.0 as f32, size.1 as f32, near, 2.0 * (fov_y * 0.5).tan() / size.1 as f32],
        };
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&params));
        let (layout, buffer) = (&self.layout, &self.buffer);
        let (depth, normals) = (&surfaces.depth, &surfaces.normals);
        let group = self.groups.get_or_insert_with(depth, normals, || {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("contact depth and normals"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(depth) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(normals) },
                ],
            })
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("contact ray march"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.mask,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl Renderer {
    pub(super) fn encode_contact(&mut self, encoder: &mut wgpu::CommandEncoder, size: (u32, u32), frame: &FrameParams) {
        if !self.experiments.contact_shadows || !self.screen_lighting_active(frame) {
            return;
        }
        let view_proj = self.world_to_clip(frame);
        let mut contact =
            self.contact.take().unwrap_or_else(|| Contact::new(&self.device, size, self.pipeline_cache.as_ref()));
        if contact.size() != size {
            contact.mask = super::surface::mask(&self.device, size, "contact visibility");
            contact.size = size;
        }
        if self.shared.shadows.contact != contact.mask {
            self.shared.shadows.set_contact(&self.device, &self.shared.glossy.material, contact.mask.clone());
            if let Some(glossy) = &mut self.glossy {
                glossy.rebind_shadows(&self.device, &self.shared.shadows);
            }
        }
        if let Some(surfaces) = &self.surfaces {
            contact.visibility(
                &self.device,
                &self.queue,
                encoder,
                frame,
                March { view_proj, length: self.experiments.contact_length },
                &surfaces.targets,
            );
        }
        self.contact = Some(contact);
    }
}
