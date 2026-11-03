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
    cache: Option<&'a wgpu::PipelineCache>,
    motion: Option<&'a super::temporal::EffectMotion>,
}

/// Bind groups keyed by the `N` texture views they bind. A few entries cover ping-pong targets and resizes.
pub(super) struct Groups<const N: usize> {
    entries: Vec<([wgpu::TextureView; N], wgpu::BindGroup)>,
}
impl<const N: usize> Default for Groups<N> {
    fn default() -> Self {
        Self { entries: Vec::new() }
    }
}
impl<const N: usize> Groups<N> {
    const CAPACITY: usize = 4;
    pub fn get(&mut self, key: [&wgpu::TextureView; N], make: impl FnOnce() -> wgpu::BindGroup) -> &wgpu::BindGroup {
        let found = self.entries.iter().position(|(views, _)| views.iter().zip(key).all(|(a, b)| a == b));
        let index = match found {
            Some(index) => index,
            None => {
                if self.entries.len() == Self::CAPACITY {
                    self.entries.remove(0);
                }
                self.entries.push((key.map(Clone::clone), make()));
                self.entries.len() - 1
            }
        };
        &self.entries[index].1
    }
}

pub(super) struct RayShadows {
    buffer: wgpu::Buffer,
    layout: wgpu::BindGroupLayout,
    /// The RT scene layout of the trace pipelines, and its bind group for one `ray_reflections` generation.
    scene_layout: wgpu::BindGroupLayout,
    scene: Option<(u64, wgpu::Buffer, wgpu::BindGroup)>,
    pipeline: wgpu::RenderPipeline,
    float_pipeline: wgpu::RenderPipeline,
    raw: Option<wgpu::TextureView>,
    /// Per ray cell receiver planes for the filter, beside `raw`.
    surface: Option<wgpu::TextureView>,
    filter: filter::Filter,
    spatial: Option<wgpu::TextureView>,
    history: Option<history::History>,
    mask: wgpu::TextureView,
    size: (u32, u32),
    /// Two-ray temporal frames alternate between the even and odd strata of a four-ray pattern.
    phase: bool,
    /// Inputs keyed by the depth and normal views, which only change with the surface targets.
    groups: BindCache<wgpu::TextureView, wgpu::BindGroup>,
}

impl RayShadows {
    fn new(device: &wgpu::Device, size: (u32, u32), cache: Option<&wgpu::PipelineCache>) -> Self {
        let texture = |binding, sample_type| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
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
                texture(1, wgpu::TextureSampleType::Depth),
                texture(2, wgpu::TextureSampleType::Float { filterable: false }),
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
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sun RT scene"),
            entries: &super::ray_reflections::layout_entries(),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("RT sun shadow visibility"),
            bind_group_layouts: &[Some(&scene_layout), Some(&layout)],
            immediate_size: 0,
        });
        let colour = |format| Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL });
        let make = |entry_point, targets: &[Option<wgpu::ColorTargetState>]| {
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
                    entry_point: Some(entry_point),
                    compilation_options: Default::default(),
                    targets,
                }),
                multiview_mask: None,
                cache,
            })
        };
        Self {
            buffer,
            layout,
            scene_layout,
            scene: None,
            pipeline: make("fs_main", &[colour(wgpu::TextureFormat::Rg8Unorm)]),
            float_pipeline: make(
                "fs_filtered",
                &[colour(wgpu::TextureFormat::Rgba16Float), colour(wgpu::TextureFormat::Rgba32Uint)],
            ),
            raw: None,
            surface: None,
            filter: filter::Filter::new(device, cache),
            spatial: None,
            history: None,
            mask: mask(device, size),
            size,
            phase: false,
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
        let SunView { options, view_proj, rays, cache, motion } = view;
        let Projection::PerspectiveInfiniteReverse { near, fov_y, .. } = frame.projection else { return };
        let size = self.size;
        let filtered = options.ray_soft_shadows && options.sun_angle > 0.0 && options.ray_shadow_filter > 0.0;
        let temporal = filtered && options.ray_shadow_temporal;
        let ray_size = match filtered && options.ray_shadow_resolution == 50 {
            true => (size.0.div_ceil(2), size.1.div_ceil(2)),
            false => size,
        };
        // Two rays alone leave visible noise. With history on, alternate frames trace the even and odd strata
        // of a four-ray pattern, so the two-frame resolve averages four stratified rays. History still rejects
        // large differences, motion, cuts and sun changes; 4/8 rays and every other mode keep one fixed pattern.
        let interleave = temporal && options.ray_shadow_samples == 2;
        self.phase = interleave && !self.phase;
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
                match interleave {
                    true => 1.0 + f32::from(u8::from(self.phase)),
                    false => 0.0,
                },
            ],
        };
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&params));
        if filtered
            && self.raw.as_ref().is_none_or(|v| v.texture().width() != ray_size.0 || v.texture().height() != ray_size.1)
        {
            self.raw = Some(filter::raw(device, ray_size));
            self.surface = Some(filter::surface(device, ray_size));
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
        let attachment = |view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })
        };
        let attachments = match (&self.raw, &self.surface) {
            (Some(raw), Some(surface)) if filtered => vec![attachment(raw), attachment(surface)],
            _ => vec![attachment(&self.mask)],
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RT sun shadow ray march"),
            color_attachments: &attachments,
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
        if !temporal && let Some(history) = &mut self.history {
            history.invalidate();
        }
        if !filtered {
            return;
        }
        if temporal && self.spatial.is_none() {
            self.spatial = Some(filter::raw(device, size));
            self.history = Some(history::History::with_cache(device, size, cache));
        }
        let output = match temporal {
            true => self.spatial.as_ref().unwrap(),
            false => &self.mask,
        };
        let inputs = filter::Inputs {
            params: &self.buffer,
            raw: self.raw.as_ref().unwrap(),
            surface: self.surface.as_ref().unwrap(),
            output,
            targets: surfaces,
        };
        self.filter.draw(device, encoder, inputs);
        if !temporal {
            return;
        }
        self.history.as_mut().unwrap().resolve_ranged(
            device,
            queue,
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
            pass.surface = None;
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
        let globals = &self.shared.bindings.globals;
        // The scene group only changes with the RT scene's resources, not with its per-frame TLAS rebuild.
        if pass.scene.as_ref().is_none_or(|(generation, bound, _)| *generation != rt.generation || bound != globals) {
            let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sun RT scene"),
                layout: &pass.scene_layout,
                entries: &rt.bindings.entries(globals),
            });
            pass.scene = Some((rt.generation, globals.clone(), group));
        }
        let rays = pass.scene.as_ref().map(|(_, _, group)| group.clone()).unwrap();
        if let Some(surfaces) = &self.surfaces {
            let view = SunView {
                options: &self.experiments,
                view_proj,
                rays: &rays,
                cache: self.pipeline_cache.as_ref(),
                motion: self.effect_motion.as_ref(),
            };
            pass.visibility(&self.device, &self.queue, encoder, frame, view, &surfaces.targets);
        }
        self.ray_shadows = Some(pass);
    }
}
