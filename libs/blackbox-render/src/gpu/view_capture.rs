//! The rear-view mirror's picture: a reduced frame drawn off-screen and read back without waiting for the GPU.
//!
//! A full capture ([`Renderer::begin_capture`]) draws the whole frame and blocks until it is read back. A view
//! capture skips the depth prepass, contact shadows, sun shadows, AO, motion vectors, SMAA, the UI and the GPU
//! timing. The scene pass reads no shadow or AO input when `reduced` is set in its globals. The frame is submitted
//! at once and its readback is mapped without a wait; [`Renderer::poll_view_capture`] takes the picture on a later
//! frame. One capture is in flight at a time, and its images are kept for the next capture of the same size.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use super::Renderer;
use super::capture::{is_bgra, pack_rgba};
use super::output::{HEADLESS_FORMAT, Output};
use super::targets::{ScenePlan, SceneTargets};
use crate::{FrameParams, Instance, RenderError};
use blackbox_gfx::{CaptureId, RgbaImage};

/// The readback is still being mapped.
const WAITING: u8 = 0;
/// The readback is mapped and holds the picture.
const MAPPED: u8 = 1;
/// Mapping the readback failed.
const FAILED: u8 = 2;

/// The off-screen picture, its readback buffer and the capture in flight, if any.
pub(super) struct ViewCapture {
    size: (u32, u32),
    format: wgpu::TextureFormat,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    scene: SceneTargets,
    readback: wgpu::Buffer,
    /// Bytes per readback row, aligned for the copy.
    row: u32,
    pending: Option<Pending>,
}

/// A capture whose readback is being mapped.
struct Pending {
    id: u64,
    /// Set by the map callback to [`MAPPED`] or [`FAILED`].
    state: Arc<AtomicU8>,
}

impl ViewCapture {
    fn new(device: &wgpu::Device, size: (u32, u32), format: wgpu::TextureFormat, plan: &ScenePlan) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("view capture"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let row = (size.0 * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("view capture readback"),
            size: u64::from(row * size.1),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Self { size, format, texture, view, scene: SceneTargets::new(device, plan), readback, row, pending: None }
    }

    /// The mapped picture as an image. Unmaps the readback, which the next capture maps again.
    fn take_image(&mut self) -> Result<RgbaImage, RenderError> {
        self.pending = None;
        let (width, height) = self.size;
        let image = match self.readback.slice(..).get_mapped_range() {
            Ok(mapped) => {
                let rgba = pack_rgba(&mapped, self.row, width, height, is_bgra(self.format));
                RgbaImage::new(width, height, rgba)
            }
            Err(e) => Err(RenderError::Device(format!("view capture readback: {e:?}"))),
        };
        self.readback.unmap();
        image
    }
}

impl Renderer {
    /// Start a view capture at `size` (width, height): the reduced frame is drawn and submitted now, and its
    /// readback is mapped without a wait. [`Self::poll_view_capture`] returns the picture once the GPU is done.
    pub(super) fn begin_view_capture(
        &mut self,
        size: [u32; 2],
        frame: &FrameParams,
        instances: &[Instance],
    ) -> Result<CaptureId, RenderError> {
        let [width, height] = size;
        if width == 0 || height == 0 {
            return Err(RenderError::Device(format!("cannot capture a {width}x{height} image")));
        }
        if self.view_capture.as_ref().is_some_and(|capture| capture.pending.is_some()) {
            return Err(RenderError::Device("a view capture is still in flight".into()));
        }
        // UI pictures have an RGBA8 SDR contract even when the main window uses float HDR.
        let mapping = self.post.display;
        if mapping.is_some() {
            self.post.set_display(
                &self.device,
                Some(super::post::display::Mapping { peak: 1.0, white: 80.0, linear: false }),
            );
        }
        let format = HEADLESS_FORMAT;
        let plan = self.scene_plan_for_format((width, height), false, format);
        let cached = self
            .view_capture
            .take()
            .filter(|c| c.size == (width, height) && c.format == format && c.scene.matches(&plan));
        let mut capture = cached.unwrap_or_else(|| ViewCapture::new(&self.device, (width, height), format, &plan));
        let output = std::mem::replace(
            &mut self.output,
            Output::Texture { texture: capture.texture.clone(), size: (width, height) },
        );

        let id = self.captures.next_id();
        self.update_irradiance();
        self.begin_temporal(false, frame);
        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("view capture") });
        self.instances.upload_instances(&self.device, &self.queue, instances);
        self.encode_scene_mode(&mut encoder, Some(&capture.scene), &capture.view, frame, instances, true, None);
        self.encode_post(&mut encoder, Some(&capture.scene), &capture.view, (width, height));
        encoder.copy_texture_to_buffer(
            capture.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &capture.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(capture.row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);
        self.end_temporal();
        self.output = output;
        self.post.set_display(&self.device, mapping);

        let state = Arc::new(AtomicU8::new(WAITING));
        let on_map = Arc::clone(&state);
        capture.readback.slice(..).map_async(wgpu::MapMode::Read, move |result| {
            on_map.store(if result.is_ok() { MAPPED } else { FAILED }, Ordering::Release);
        });
        capture.pending = Some(Pending { id, state });
        self.view_capture = Some(capture);
        Ok(CaptureId::from_raw(id))
    }

    /// The picture of view capture `id` once its readback is mapped. `None` while the GPU still works on it, and
    /// for any other id. Each capture is returned once.
    pub(super) fn poll_view_capture(&mut self, id: u64) -> Option<Result<RgbaImage, RenderError>> {
        let pending_id = self.view_capture.as_ref()?.pending.as_ref()?.id;
        if pending_id != id {
            return None;
        }
        let _ = self.device.poll(wgpu::PollType::Poll);
        let capture = self.view_capture.as_mut()?;
        let state = capture.pending.as_ref()?.state.load(Ordering::Acquire);
        match state {
            WAITING => None,
            MAPPED => Some(capture.take_image()),
            _ => {
                capture.pending = None;
                Some(Err(RenderError::Device("view capture readback could not be mapped".into())))
            }
        }
    }
}
