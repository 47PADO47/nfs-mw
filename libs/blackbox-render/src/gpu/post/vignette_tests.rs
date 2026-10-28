//! GPU checks for the separate user vignette and frame-generation suppression.
use super::PostChain;
use super::tests::Gpu;
use blackbox_gfx::PostSettings;

#[test]
#[ignore = "requires a Vulkan GPU"]
fn vignette_is_suppressed_only_while_frame_generation_is_active() {
    let _guard = crate::gpu::test_support::serial();
    let gpu = Gpu::new(wgpu::Backends::VULKAN);
    let mut chain = PostChain::new(&gpu.device, None);
    chain.set_effects(&gpu.device, PostSettings { vignette: 0.65, ..Default::default() });
    let size = (64, 64);
    let color = wgpu::Color { r: 0.8, g: 0.8, b: 0.8, a: 1.0 };
    let off = gpu.run_with_fg(&mut chain, size, color, size, None, false);
    let on = gpu.run_with_fg(&mut chain, size, color, size, None, true);
    assert!(off[0][0] + 50 < on[0][0], "corner should darken when frame generation is off");
    assert_eq!(off[32 * 64 + 32], on[32 * 64 + 32], "centre is unchanged");
    let resumed = gpu.run_with_fg(&mut chain, size, color, size, None, false);
    assert_eq!(off, resumed, "vignette returns when frame generation stops");
}

#[test]
#[ignore = "requires a Vulkan GPU"]
fn treatment_gain_is_neutral_with_and_without_frame_generation() {
    let _guard = crate::gpu::test_support::serial();
    let gpu = Gpu::new(wgpu::Backends::VULKAN);
    let mut chain = PostChain::new(&gpu.device, None);
    let profile = blackbox_gfx::ColourTreatment {
        tint: [1.0; 3],
        luminance: [1.0 / 3.0; 3],
        desaturation: 1.0,
        black_intensity: 0.0,
        colour_intensity: 0.0,
        black_curve: [0.0; 4],
        colour_curve: [0.0; 4],
    };
    let gain = blackbox_gfx::RgbaImage::new(1, 1, vec![0, 0, 128, 255]).unwrap();
    chain.set_treatment(
        &gpu.device,
        Some(super::treatment::Profile {
            settings: profile,
            gain: super::treatment::gain_texture(&gpu.device, &gpu.queue, Some(&gain)),
            luma_format: crate::gpu::targets::HDR_FORMAT,
        }),
    );
    chain.set_effects(&gpu.device, PostSettings { colour_treatment: 1.0, ..Default::default() });
    let size = (16, 16);
    let color = wgpu::Color { r: 0.4, g: 0.4, b: 0.4, a: 1.0 };
    let normal = gpu.run_with_fg(&mut chain, size, color, size, None, false);
    let fg = gpu.run_with_fg(&mut chain, size, color, size, None, true);
    assert_eq!(normal, fg, "the legacy gain is neutral regardless of frame generation");
    assert!((normal[0][0] as i32 - 102).abs() <= 1, "the treatment uses unit gain");
    assert_eq!(normal, gpu.run_with_fg(&mut chain, size, color, size, None, false));
}
