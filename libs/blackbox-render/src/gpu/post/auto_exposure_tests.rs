//! Fixed luminance scenes isolate the meter from artwork, grading and tone mapping.
use super::*;

#[test]
fn histogram_shader_validates() {
    validate("auto-exposure", include_str!("../../shaders/auto_exposure.wgsl"));
}

#[test]
#[ignore = "needs a Vulkan GPU"]
fn auto_exposure_preserves_daylight_rejects_outliers_and_resets_on_cut() {
    use blackbox_gfx::settings::post::AutoExposure;
    let _gpu = crate::gpu::test_support::serial();
    let gpu = Gpu::new(wgpu::Backends::VULKAN);
    let tuning = AutoExposure {
        neutral_log_luminance: [-4.0, -1.0],
        min_ev: -0.5,
        max_ev: 1.0,
        compensation_ev: 0.0,
        brighten_rate: 0.0,
        darken_rate: 0.0,
    };
    let mut chain = gpu.chain_with(PostSettings { auto_exposure: Some(tuning), ..Default::default() });
    let day = "return vec4<f32>(0.25, 0.25, 0.25, 1.0);";
    let tunnel = "let c = select(0.03125, 20.0, pos.x < 6.0); return vec4<f32>(vec3<f32>(c), 1.0);";
    let red = |pixels: Vec<[u8; 4]>| pixels[32][0];
    assert_eq!(red(gpu.run(&mut chain, (64, 64), day)), 64);
    assert_eq!(red(gpu.run(&mut chain, (64, 64), tunnel)), 8, "zero rate retains daylight EV");
    chain.camera_cut();
    assert_eq!(red(gpu.run(&mut chain, (64, 64), tunnel)), 15, "trim bright exit and snap to the dark interior");
    chain.camera_cut();
    assert_eq!(red(gpu.run(&mut chain, (64, 64), day)), 64);
}
