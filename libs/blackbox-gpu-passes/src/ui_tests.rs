//! GPU checks of the UI pass on a real backend, with a plain wgpu device and no renderer around it.

use blackbox_gfx::{UiLayer, UiMesh, UiTextureId, UiTexturePatch, UiVertex};

use crate::UiPass;
use crate::test_support::{Gpu, serial};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 32;
const BACKGROUND: [u8; 4] = [10, 20, 30, 255];

#[test]
#[ignore = "needs a Vulkan GPU"]
fn ui_vulkan() {
    check(wgpu::Backends::VULKAN);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "needs a Direct3D 12 GPU"]
fn ui_dx12() {
    check(wgpu::Backends::DX12);
}

/// Two triangles covering `[x0, y0, x1, y1]` (points) with one premultiplied colour and UVs 0..1.
fn rect(texture: UiTextureId, [x0, y0, x1, y1]: [f32; 4], color: [u8; 4], clip: [f32; 4]) -> UiMesh {
    let vertex = |x: f32, y: f32, u: f32, v: f32| UiVertex { position: [x, y], uv: [u, v], color_rgba: color };
    UiMesh {
        vertices: vec![
            vertex(x0, y0, 0.0, 0.0),
            vertex(x1, y0, 1.0, 0.0),
            vertex(x1, y1, 1.0, 1.0),
            vertex(x0, y1, 0.0, 1.0),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        texture,
        clip,
    }
}

/// Draw `layer` over the background through `pass` and return the pixels.
fn draw(gpu: &Gpu, pass: &mut UiPass, layer: UiLayer) -> Vec<u8> {
    let target = gpu.target(SIZE);
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let [r, g, b, _] = BACKGROUND.map(|c| f64::from(c) / 255.0);
    drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("ui test background"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    }));
    pass.set_layer(layer);
    pass.encode(&gpu.device, &gpu.queue, &mut encoder, &view, FORMAT, (SIZE, SIZE));
    gpu.finish(encoder, &target)
}

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    pixels[((y * SIZE + x) * 4) as usize..][..4].try_into().unwrap()
}

fn check(backend: wgpu::Backends) {
    let _lock = serial();
    let gpu = Gpu::new(backend);
    let mut pass = UiPass::new(&gpu.device);
    let (white, checker) = (UiTextureId::from_raw(1), UiTextureId::from_raw(2));
    let patch = |id, size: [u32; 2], rgba: &'static [u8]| UiTexturePatch { id, offset: None, size, rgba };
    pass.update_texture(&gpu.device, &gpu.queue, &patch(white, [1, 1], &[255; 4])).unwrap();
    // 2x1 texture: red, then blue, both opaque (sampled with linear filtering, so look at the middle of each).
    let two = &[255, 0, 0, 255, 0, 0, 255, 255];
    pass.update_texture(&gpu.device, &gpu.queue, &patch(checker, [2, 1], two)).unwrap();

    // Nothing to draw leaves the background alone.
    let empty = draw(&gpu, &mut pass, UiLayer::default());
    assert_eq!(pixel(&empty, 16, 16), BACKGROUND);

    // An opaque quad covers the background; outside it stays untouched.
    let quad = rect(white, [8.0, 8.0, 24.0, 24.0], [200, 100, 50, 255], [0.0, 0.0, 32.0, 32.0]);
    let opaque = draw(&gpu, &mut pass, UiLayer { pixels_per_point: 1.0, meshes: vec![quad.clone()] });
    assert_eq!(pixel(&opaque, 16, 16), [200, 100, 50, 255]);
    assert_eq!(pixel(&opaque, 2, 2), BACKGROUND);

    // Points scale to pixels: the same quad at 2 pixels per point covers four times the area.
    let doubled = draw(&gpu, &mut pass, UiLayer { pixels_per_point: 0.5, meshes: vec![quad.clone()] });
    assert_eq!(pixel(&doubled, 6, 6), [200, 100, 50, 255], "8 points at 0.5 px per point start at pixel 4");
    assert_eq!(pixel(&doubled, 14, 14), BACKGROUND, "and end at pixel 12");

    // A clip rectangle cuts the quad.
    let clipped = rect(white, [8.0, 8.0, 24.0, 24.0], [200, 100, 50, 255], [0.0, 0.0, 16.0, 32.0]);
    let cut = draw(&gpu, &mut pass, UiLayer { pixels_per_point: 1.0, meshes: vec![clipped] });
    assert_eq!(pixel(&cut, 12, 16), [200, 100, 50, 255]);
    assert_eq!(pixel(&cut, 20, 16), BACKGROUND, "right of the clip rectangle");

    // Premultiplied alpha: half-transparent white adds half of its colour to half of the background.
    let glass = rect(white, [0.0, 0.0, 32.0, 32.0], [128, 128, 128, 128], [0.0, 0.0, 32.0, 32.0]);
    let blended = draw(&gpu, &mut pass, UiLayer { pixels_per_point: 1.0, meshes: vec![glass] });
    let [r, g, b, _] = pixel(&blended, 16, 16);
    assert!(r.abs_diff(133) <= 2 && g.abs_diff(138) <= 2 && b.abs_diff(143) <= 2, "{r} {g} {b}");

    // A textured quad samples its texture: the left half is red, the right half blue.
    let textured = rect(checker, [0.0, 0.0, 32.0, 32.0], [255; 4], [0.0, 0.0, 32.0, 32.0]);
    let sampled = draw(&gpu, &mut pass, UiLayer { pixels_per_point: 1.0, meshes: vec![textured] });
    assert!(pixel(&sampled, 4, 16)[0] > 200 && pixel(&sampled, 4, 16)[2] < 50);
    assert!(pixel(&sampled, 28, 16)[2] > 200 && pixel(&sampled, 28, 16)[0] < 50);

    // A mesh that names a freed or unknown texture is skipped.
    pass.free_texture(white);
    let missing = rect(white, [0.0, 0.0, 32.0, 32.0], [255; 4], [0.0, 0.0, 32.0, 32.0]);
    let skipped = draw(&gpu, &mut pass, UiLayer { pixels_per_point: 1.0, meshes: vec![missing] });
    assert_eq!(pixel(&skipped, 16, 16), BACKGROUND);
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn bad_patches_are_refused_and_change_nothing() {
    let _lock = serial();
    let gpu = Gpu::new(wgpu::Backends::VULKAN);
    let mut pass = UiPass::new(&gpu.device);
    let id = UiTextureId::from_raw(5);
    let patch = |offset, size: [u32; 2], rgba: &'static [u8]| UiTexturePatch { id, offset, size, rgba };
    let refused = pass.update_texture(&gpu.device, &gpu.queue, &patch(None, [2, 2], &[0; 4]));
    assert!(matches!(refused, Err(crate::UiTextureError::BadSize { id: 5, bytes: 4, size: [2, 2] })));
    let missing = pass.update_texture(&gpu.device, &gpu.queue, &patch(Some([0, 0]), [1, 1], &[0; 4]));
    assert!(matches!(missing, Err(crate::UiTextureError::RegionDoesNotFit { id: 5 })), "no texture to patch");
    pass.update_texture(&gpu.device, &gpu.queue, &patch(None, [2, 2], &[0; 16])).unwrap();
    let outside = pass.update_texture(&gpu.device, &gpu.queue, &patch(Some([1, 1]), [2, 2], &[0; 16]));
    assert!(matches!(outside, Err(crate::UiTextureError::RegionDoesNotFit { id: 5 })));
}
