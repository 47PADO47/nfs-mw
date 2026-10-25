//! Deterministic sun-ray mask: red visibility, green cascade replacement weight.
#[cfg(test)]
mod budget_tests;
mod filter;
pub(super) mod history;
#[cfg(test)]
mod reconstruction_tests;
#[cfg(test)]
mod soft_tests;
#[cfg(test)]
mod tests;
use super::surface::targets::Targets;

use super::Renderer;
use blackbox_gfx::{FrameParams, Projection};
use blackbox_gpu_passes::BindCache;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    inverse: [f32; 16],
    view_proj: [f32; 16],
    camera: [f32; 4],
    sun: [f32; 4],
    screen: [f32; 4],
    tuning: [f32; 4],
    soft: [f32; 4],
}

struct SunView<'a> {
    options: &'a blackbox_gfx::RenderExperiments,
    view_proj: glam::Mat4,
    rays: &'a wgpu::BindGroup,
    motion: Option<&'a super::temporal::EffectMotion>,
}

pub(super) struct RayShadows {
    buffer: wgpu::Buffer,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    float_pipeline: wgpu::RenderPipeline,
    raw: Option<wgpu::TextureView>,
    filter: Option<filter::Filter>,
    spatial: Option<wgpu::TextureView>,
    history: Option<history::History>,
    mask: wgpu::TextureView,
    size: (u32, u32),
    /// Inputs keyed by the depth and normal views, which only change with the surface targets.
    groups: BindCache<wgpu::TextureView, wgpu::BindGroup>,
}

impl RayShadows {
    fn new(device: &wgpu::Device, size: (u32, u32), cache: Option<&wgpu::PipelineCache>) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("RT sun shadow inputs"),
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
            label: Some("RT sun shadow parameters"),
            size: size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RT sun shadow visibility"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/ray_shadow.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("RT sun shadow visibility"),
            bind_group_layouts: &[
                Some(&device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("sun RT scene"),
                    entries: &super::ray_reflections::layout_entries(),
                })),
                Some(&layout),
            ],
            immediate_size: 0,
        });
        let make = |format| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("RT sun shadow visibility"),
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
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache,
            })
        };
        Self {
            buffer,
            layout,
            pipeline: make(wgpu::TextureFormat::Rg8Unorm),
            float_pipeline: make(wgpu::TextureFormat::Rgba16Float),
            raw: None,
            filter: None,
            spatial: None,
            history: None,
            mask: mask(device, size),
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
        view: SunView<'_>,
        surfaces: &Targets,
    ) {
        let SunView { options, view_proj, rays, motion } = view;
        let Projection::PerspectiveInfiniteReverse { near, fov_y, .. } = frame.projection else { return };
        let size = self.size;
        let filtered = options.ray_soft_shadows && options.sun_angle > 0.0 && options.ray_shadow_filter > 0.0;
        let ray_size = match filtered && options.ray_shadow_resolution == 50 {
            true => (size.0.div_ceil(2), size.1.div_ceil(2)),
            false => size,
        };
        let params = Params {
            inverse: view_proj.inverse().to_cols_array(),
            view_proj: view_proj.to_cols_array(),
            camera: frame.camera_position.extend(options.ray_shadow_range).to_array(),
            sun: (-frame.light_dir).normalize_or_zero().extend(options.ray_shadow_length).to_array(),
            screen: [size.0 as f32, size.1 as f32, near, 2.0 * (fov_y * 0.5).tan() / size.1 as f32],
            tuning: [
                options.ray_shadow_bias,
                u8::from(options.original_world_materials) as f32,
                ray_size.0 as f32,
                ray_size.1 as f32,
            ],
            soft: [
                match options.ray_soft_shadows {
                    true => options.sun_angle.to_radians().tan(),
                    false => 0.0,
                },
                options.ray_shadow_samples as f32,
                options.ray_shadow_filter,
                0.0,
            ],
        };
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&params));
        if filtered
            && self.raw.as_ref().is_none_or(|v| v.texture().width() != ray_size.0 || v.texture().height() != ray_size.1)
        {
            self.raw = Some(filter::raw(device, ray_size));
            self.filter = Some(filter::Filter::new(device));
        }
        let (layout, buffer) = (&self.layout, &self.buffer);
        let (depth, normals) = (&surfaces.depth, &surfaces.normals);
        let group = self.groups.get_or_insert_with(depth, normals, || {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("RT sun shadow depth and normals"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(depth) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(normals) },
                ],
            })
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RT sun shadow ray march"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: match filtered {
                    true => self.raw.as_ref().unwrap(),
                    false => &self.mask,
                },
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(match filtered {
            true => &self.float_pipeline,
            false => &self.pipeline,
        });
        pass.set_bind_group(0, rays, &[]);
        pass.set_bind_group(1, group, &[]);
        pass.draw(0..3, 0..1);
        drop(pass);
        if filtered && options.ray_shadow_temporal && self.spatial.is_none() {
            self.spatial = Some(filter::raw(device, size));
            self.history = Some(history::History::new(device, size));
        }
        if filtered {
            self.filter.as_ref().unwrap().draw(
                device,
                encoder,
                &self.buffer,
                self.raw.as_ref().unwrap(),
                match options.ray_shadow_temporal {
                    true => self.spatial.as_ref().unwrap(),
                    false => &self.mask,
                },
                surfaces,
            );
        }
        if filtered && options.ray_shadow_temporal {
            self.history.as_mut().unwrap().resolve(
                device,
                encoder,
                history::Draw {
                    source: self.spatial.as_ref().unwrap(),
                    output: &self.mask,
                    surfaces,
                    frame,
                    options,
                    view_proj,
                    motion,
                },
            );
        }
        if !(filtered && options.ray_shadow_temporal)
            && let Some(history) = &mut self.history
        {
            history.invalidate();
        }
    }
}

pub(super) fn mask(device: &wgpu::Device, size: (u32, u32)) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("RT sun visibility and coverage"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
impl Renderer {
    pub(super) fn ray_shadows_active(&self, frame: &FrameParams) -> bool {
        self.ray_reflections.is_some()
            && self.experiments.ray_sun_shadows
            && self.experiments.sun_shadows
            && self.experiments.shadow_projection.is_none()
            && self.screen_lighting_active(frame)
            && frame.light_dir.is_finite()
            && frame.light_dir.length_squared() > 0.5
    }
    pub(super) fn encode_ray_shadows(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        size: (u32, u32),
        frame: &FrameParams,
    ) {
        if !self.ray_shadows_active(frame) {
            if let Some(pass) = &mut self.ray_shadows
                && let Some(history) = &mut pass.history
            {
                history.invalidate();
            }
            return;
        }
        let _scope = blackbox_trace::scope("Vulkan RT: sun visibility");
        let view_proj = self.world_to_clip(frame);
        let mut pass = self
            .ray_shadows
            .take()
            .unwrap_or_else(|| RayShadows::new(&self.device, size, self.pipeline_cache.as_ref()));
        if pass.size != size {
            pass.mask = mask(&self.device, size);
            pass.size = size;
            pass.raw = None;
            pass.spatial = None;
            pass.history = None;
        }
        if self.shared.shadows.contact != pass.mask {
            self.shared.shadows.set_contact(&self.device, &self.shared.glossy.material, pass.mask.clone());
            if let Some(glossy) = &mut self.glossy {
                glossy.rebind_shadows(&self.device, &self.shared.shadows);
            }
        }
        let rt = self.ray_reflections.as_ref().unwrap();
        let layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sun RT scene"),
            entries: &super::ray_reflections::layout_entries(),
        });
        let rays = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sun RT scene"),
            layout: &layout,
            entries: &rt.bindings.entries(&self.shared.bindings.globals),
        });
        if let Some(surfaces) = &self.surfaces {
            pass.visibility(
                &self.device,
                &self.queue,
                encoder,
                frame,
                SunView { options: &self.experiments, view_proj, rays: &rays, motion: self.effect_motion.as_ref() },
                &surfaces.targets,
            );
        }
        self.ray_shadows = Some(pass);
    }
}
