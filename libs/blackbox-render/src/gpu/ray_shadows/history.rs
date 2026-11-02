//! Two unblended spatial frames only. The resolved output is never fed back into history.
use super::{
    super::{surface::targets::Targets, temporal::EffectMotion},
    Groups, filter,
};
use blackbox_gfx::{FrameParams, Projection, RenderExperiments};
use glam::{Mat4, Vec3};
pub(in crate::gpu) struct Draw<'a> {
    pub source: &'a wgpu::TextureView,
    pub output: &'a wgpu::TextureView,
    pub surfaces: &'a Targets,
    pub frame: &'a FrameParams,
    pub options: &'a RenderExperiments,
    pub view_proj: Mat4,
    pub motion: Option<&'a EffectMotion>,
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    previous_inverse: [f32; 16],
    screen: [f32; 4],
    control: [f32; 4],
    jitter: [f32; 4],
}
pub(in crate::gpu) struct History {
    images: [wgpu::TextureView; 2],
    index: usize,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    /// Rewritten each frame; one buffer for the renderer's lifetime.
    buffer: wgpu::Buffer,
    /// Inputs keyed by source, stored history, depth, normals and motion views (two per ping-pong).
    groups: Groups<5>,
    valid: bool,
    previous_inverse: Mat4,
    near: f32,
    options: Option<RenderExperiments>,
    sun: Vec3,
}
impl History {
    #[cfg(test)]
    pub(super) fn stored(&self) -> &wgpu::TextureView {
        &self.images[self.index]
    }
    pub fn new(device: &wgpu::Device, size: (u32, u32)) -> Self {
        Self::with_cache(device, size, None)
    }
    pub fn with_cache(device: &wgpu::Device, size: (u32, u32), cache: Option<&wgpu::PipelineCache>) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("two-frame sun resolve"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/ray_shadow_history.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("two-frame sun resolve"),
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
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rg8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
            }),
            multiview_mask: None,
            cache,
        });
        Self {
            images: [filter::raw(device, size), filter::raw(device, size)],
            index: 0,
            layout: pipeline.get_bind_group_layout(0),
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sun history rejection"),
                size: size_of::<Params>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            groups: Groups::default(),
            pipeline,
            valid: false,
            previous_inverse: Mat4::IDENTITY,
            near: 0.1,
            options: None,
            sun: Vec3::ZERO,
        }
    }
    pub fn invalidate(&mut self) {
        self.valid = false;
    }
    /// Resolve a source without a stored neighbourhood range, uploading the parameters through `encoder`.
    pub fn resolve(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, draw: Draw<'_>) {
        self.resolve_with(device, None, encoder, draw);
    }
    /// Resolve the RT spatial filter's output, whose blue/alpha hold its neighbourhood range.
    pub fn resolve_ranged(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        draw: Draw<'_>,
    ) {
        self.resolve_with(device, Some(queue), encoder, draw);
    }
    /// `queue` writes the parameters directly and marks the source as ranged.
    fn resolve_with(
        &mut self,
        device: &wgpu::Device,
        queue: Option<&wgpu::Queue>,
        encoder: &mut wgpu::CommandEncoder,
        draw: Draw<'_>,
    ) {
        let Projection::PerspectiveInfiniteReverse { near, fov_y, .. } = draw.frame.projection else {
            self.invalidate();
            return;
        };
        let sun = draw.frame.light_dir.normalize_or_zero();
        let valid = self.valid
            && !draw.frame.camera_cut
            && self.options == Some(*draw.options)
            && self.sun.dot(sun) > 0.999999
            && draw.motion.is_some_and(|m| !m.reset);
        let size = draw.surfaces.size;
        let jitter = draw.motion.map_or(glam::Vec2::ZERO, |m| m.jitter_delta);
        let params = Params {
            previous_inverse: self.previous_inverse.to_cols_array(),
            screen: [size.0 as f32, size.1 as f32, near, 2.0 * (fov_y * 0.5).tan() / size.1 as f32],
            control: [u8::from(valid) as f32, 0.5, self.near, u8::from(queue.is_some()) as f32],
            jitter: [jitter.x, jitter.y, 0.0, 0.0],
        };
        upload(device, queue, encoder, &self.buffer, &params);
        let tex = |binding, view| wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::TextureView(view) };
        let motion = draw.motion.map_or(draw.source, |m| &m.view);
        let (layout, buffer, stored) = (&self.layout, &self.buffer, &self.images[self.index]);
        let key = [draw.source, stored, &draw.surfaces.depth, &draw.surfaces.normals, motion];
        let group = self.groups.get(key, || {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("two-frame sun inputs"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
                    tex(1, draw.source),
                    tex(2, stored),
                    tex(3, &draw.surfaces.depth),
                    tex(4, &draw.surfaces.normals),
                    tex(5, motion),
                ],
            })
        });
        let next = 1 - self.index;
        let attachment = |view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("two-frame sun stabilization"),
            color_attachments: &[attachment(draw.output), attachment(&self.images[next])],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
        drop(pass);
        self.index = next;
        self.valid = true;
        self.options = Some(*draw.options);
        self.sun = sun;
        self.previous_inverse = draw.view_proj.inverse();
        self.near = near;
    }
}
fn upload(
    device: &wgpu::Device,
    queue: Option<&wgpu::Queue>,
    encoder: &mut wgpu::CommandEncoder,
    buffer: &wgpu::Buffer,
    params: &Params,
) {
    if let Some(queue) = queue {
        queue.write_buffer(buffer, 0, bytemuck::bytes_of(params));
        return;
    }
    use wgpu::util::DeviceExt;
    let staging = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sun history rejection upload"),
        contents: bytemuck::bytes_of(params),
        usage: wgpu::BufferUsages::COPY_SRC,
    });
    encoder.copy_buffer_to_buffer(&staging, 0, buffer, 0, size_of::<Params>() as u64);
}
