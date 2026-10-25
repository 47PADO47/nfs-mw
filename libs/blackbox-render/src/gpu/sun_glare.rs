//! Main-view solar lens artwork, occluded by current scene depth and the shared cloud map.
#[cfg(test)]
#[path = "sun_glare_tests.rs"]
mod tests;
use super::Renderer;
use blackbox_gfx::{FrameParams, Projection};
use std::collections::HashMap;
use wgpu::util::DeviceExt;
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    sun: [f32; 4],
    screen: [f32; 4],
    colour: [f32; 4],
    layer: [f32; 4],
    cloud_uv: [f32; 4],
}
pub(super) struct Pass {
    shader: wgpu::ShaderModule,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    sampler: wgpu::Sampler,
    fallback: wgpu::TextureView,
}
impl Pass {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("clear sky glare fallback"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            &[255; 4],
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
            texture.size(),
        );
        Self {
            shader: device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("solar lens artwork"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/sun_glare.wgsl").into()),
            }),
            pipelines: HashMap::new(),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            fallback: texture.create_view(&Default::default()),
        }
    }
    fn pipeline(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) -> &wgpu::RenderPipeline {
        self.pipelines.entry(format).or_insert_with(|| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("solar lens artwork"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::COLOR,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        })
    }
}
impl Renderer {
    pub(super) fn encode_sun_glare(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        frame: &FrameParams,
    ) {
        let Some(state) = self
            .sun_glare_state
            .as_ref()
            .filter(|s| s.strength.is_finite() && s.strength > 0.0 && !s.layers.is_empty())
        else {
            return;
        };
        let Projection::PerspectiveInfiniteReverse { fov_y, .. } = frame.projection else { return };
        let sun = (-frame.light_dir).normalize_or_zero();
        if sun.z <= 0.0 {
            return;
        }
        let clip = self.world_to_clip(frame) * (frame.camera_position + sun * 10000.0).extend(1.0);
        if clip.w <= 0.0 {
            return;
        }
        let ndc = clip.truncate() / clip.w;
        if ndc.x.abs() > 1.3 || ndc.y.abs() > 1.3 {
            return;
        }
        let mut glare = self.sun_glare.take().unwrap_or_else(|| Pass::new(&self.device, &self.queue));
        glare.pipeline(&self.device, target.texture().format());
        let pipeline = &glare.pipelines[&target.texture().format()];
        let cloud = self.glare_clouds();
        let size = (depth.texture().width(), depth.texture().height());
        let radius =
            (size.1 as f32 * state.angular_radius.to_radians().tan() / (2.0 * (fov_y * 0.5).tan())).clamp(1.0, 12.0);
        for layer in state.layers.iter().take(8) {
            if !layer.radius.is_finite() || !layer.brightness.is_finite() || !layer.rotation.is_finite() {
                continue;
            }
            let Some(image) = self.texture_views.get(&layer.texture.raw()) else { continue };
            let params = Params {
                sun: [ndc.x, ndc.y, 0.0, 1.0],
                screen: [size.0 as f32, size.1 as f32, radius, u8::from(cloud.is_some()) as f32],
                colour: [state.colour[0], state.colour[1], state.colour[2], state.strength.clamp(0.0, 8.0)],
                layer: [
                    layer.radius.clamp(0.001, 0.5),
                    layer.brightness.clamp(0.0, 8.0),
                    layer.rotation,
                    state.weather_visibility.clamp(0.0, 1.0),
                ],
                cloud_uv: [
                    sun.y.atan2(sun.x) / std::f32::consts::TAU + 0.5,
                    sun.z.asin() / std::f32::consts::FRAC_PI_2,
                    0.0,
                    0.0,
                ],
            };
            let uniform = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("solar glare layer"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let tex =
                |binding, view| wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::TextureView(view) };
            let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("solar glare visibility"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() },
                    tex(1, depth),
                    tex(2, cloud.unwrap_or(&glare.fallback)),
                ],
            });
            let art = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("solar glare artwork"),
                layout: &pipeline.get_bind_group_layout(1),
                entries: &[
                    tex(0, image),
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&glare.sampler) },
                ],
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("solar glare"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.set_bind_group(1, &art, &[]);
            pass.draw(0..6, 0..1);
        }
        self.sun_glare = Some(glare);
    }
}
