//! Shared helpers of the headless GPU tests: a device, texture readback and image metrics.
//!
//! The tests need a Vulkan device (or lavapipe: set `FSR3_TEST_FALLBACK=1`), and are `#[ignore]`d so
//! `cargo test` stays GPU-free; run them with `cargo test -p fsr3-wgpu -- --ignored`.

#![allow(dead_code)]

pub mod scene;
mod scene_gpu;
mod scene_shader;

use glam::UVec2;

pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub name: String,
}

impl Gpu {
    pub fn new() -> Self {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = wgpu::Backends::VULKAN;
        let instance = wgpu::Instance::new(desc);
        let fallback = std::env::var_os("FSR3_TEST_FALLBACK").is_some();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: fallback,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .expect("a Vulkan adapter");
        let name = adapter.get_info().name;
        // The default limits on purpose: the upscaler has to work within them.
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("a device with the default limits");
        Self { device, queue, name }
    }

    /// Reads the first `size` bytes of a `COPY_SRC` buffer back.
    pub fn read_buffer(&self, buffer: &wgpu::Buffer, size: u64) -> Vec<u8> {
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_buffer_to_buffer(buffer, 0, &readback, 0, size);
        self.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        slice.get_mapped_range().unwrap().to_vec()
    }

    /// Reads an `Rgba32Float` texture back.
    pub fn read_rgba32f(&self, texture: &wgpu::Texture, size: UVec2) -> Vec<[f32; 4]> {
        let bytes = self.read_bytes(texture, size, 16);
        bytemuck::cast_slice::<u8, [f32; 4]>(&bytes).to_vec()
    }

    /// Reads an `Rgba16Float` texture back as floats.
    pub fn read_rgba16f(&self, texture: &wgpu::Texture, size: UVec2) -> Vec<[f32; 4]> {
        let bytes = self.read_bytes(texture, size, 8);
        let halves = bytemuck::cast_slice::<u8, [u16; 4]>(&bytes);
        halves.iter().map(|h| h.map(decode_f16)).collect()
    }

    pub fn read_bytes(&self, texture: &wgpu::Texture, size: UVec2, texel: u32) -> Vec<u8> {
        let row = (size.x * texel).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(row * size.y),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(size.y),
                },
            },
            wgpu::Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let mapped = slice.get_mapped_range().unwrap();
        let mut bytes = Vec::with_capacity((size.x * size.y * texel) as usize);
        for y in 0..size.y as usize {
            bytes.extend_from_slice(&mapped[y * row as usize..][..(size.x * texel) as usize]);
        }
        bytes
    }
}

/// IEEE half to single.
pub fn decode_f16(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let mantissa = f32::from(bits & 0x3ff);
    match exponent {
        0 => sign * mantissa * 2f32.powi(-24),
        31 if mantissa == 0.0 => sign * f32::INFINITY,
        31 => f32::NAN,
        _ => sign * (1.0 + mantissa / 1024.0) * 2f32.powi(exponent - 15),
    }
}

/// The peak signal-to-noise ratio of the RGB channels in dB (peak 1.0), ignoring `border` pixels
/// around the image.
pub fn psnr(a: &[[f32; 4]], b: &[[f32; 4]], size: UVec2, border: u32) -> f64 {
    let mse = mean_squared_error(a, b, size, border);
    if mse <= 0.0 {
        return 200.0;
    }
    10.0 * (1.0 / mse).log10()
}

pub fn mean_squared_error(a: &[[f32; 4]], b: &[[f32; 4]], size: UVec2, border: u32) -> f64 {
    assert_eq!(a.len(), (size.x * size.y) as usize);
    assert_eq!(b.len(), a.len());
    let mut sum = 0.0_f64;
    let mut count = 0.0_f64;
    for y in border..size.y - border {
        for x in border..size.x - border {
            let i = (y * size.x + x) as usize;
            for c in 0..3 {
                let d = f64::from(a[i][c]) - f64::from(b[i][c]);
                sum += d * d;
                count += 1.0;
            }
        }
    }
    sum / count
}

/// A bilinear resize of an image, to compare a plain upscale with the temporal one.
pub fn resize_bilinear(src: &[[f32; 4]], from: UVec2, to: UVec2) -> Vec<[f32; 4]> {
    let mut out = Vec::with_capacity((to.x * to.y) as usize);
    for y in 0..to.y {
        for x in 0..to.x {
            let sx = ((x as f32 + 0.5) * from.x as f32 / to.x as f32 - 0.5).clamp(0.0, from.x as f32 - 1.0);
            let sy = ((y as f32 + 0.5) * from.y as f32 / to.y as f32 - 0.5).clamp(0.0, from.y as f32 - 1.0);
            let (x0, y0) = (sx.floor() as u32, sy.floor() as u32);
            let (x1, y1) = ((x0 + 1).min(from.x - 1), (y0 + 1).min(from.y - 1));
            let (fx, fy) = (sx - x0 as f32, sy - y0 as f32);
            let at = |px: u32, py: u32| src[(py * from.x + px) as usize];
            let mut texel = [1.0; 4];
            for (c, value) in texel.iter_mut().enumerate().take(3) {
                let top = at(x0, y0)[c] * (1.0 - fx) + at(x1, y0)[c] * fx;
                let bottom = at(x0, y1)[c] * (1.0 - fx) + at(x1, y1)[c] * fx;
                *value = top * (1.0 - fy) + bottom * fy;
            }
            out.push(texel);
        }
    }
    out
}

/// Writes `image` as a binary PPM named `name` into the directory of `FSR3_TEST_DUMP`, if that is set:
/// a way to look at the results while developing. Values are clamped to `[0, 1]` and not gamma encoded.
pub fn dump_ppm(name: &str, image: &[[f32; 4]], size: UVec2) {
    let Some(dir) = std::env::var_os("FSR3_TEST_DUMP") else {
        return;
    };
    let mut bytes = format!("P6\n{} {}\n255\n", size.x, size.y).into_bytes();
    for texel in image {
        bytes.extend(texel[..3].iter().map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8));
    }
    std::fs::write(std::path::Path::new(&dir).join(format!("{name}.ppm")), bytes).expect("a dump file");
}

/// Whether every channel of every texel is finite.
pub fn all_finite(image: &[[f32; 4]]) -> bool {
    image.iter().all(|t| t.iter().all(|v| v.is_finite()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_floats_decode() {
        assert_eq!(decode_f16(0x3c00), 1.0);
        assert_eq!(decode_f16(0xc000), -2.0);
        assert_eq!(decode_f16(0x3800), 0.5);
        assert_eq!(decode_f16(0), 0.0);
        assert!(decode_f16(0x7e00).is_nan());
        assert_eq!(decode_f16(0x7c00), f32::INFINITY);
    }

    #[test]
    fn psnr_of_equal_images_is_high_and_of_a_known_error_is_exact() {
        let size = UVec2::new(8, 8);
        let a = vec![[0.5, 0.5, 0.5, 1.0]; 64];
        let mut b = a.clone();
        assert_eq!(psnr(&a, &b, size, 0), 200.0);
        b.fill([0.6, 0.6, 0.6, 1.0]);
        assert!((psnr(&a, &b, size, 0) - 20.0).abs() < 1e-4);
    }
}
