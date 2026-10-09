//! Check sampled color/alpha, animated-cell UVs and depth on actual GPU backends.

use super::{BACKGROUND, Gpu, quad};
use crate::{BlendMode, EffectLayer, TextureHandle, TexturedEffect};

#[test]
#[ignore = "needs a Vulkan GPU"]
fn textured_particles_vulkan() {
    check(wgpu::Backends::VULKAN);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "needs a Direct3D 12 GPU"]
fn textured_particles_dx12() {
    check(wgpu::Backends::DX12);
}

fn check(backend: wgpu::Backends) {
    let mut gpu = Gpu::new(backend);
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("synthetic particle atlas"),
        size: wgpu::Extent3d { width: 2, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    gpu.queue.write_texture(
        texture.as_image_copy(),
        &[100, 0, 0, 128, 0, 0, 0, 0],
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(8), rows_per_image: Some(1) },
        texture.size(),
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("synthetic particle atlas"),
        layout: &gpu.shared.texture_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&gpu.shared.sampler) },
        ],
    });
    let handle = TextureHandle::from_raw(gpu.textures.insert(group));
    let mut vertices = Vec::new();
    quad(&mut vertices, 0.8, [128, 255, 255, 128]);
    vertices.iter_mut().for_each(|v| v.uv = [0.25, 0.5]);
    let mut layer = EffectLayer::default();
    layer.textured.push(TexturedEffect { texture: handle, blend: BlendMode::Additive, vertices });
    let fog = [f32::MAX, f32::MAX];
    let front = gpu.sample(&layer, 0.6, 64, fog).center();
    assert!(
        (28..=30).contains(&front[0]),
        "texture color, texture alpha and vertex alpha must all contribute: {front:?}"
    );
    assert_eq!(&front[1..], &BACKGROUND[1..]);
    assert_eq!(gpu.sample(&layer, 0.9, 64, fog).center(), BACKGROUND, "the car's opaque depth occludes stock sprites");
    layer.textured[0].vertices.iter_mut().for_each(|v| v.uv = [0.75, 0.5]);
    assert_eq!(gpu.sample(&layer, 0.0, 64, fog).center(), BACKGROUND, "animated UVs select the transparent atlas cell");
    layer.textured[0].vertices.iter_mut().for_each(|v| v.uv = [0.25, 0.5]);
    let mut farther = layer.textured[0].vertices.clone();
    farther.iter_mut().for_each(|v| v.position[2] = 0.2);
    layer.textured.push(TexturedEffect { texture: handle, blend: BlendMode::Additive, vertices: farther });
    assert!(gpu.sample(&layer, 0.0, 64, fog).center()[0] > front[0] + 10, "particles do not write depth");
    assert_eq!(gpu.sample(&layer, 0.0, 64, [0.0, 0.1]).center(), BACKGROUND, "additive light disappears in fog");
    layer.clear();
    assert_eq!(gpu.sample(&layer, 0.0, 64, fog).center(), BACKGROUND, "style switching clears all GPU batches");
}
