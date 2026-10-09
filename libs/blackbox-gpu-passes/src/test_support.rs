//! Helpers for GPU tests, here and in the crates that call these passes (enable the `test-support`
//! feature).
//!
//! The tests run in parallel threads, and creating Vulkan devices at the same time crashes some Mesa
//! drivers, so every test that creates a device holds [`serial`] for its whole length.

use std::sync::{Mutex, MutexGuard, PoisonError};

static GPU: Mutex<()> = Mutex::new(());

/// Hold this for the whole of a test that creates a GPU device. The tests run in parallel threads, and
/// creating Vulkan devices at the same time crashes some Mesa drivers (SIGSEGV), so they take turns.
pub fn serial() -> MutexGuard<'static, ()> {
    GPU.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A device and queue on the first high-performance adapter of `backend`, with no surface.
pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    /// Panics when the machine has no adapter for `backend` (the tests that use it are `#[ignore]`).
    pub fn new(backend: wgpu::Backends) -> Self {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = backend;
        let instance = wgpu::Instance::new(desc);
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .expect("test GPU adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        Self { device, queue }
    }

    /// A `size` x `size` `Rgba8Unorm` image to render into and read back with [`Self::finish`].
    pub fn target(&self, size: u32) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test target"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    /// Submit `encoder` after copying the square `target` out of it, and return its pixels as rows of
    /// RGBA bytes (no row padding).
    pub fn finish(&self, mut encoder: wgpu::CommandEncoder, target: &wgpu::Texture) -> Vec<u8> {
        let width = target.width();
        let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("test readback"),
            size: u64::from(row * width),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(width),
                },
            },
            target.size(),
        );
        self.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let mapped = slice.get_mapped_range().unwrap();
        (0..width as usize).flat_map(|y| mapped[y * row as usize..][..(width * 4) as usize].iter().copied()).collect()
    }
}
