//! Pixel checks for the actual UI recomposition shader, including additive HUD elements.
use blackbox_gpu_passes::test_support::{Gpu, serial};

#[test]
#[ignore = "requires a Vulkan adapter"]
fn separate_ui_preserves_scene_alpha_and_additive_composition() {
    let _serial = serial();
    let gpu = Gpu::new(wgpu::Backends::VULKAN);
    let scene = gpu.target(2);
    let raw = gpu.target(2);
    let output = gpu.target(2);
    let ui = gpu.target(2);
    for (texture, pixels) in
        [(&scene, [32, 64, 96, 255].repeat(4)), (&raw, vec![0, 0, 0, 0, 128, 0, 0, 255, 0, 32, 0, 128, 16, 0, 0, 0])]
    {
        gpu.queue.write_texture(
            texture.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(8), rows_per_image: Some(2) },
            texture.size(),
        );
    }
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    super::composition::Composite::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm).encode(
        &gpu.device,
        &mut encoder,
        &scene.create_view(&Default::default()),
        &raw.create_view(&Default::default()),
        &output.create_view(&Default::default()),
        &ui.create_view(&Default::default()),
    );
    let final_pixels = gpu.finish(encoder, &output);
    let encoder = gpu.device.create_command_encoder(&Default::default());
    let ui_pixels = gpu.finish(encoder, &ui);
    assert_eq!(&ui_pixels[..4], &[0, 0, 0, 0], "no HUD coverage outside UI");
    assert_eq!(&final_pixels[..4], &[32, 64, 96, 255], "HUDless background stays unchanged");
    assert_eq!(&ui_pixels[4..8], &[128, 0, 0, 255], "opaque UI excludes the background");
    assert_eq!(&final_pixels[12..16], &[48, 64, 96, 255], "additive UI retains its real-frame blend");
    for pixel in 0..4 {
        let alpha = f32::from(ui_pixels[pixel * 4 + 3]) / 255.0;
        for channel in 0..3 {
            let background = [32.0, 64.0, 96.0][channel];
            let recomposed = f32::from(ui_pixels[pixel * 4 + channel]) + (1.0 - alpha) * background;
            assert!((recomposed - f32::from(final_pixels[pixel * 4 + channel])).abs() <= 1.1);
        }
    }
}
