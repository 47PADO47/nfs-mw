//! Screen-space sun shadows after Bend Studio's method (`screen_shadow.wgsl`, `wavefront.rs`): 60 depth samples
//! towards the sun per pixel, written to the mask the short contact march otherwise fills.
use super::wavefront::{self, WAVE};
use crate::gpu::surface::targets::Targets;
use glam::Mat4;

/// How much of the direct light a screen-space shadow removes.
const STRENGTH: f32 = 0.8;
/// The most dispatches a light needs.
const DISPATCHES: usize = 8;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    light: [f32; 4],
    sun: [f32; 4],
    wave_offset: [i32; 2],
    size: [i32; 2],
}

pub(super) struct ScreenShadows {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    /// One parameter block per dispatch.
    buffers: [wgpu::Buffer; DISPATCHES],
    pub mask: wgpu::TextureView,
    size: (u32, u32),
    /// The bind groups of the dispatches, for the depth and normal views they were made with.
    groups: Option<(wgpu::TextureView, wgpu::TextureView, [wgpu::BindGroup; DISPATCHES])>,
}

impl ScreenShadows {
    /// Whether `device` can run the pass: 64-wide workgroups and a storage texture in a compute stage.
    pub fn supported(device: &wgpu::Device) -> bool {
        let limits = device.limits();
        limits.max_compute_workgroup_size_x >= WAVE as u32
            && limits.max_compute_invocations_per_workgroup >= WAVE as u32
            && limits.max_storage_textures_per_shader_stage >= 1
            && limits.max_compute_workgroups_per_dimension >= 4096
    }

    pub fn new(device: &wgpu::Device, size: (u32, u32), cache: Option<&wgpu::PipelineCache>) -> Self {
        let texture = |binding, sample_type| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("screen shadow inputs"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                texture(1, wgpu::TextureSampleType::Depth),
                texture(2, wgpu::TextureSampleType::Float { filterable: false }),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("screen shadows"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/screen_shadow.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("screen shadows"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("screen shadows"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("shadow"),
            compilation_options: Default::default(),
            cache,
        });
        let buffers = std::array::from_fn(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("screen shadow dispatch"),
                size: size_of::<Params>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        Self { layout, pipeline, buffers, mask: mask(device, size), size, groups: None }
    }

    /// Writes the sun's screen-space shadow of `surfaces` into the mask. `view_proj` is the matrix the depth was
    /// drawn with and `to_sun` the unit direction to the sun.
    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        (view_proj, to_sun): (Mat4, glam::Vec3),
        surfaces: &Targets,
        size: (u32, u32),
    ) {
        if self.size != size {
            self.mask = mask(device, size);
            self.size = size;
            self.groups = None;
        }
        let stale = self
            .groups
            .as_ref()
            .is_none_or(|(depth, normals, _)| *depth != surfaces.depth || *normals != surfaces.normals);
        if stale {
            let groups = std::array::from_fn(|index| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("screen shadow dispatch"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: self.buffers[index].as_entire_binding() },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&surfaces.depth),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&surfaces.normals),
                        },
                        wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&self.mask) },
                    ],
                })
            });
            self.groups = Some((surfaces.depth.clone(), surfaces.normals.clone(), groups));
        }
        // Pixels no wavefront reaches stay lit.
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("screen shadow clear"),
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
        }));
        let viewport = [size.0 as i32, size.1 as i32];
        let list = wavefront::dispatch_list((view_proj * to_sun.extend(0.0)).to_array(), viewport, [0, 0], viewport);
        let groups = &self.groups.as_ref().expect("the bind groups were just made").2;
        let mut pass = encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("screen shadows"), timestamp_writes: None });
        pass.set_pipeline(&self.pipeline);
        for (index, dispatch) in list.dispatches.iter().enumerate().take(DISPATCHES) {
            let params = Params {
                light: list.light,
                sun: to_sun.extend(STRENGTH).to_array(),
                wave_offset: dispatch.offset,
                size: viewport,
            };
            queue.write_buffer(&self.buffers[index], 0, bytemuck::bytes_of(&params));
            pass.set_bind_group(0, &groups[index], &[]);
            pass.dispatch_workgroups(dispatch.waves[0], dispatch.waves[1], dispatch.waves[2]);
        }
    }
}

fn mask(device: &wgpu::Device, size: (u32, u32)) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("screen shadow visibility"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
