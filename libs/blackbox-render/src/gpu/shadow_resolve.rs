//! Optional non-recursive, two-frame reconstruction for cascaded shadow visibility.
#[cfg(test)]
#[path = "shadow_resolve_tests.rs"]
mod tests;
use super::{
    Renderer,
    ray_shadows::{self, history},
    surface::targets::Targets,
};
use blackbox_gfx::FrameParams;
use wgpu::util::DeviceExt;
pub(super) struct Resolve {
    size: (u32, u32),
    raw: wgpu::TextureView,
    mask: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    history: history::History,
}
impl Resolve {
    fn new(device: &wgpu::Device, size: (u32, u32)) -> Self {
        let source = format!(
            "{}\n{}",
            include_str!("../shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "1"),
            include_str!("../shaders/shadow_resolve.wgsl")
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cascade visibility resolve"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cascade visibility resolve"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            size,
            raw: raw(device, size),
            mask: ray_shadows::mask(device, size),
            pipeline,
            history: history::History::new(device, size),
        }
    }
    fn encode(&mut self, r: &Renderer, encoder: &mut wgpu::CommandEncoder, frame: &FrameParams, surfaces: &Targets) {
        let inverse = r.world_to_clip(frame).inverse();
        let mut values = inverse.to_cols_array().to_vec();
        values.extend([self.size.0 as f32, self.size.1 as f32, u8::from(r.experiments.contact_shadows) as f32, 0.0]);
        let buffer = r.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cascade visibility inverse"),
            contents: bytemuck::cast_slice(&values),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let texture =
            |binding, view| wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::TextureView(view) };
        let group = r.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cascade visibility surfaces"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
                texture(1, &surfaces.depth),
                texture(2, &surfaces.normals),
            ],
        });
        let maps = &r.shared.shadows;
        let atlas = r.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cascade visibility atlas"),
            layout: &self.pipeline.get_bind_group_layout(1),
            entries: &[
                wgpu::BindGroupEntry { binding: 1, resource: maps.buffer.as_entire_binding() },
                texture(2, &maps.view),
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&maps.sampler) },
                texture(4, &maps.contact),
                texture(6, &maps.detail),
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("cascade visibility"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.raw,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.set_bind_group(1, &atlas, &[]);
        pass.draw(0..3, 0..1);
        drop(pass);
        self.history.resolve(
            &r.device,
            encoder,
            history::Draw {
                source: &self.raw,
                output: &self.mask,
                surfaces,
                frame,
                options: &r.experiments,
                view_proj: r.world_to_clip(frame),
                motion: r.effect_motion.as_ref(),
            },
        );
    }
}
fn raw(device: &wgpu::Device, size: (u32, u32)) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("unblended cascade visibility"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
impl Renderer {
    pub(super) fn cascade_history_active(&self, frame: &FrameParams) -> bool {
        self.experiments.cascade_temporal
            && self.experiments.sun_shadows
            && self.experiments.shadow_projection.is_none()
            && !self.ray_shadows_active(frame)
            && self.screen_lighting_active(frame)
    }
    pub(super) fn encode_cascade_history(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &FrameParams) {
        if !self.cascade_history_active(frame) {
            if let Some(resolve) = &mut self.cascade_resolve {
                resolve.history.invalidate();
            }
            return;
        }
        let Some(surfaces) = &self.surfaces else { return };
        let size = surfaces.targets.size;
        let mut resolve =
            self.cascade_resolve.take().filter(|p| p.size == size).unwrap_or_else(|| Resolve::new(&self.device, size));
        resolve.encode(self, encoder, frame, &surfaces.targets);
        if self.shared.shadows.contact != resolve.mask {
            self.shared.shadows.set_contact(&self.device, &self.shared.glossy.material, resolve.mask.clone());
            if let Some(glossy) = &mut self.glossy {
                glossy.rebind_shadows(&self.device, &self.shared.shadows);
            }
        }
        self.cascade_resolve = Some(resolve);
    }
}
