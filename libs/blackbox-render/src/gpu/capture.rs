//! Off-screen rendering to RGBA8, for screenshots and tests.

use super::{Renderer, resources};
use crate::{FrameParams, Instance, RenderError};

impl Renderer {
    /// Render one frame off-screen and return it as tightly packed RGBA8.
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
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = resources::create_depth(&self.device, width, height);
        let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture readback"),
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("capture") });
        self.encode_scene(&mut encoder, &view, Some(&depth), frame, instances);
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(height),
                },
            },
            size,
        );
        self.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let mapped = slice.get_mapped_range().map_err(|e| RenderError::Device(format!("capture readback: {e:?}")))?;
        let bgra = matches!(self.config.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
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
