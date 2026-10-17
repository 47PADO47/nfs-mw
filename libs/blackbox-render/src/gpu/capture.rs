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

impl Captures {
    /// A new capture id. Ids are never reused, so a view capture and a sync capture cannot share one.
    pub(super) fn next_id(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        id
    }
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
        let id = self.captures.next_id();
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
        if width == 0 || height == 0 {
            return Err(RenderError::Device("cannot capture an empty image".into()));
        }
        // PNG captures have an explicit SDR contract, including when the window is scRGB.
        let old_output =
            std::mem::replace(&mut self.output, super::output::Output::texture(&self.device, (width, height)));
        let old_mapping = self.post.display;
        let old_ui = self.ui.hdr_white();
        if old_mapping.is_some() {
            self.post.set_display(
                &self.device,
                Some(super::post::display::Mapping { peak: 1.0, white: 80.0, linear: false }),
            );
        }
        self.ui.set_hdr_white(None);
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
        let scene = SceneTargets::new(&self.device, &self.scene_plan((width, height), false));
        self.submit_frame(Some(&scene), &view, (width, height), frame, instances);
        let result = self.read_texture_bytes(&target, (width, height));
        self.output = old_output;
        self.post.set_display(&self.device, old_mapping);
        self.ui.set_hdr_white(old_ui);
        result
    }

    /// Copy `texture` (the output format, `size` pixels) to the CPU as tightly packed RGBA8 with alpha 255.
    fn read_texture_bytes(&self, texture: &wgpu::Texture, size: (u32, u32)) -> Result<Vec<u8>, RenderError> {
        if !matches!(
            texture.format(),
            wgpu::TextureFormat::Rgba8Unorm
                | wgpu::TextureFormat::Bgra8Unorm
                | wgpu::TextureFormat::Rgba8UnormSrgb
                | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            return Err(RenderError::Device("Float output requires an HDR float capture, not RGBA8 readback".into()));
        }
        let bytes = self.read_raw_texture(texture, size, 4)?;
        let bgra = matches!(texture.format(), wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
        let mut out = Vec::with_capacity(bytes.len());
        for px in bytes.as_chunks::<4>().0 {
            let (r, b) = match bgra {
                true => (px[2], px[0]),
                false => (px[0], px[2]),
            };
            out.extend_from_slice(&[r, px[1], b, 255]);
        }
        Ok(out)
    }

    /// Linear BT.709 scRGB values (1.0 = 80 nits), preserving signed and >1 components.
    pub fn capture_hdr_linear(
        &mut self,
        width: u32,
        height: u32,
        frame: &FrameParams,
        instances: &[Instance],
    ) -> Result<Vec<[f32; 4]>, RenderError> {
        if width == 0 || height == 0 || !self.post.display.is_some_and(|m| m.linear) {
            return Err(RenderError::Device(
                "HDR float capture requires active HDR mapping and a nonempty size".into(),
            ));
        }
        let output =
            super::output::Output::texture_format(&self.device, (width, height), wgpu::TextureFormat::Rgba16Float);
        let old = std::mem::replace(&mut self.output, output);
        let super::output::Output::Texture { texture, .. } = &self.output else { unreachable!() };
        let target = texture.clone();
        let view = target.create_view(&Default::default());
        let scene = SceneTargets::new(&self.device, &self.scene_plan((width, height), false));
        self.submit_frame(Some(&scene), &view, (width, height), frame, instances);
        let result = self.read_raw_texture(&target, (width, height), 8).map(|bytes| {
            bytes
                .as_chunks::<8>()
                .0
                .iter()
                .map(|pixel| {
                    std::array::from_fn(|i| half::f16::from_le_bytes([pixel[2 * i], pixel[2 * i + 1]]).to_f32())
                })
                .collect()
        });
        self.output = old;
        result
    }

    pub(in crate::gpu) fn read_raw_texture(
        &self,
        texture: &wgpu::Texture,
        size: (u32, u32),
        bytes_per_pixel: u32,
    ) -> Result<Vec<u8>, RenderError> {
        let (width, height) = size;
        let row = (width * bytes_per_pixel).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
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
        Ok(pack_rgba(&mapped, row, width, height, is_bgra(self.output.format())))
    }
}

/// Whether `format` stores blue first, so its readback needs the red and blue swapped.
pub(super) fn is_bgra(format: wgpu::TextureFormat) -> bool {
    matches!(format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb)
}

/// Repack a mapped readback (rows `row` bytes apart, `width` by `height` pixels) as tightly packed RGBA8 with
/// alpha 255. `bgra` swaps red and blue on the way.
pub(super) fn pack_rgba(mapped: &[u8], row: u32, width: u32, height: u32, bgra: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height as usize {
        let line = &mapped[y * row as usize..][..width as usize * 4];
        for px in line.as_chunks::<4>().0 {
            let (r, b) = if bgra { (px[2], px[0]) } else { (px[0], px[2]) };
            out.extend_from_slice(&[r, px[1], b, 255]);
        }
    }
    out
}
