//! One view's fog: its volume, half-resolution image, parameters and history, kept between frames.
use super::grid::Volume;
use super::params::{Params, Previous};
use super::{FogView, Pipelines};

/// Frames a view's history stays usable without a draw (a split-screen half is drawn every other capture).
const HISTORY_FRAMES: u64 = 8;
/// Furthest the camera may move between two draws of a view and keep its history.
pub(super) const HISTORY_DISTANCE: f32 = 10.0;

struct History {
    view_proj: glam::Mat4,
    camera: glam::Vec3,
    options: [f32; 2],
    distance: f32,
    frame: u64,
}

struct Image {
    size: (u32, u32),
    view: wgpu::TextureView,
    group: wgpu::BindGroup,
}

pub(super) struct ViewFog {
    pub(super) key: FogView,
    /// Full-resolution size of the depth the view was last drawn with.
    pub(super) size: (u32, u32),
    pub(super) last_frame: u64,
    uniform: wgpu::Buffer,
    volume: Option<Volume>,
    image: Option<Image>,
    /// The input bind group and the depth view it reads.
    inputs: Option<(wgpu::TextureView, wgpu::BindGroup)>,
    history: Option<History>,
    /// Camera position of the last draw, kept when the history is dropped.
    camera: Option<glam::Vec3>,
}

/// What a view needs to decide whether its history applies.
pub(super) struct Frame {
    pub(super) number: u64,
    pub(super) camera: glam::Vec3,
    pub(super) camera_cut: bool,
    pub(super) options: [f32; 2],
    pub(super) distance: f32,
}

impl ViewFog {
    pub(super) fn new(device: &wgpu::Device, key: FogView, size: (u32, u32)) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("light fog parameters"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { key, size, last_frame: 0, uniform, volume: None, image: None, inputs: None, history: None, camera: None }
    }

    /// Camera position of the last draw.
    pub(super) fn camera(&self) -> Option<glam::Vec3> {
        self.camera
    }

    pub(super) fn forget_history(&mut self) {
        self.history = None;
    }

    /// Make the volume and image fit `size` and `dimensions`, reusing them when they already do.
    pub(super) fn prepare(&mut self, device: &wgpu::Device, gpu: &Pipelines, size: (u32, u32), dimensions: [u32; 3]) {
        self.size = size;
        if self.volume.as_ref().is_none_or(|volume| volume.size() != dimensions) {
            self.volume = Some(gpu.grid.volume(device, dimensions, &self.uniform));
            self.inputs = None;
            self.history = None;
        }
        let half = (size.0.div_ceil(2).max(1), size.1.div_ceil(2).max(1));
        if self.image.as_ref().is_some_and(|image| image.size == half) {
            return;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("light fog half resolution"),
            size: wgpu::Extent3d { width: half.0, height: half.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("light fog image"),
            layout: &gpu.texture_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }],
        });
        self.image = Some(Image { size: half, view, group });
    }

    /// The previous frame to reproject, when nothing invalidates it.
    pub(super) fn previous(&self, frame: &Frame) -> Option<Previous> {
        let history = self.history.as_ref()?;
        let usable = !frame.camera_cut
            && frame.number.saturating_sub(history.frame) <= HISTORY_FRAMES
            && history.camera.distance(frame.camera) < HISTORY_DISTANCE
            && (history.distance - frame.distance).abs() < 0.01
            && (history.options[0] - frame.options[0]).abs() < 0.01
            && (history.options[1] - frame.options[1]).abs() < 0.05;
        usable.then_some(Previous { view_proj: history.view_proj, camera: history.camera, distance: history.distance })
    }

    /// Write the parameters and record the volume, its integration and the composite into `draw.target`.
    pub(super) fn encode(
        &mut self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        gpu: &mut Pipelines,
        encoder: &mut wgpu::CommandEncoder,
        draw: &crate::SoftDraw<'_>,
        (params, frame): (&Params, &Frame),
    ) {
        let (Some(volume), Some(image)) = (self.volume.as_mut(), self.image.as_ref()) else { return };
        let Some(composite) = gpu.composite.get(&gpu.format) else { return };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(params));
        if self.inputs.as_ref().is_none_or(|(depth, _)| depth != draw.depth) {
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("light fog inputs"),
                layout: &gpu.input_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(draw.depth) },
                    wgpu::BindGroupEntry { binding: 1, resource: self.uniform.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: volume.integrated().as_entire_binding() },
                ],
            });
            self.inputs = Some((draw.depth.clone(), group));
        }
        let inputs = &self.inputs.as_ref().unwrap().1;
        gpu.grid.encode(device, encoder, draw.globals, volume, draw.local_fog_shadows.as_ref());
        self.history = Some(History {
            view_proj: draw.inverse_view_proj.inverse(),
            camera: draw.camera_position,
            options: frame.options,
            distance: frame.distance,
            frame: frame.number,
        });
        self.last_frame = frame.number;
        self.camera = Some(draw.camera_position);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("integrate light fog"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &image.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&gpu.integrate);
            pass.set_bind_group(0, draw.globals, &[]);
            pass.set_bind_group(1, inputs, &[]);
            pass.draw(0..3, 0..1);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("composite light fog"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: draw.target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            ..Default::default()
        });
        pass.set_pipeline(composite);
        pass.set_bind_group(0, draw.globals, &[]);
        pass.set_bind_group(1, inputs, &[]);
        pass.set_bind_group(2, &image.group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Bytes of GPU memory the view holds.
    pub(super) fn bytes(&self) -> u64 {
        let image = self.image.as_ref().map_or(0, |image| u64::from(image.size.0) * u64::from(image.size.1) * 8);
        self.uniform.size() + image + self.volume.as_ref().map_or(0, Volume::bytes)
    }
}

#[cfg(test)]
impl ViewFog {
    /// The integrated volume buffer, to tell a kept volume from a reallocated one.
    pub(super) fn integrated(&self) -> Option<wgpu::Buffer> {
        self.volume.as_ref().map(|volume| volume.integrated().clone())
    }

    pub(super) fn has_history(&self) -> bool {
        self.history.is_some()
    }
}
