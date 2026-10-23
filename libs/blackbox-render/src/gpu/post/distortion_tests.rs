use super::{
    PostChain,
    distortion::NAME,
    tests::{Gpu, near},
};
use blackbox_gfx::{Antialiasing, DistortionRegion, PostSettings};

#[test]
#[ignore = "requires a Vulkan GPU"]
fn screen_displacement_vulkan() {
    check(wgpu::Backends::VULKAN);
}
#[test]
#[cfg(target_os = "windows")]
#[ignore = "requires a Direct3D 12 GPU"]
fn screen_displacement_dx12() {
    check(wgpu::Backends::DX12);
}

fn check(backend: wgpu::Backends) {
    let _lock = crate::gpu::test_support::serial();
    let gpu = Gpu::new(backend);
    let map = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("synthetic displacement"),
        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    gpu.queue.write_texture(
        map.as_image_copy(),
        &[0, 128, 128, 255],
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
        map.size(),
    );
    let view = map.create_view(&Default::default());
    let mut chain = PostChain::new(&gpu.device, None);
    let region = DistortionRegion {
        uv_rect: [0.0, 0.0, 1.0, 1.0],
        rect: [0.25, 0.25, 0.5, 0.5],
        sample_transform: [0.0, 0.0, 1.0, 1.0],
        opacity: 1.0,
        brightness: 1.2,
    };
    chain.set_distortion(&gpu.device, Some(&view), &[region]);
    chain.set_effects(&gpu.device, PostSettings { antialiasing: Antialiasing::Fxaa, ..Default::default() });
    assert_eq!(chain.names(), ["fxaa", NAME]);
    let pixels = gpu.run(&mut chain, (64, 64), wgpu::Color { r: 0.5, g: 0.25, b: 0.1, a: 1.0 }, (64, 64));
    assert!(near(pixels[0], [128, 64, 26, 255]), "outside droplet {:?}", pixels[0]);
    assert!(near(pixels[32 * 64 + 32], [153, 77, 31, 255]), "stock 1.2 multiplier {:?}", pixels[32 * 64 + 32]);
    chain.set_distortion(&gpu.device, None, &[]);
    assert_eq!(chain.names(), ["fxaa"]);
}
