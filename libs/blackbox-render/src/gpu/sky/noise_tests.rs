//! A checker field must converge to its mean, including a one-slice weather map.
#[test]
#[ignore = "needs a GPU"]
fn mip_filter_preserves_mean_energy_in_volume_and_single_slice_fields() {
    let _gpu = crate::gpu::test_support::serial();
    let Some(renderer) = crate::gpu::test_support::headless((8, 8)) else { return };
    let device = &renderer.device;
    for depth in [1, 4] {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("checker density"),
            size: wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: depth },
            mip_level_count: 3,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let mut pixels = Vec::new();
        for z in 0..depth {
            for y in 0..4 {
                for x in 0..4 {
                    let v = (((x + y + z) % 2) * 255) as u8;
                    pixels.extend([v, v, v, 255]);
                }
            }
        }
        renderer.queue.write_texture(
            texture.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(16), rows_per_image: Some(4) },
            texture.size(),
        );
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("density mean"),
            size: 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        super::mips(device, &mut encoder, &texture);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 2,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(256), rows_per_image: Some(1) },
            },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        );
        renderer.queue.submit([encoder.finish()]);
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let bytes = readback.slice(..).get_mapped_range().unwrap();
        for (actual, expected) in bytes[..4].iter().zip([128u8, 128, 128, 255]) {
            assert!(actual.abs_diff(expected) <= 1, "depth {depth}: {:?}", &bytes[..4]);
        }
    }
}
