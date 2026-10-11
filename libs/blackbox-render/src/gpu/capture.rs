//! Off-screen rendering to RGBA8, for screenshots and tests.

use std::collections::HashMap;

use super::{Renderer, targets::SceneTargets};
use crate::{FrameParams, Instance, RenderError};
use blackbox_gfx::{CaptureId, RgbaImage};

/// Captures this renderer finished and nobody has polled for yet. The renderer reads a frame back
/// before `request_capture` returns, so the first poll always finds it.
#[derive(Default)]
pub(super) struct Captures {
    next: u64,
    done: HashMap<u64, RgbaImage>,
}

impl Renderer {
    /// Render one frame off-screen at `size` (width, height) like [`Self::capture`] and keep the image
    /// for [`Self::take_capture`].
    pub(super) fn begin_capture(
        &mut self,
        size: [u32; 2],
        frame: &FrameParams,
        instances: &[Instance],
    ) -> Result<CaptureId, RenderError> {
        let [width, height] = size;
        if width == 0 || height == 0 {
            return Err(RenderError::Device(format!("cannot capture a {width}x{height} image")));
        }
        let rgba = self.capture(width, height, frame, instances)?;
        let image = RgbaImage::new(width, height, rgba)?;
        let id = self.captures.next;
        self.captures.next += 1;
        self.captures.done.insert(id, image);
        Ok(CaptureId::from_raw(id))
    }

    /// The image of a finished capture. Each capture is returned once.
    pub(super) fn take_capture(&mut self, id: CaptureId) -> Option<RgbaImage> {
        self.captures.done.remove(&id.raw())
    }

    /// Read back the last frame drawn into a headless renderer's output texture, tightly packed RGBA8.
    /// `None` for a renderer with a window, whose frames are presented and gone.
    pub fn read_output(&mut self) -> Option<Result<RgbaImage, RenderError>> {
        let super::output::Output::Texture { texture, size } = &self.output else { return None };
        let (texture, size) = (texture.clone(), *size);
        Some(self.read_texture_bytes(&texture, size).and_then(|rgba| RgbaImage::new(size.0, size.1, rgba)))
    }
}

impl Renderer {
    /// Render one frame off-screen, `width` by `height` pixels, and return it as tightly packed RGBA8.
    /// The scene is drawn at the current render scale and resolved to the full size, like on screen.
    pub fn capture(
        &mut self,
        width: u32,
        height: u32,
        frame: &FrameParams,
        instances: &[Instance],
    ) -> Result<Vec<u8>, RenderError> {
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("capture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.output.format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let scene = SceneTargets::new(&self.device, &self.scene_plan((width, height)));
        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("capture") });
        self.encode_scene(&mut encoder, Some(&scene), &view, frame, instances);
        self.encode_post(&mut encoder, Some(&scene), &view, (width, height));
        self.encode_ui(&mut encoder, &view, (width, height));
        self.queue.submit([encoder.finish()]);
        self.read_texture_bytes(&target, (width, height))
    }

    /// Copy `texture` (the output format, `size` pixels) to the CPU as tightly packed RGBA8 with alpha 255.
    fn read_texture_bytes(&self, texture: &wgpu::Texture, size: (u32, u32)) -> Result<Vec<u8>, RenderError> {
        let (width, height) = size;
        let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture readback"),
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("readback") });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let mapped = slice.get_mapped_range().map_err(|e| RenderError::Device(format!("capture readback: {e:?}")))?;
        let bgra =
            matches!(self.output.format(), wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height as usize {
            let line = &mapped[y * row as usize..][..width as usize * 4];
            for px in line.as_chunks::<4>().0 {
                let (r, b) = if bgra { (px[2], px[0]) } else { (px[0], px[2]) };
                out.extend_from_slice(&[r, px[1], b, 255]);
            }
        }
        Ok(out)
    }
}
