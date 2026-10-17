//! `Settings::graphics()`: from the settings to the renderer's request, and what the renderers make of it.

use blackbox_gfx::{Antialiasing, GraphicsSettings, Setting, Upscaler, resolve};

use super::*;

fn settings(scale: u16, mode: UpscaleMode) -> Settings {
    Settings::from(Partial { render_scale: RenderScale::new(scale), upscaler: Some(mode), ..Partial::default() })
}

#[test]
fn the_defaults_ask_for_nothing() {
    let asked = Settings::from(Partial::default()).graphics();
    assert_eq!((asked.upscaler, asked.render_scale, asked.upscale_sharpness), (Upscaler::Fsr1, 1.0, 0.8));
    assert!(asked.post.effects().is_empty());
    let native = resolve(&asked, &test_caps::native());
    assert!(native.is_exact(), "{:?}", native.downgrades);
    assert_eq!(native.effective, asked);
}

#[test]
fn shadow_request_survives_layering_and_backend_downgrades() {
    let stored = Settings::from(Partial { shadows: Some(ShadowQuality::High), ..Default::default() });
    assert_eq!(stored.graphics().shadows, ShadowQuality::High);
    assert_eq!(stored.effective(&test_caps::native()).shadows, ShadowQuality::Off);
    assert_eq!(stored.effective(&test_caps::full()).shadows, ShadowQuality::High);
    assert!(availability::check("shadows", &stored, &test_caps::native()).is_err());
    assert!(availability::check("shadows", &stored, &test_caps::full()).is_ok());
    assert_eq!(stored.shadows, ShadowQuality::High);
}

#[test]
fn the_graphics_settings_carry_the_post_effects_and_the_upscaler() {
    let asked = settings(67, UpscaleMode::Fsr1).graphics();
    assert_eq!((asked.upscaler, asked.render_scale), (Upscaler::Fsr1, 0.67));
    assert!(asked.post.effects().is_empty());
    assert_eq!(settings(150, UpscaleMode::Bilinear).graphics().render_scale, 1.5);
}

#[test]
fn off_draws_at_the_output_size_whatever_the_render_scale() {
    let asked = settings(50, UpscaleMode::Off).graphics();
    assert_eq!((asked.upscaler, asked.render_scale), (Upscaler::Off, 0.5), "the request says what was set");
    let resolved = resolve(&asked, &test_caps::native());
    assert_eq!((resolved.effective.upscaler, resolved.effective.render_scale), (Upscaler::Off, 1.0));
    assert_eq!(resolved.downgrades.len(), 1, "{:?}", resolved.downgrades);
}

fn with_aa(aa: PostAa) -> GraphicsSettings {
    Settings::from(Partial { post_aa: Some(aa), ..Partial::default() }).graphics()
}

#[test]
fn native_smaa_is_spatial_and_taa_still_falls_back() {
    let smaa = resolve(&with_aa(PostAa::Smaa), &test_caps::native());
    assert!(smaa.is_exact());
    assert_eq!(smaa.effective.post.antialiasing, Antialiasing::Smaa);
    let asked = with_aa(PostAa::Taa);
    assert_eq!(asked.post.antialiasing, Antialiasing::Taa);
    let native = resolve(&asked, &test_caps::native());
    assert_eq!(native.effective.post.antialiasing, Antialiasing::Fxaa);
    let downgrade = native.downgrade_of(Setting::Antialiasing).expect("a logged downgrade");
    assert_eq!((downgrade.requested.as_str(), downgrade.effective.as_str()), ("taa", "fxaa"));
    assert!(resolve(&asked, &test_caps::full()).is_exact());
}

#[test]
fn dlss_at_quality_on_the_native_renderer_is_fsr1_at_two_thirds_with_a_downgrade() {
    let asked = settings(100, UpscaleMode::Dlss).graphics();
    assert_eq!((asked.upscaler, asked.render_scale), (Upscaler::Dlss, 1.0));
    let native = resolve(&asked, &test_caps::native());
    assert_eq!(native.effective.upscaler, Upscaler::Fsr1);
    assert!((native.effective.render_scale - 0.67).abs() < 0.005, "{}", native.effective.render_scale);
    let downgrade = native.downgrade_of(Setting::Upscaler).expect("a logged downgrade");
    assert_eq!((downgrade.requested.as_str(), downgrade.effective.as_str()), ("dlss", "fsr1"));
    assert!(downgrade.reason.contains("blackbox"), "{}", downgrade.reason);
    let full = resolve(&asked, &test_caps::full());
    assert!(full.is_exact(), "{:?}", full.downgrades);
    assert_eq!(full.effective.upscaler, Upscaler::Dlss);
}

#[test]
fn fsr3_and_fsr4_fall_back_along_the_chain_on_a_renderer_that_lacks_them() {
    for mode in [UpscaleMode::Fsr3, UpscaleMode::Fsr4] {
        let native = resolve(&settings(100, mode).graphics(), &test_caps::native());
        assert_eq!(native.effective.upscaler, Upscaler::Fsr1, "{mode}");
    }
}

#[test]
fn fsr3_on_a_native_renderer_that_offers_it_is_kept_and_sized_by_its_quality_mode() {
    let asked =
        Settings { upscale_quality: UpscaleQuality::Performance, ..settings(100, UpscaleMode::Fsr3) }.graphics();
    let native = resolve(&asked, &test_caps::native_fsr3());
    assert_eq!(native.effective.upscaler, Upscaler::Fsr3);
    assert_eq!(native.effective.render_scale, 0.5, "the quality mode sets the render size");
    assert!(native.downgrade_of(Setting::Upscaler).is_none(), "{:?}", native.downgrades);
    let lacking = resolve(&asked, &test_caps::native());
    assert_eq!(lacking.effective.upscaler, Upscaler::Fsr1, "without the device limits it is FSR 1 at the same size");
    assert_eq!(lacking.effective.render_scale, 0.5);
}

#[test]
fn the_graphics_request_carries_ao_quality_intensity_and_comparison() {
    let stored = Settings::from(Partial {
        ao: Some(AoQuality::High),
        ao_intensity: Some(Percent(60)),
        ao_compare: Some(true),
        ..Partial::default()
    });
    let asked = stored.graphics();
    assert_eq!(asked.ao.quality, AoQuality::High);
    assert!((asked.ao.intensity - 0.6).abs() < f32::EPSILON);
    assert!(asked.ao.compare);
    assert_eq!(resolve(&asked, &test_caps::native()).effective.ao, asked.ao);
    let unsupported = blackbox_gfx::Capabilities { ambient_occlusion: false, ..test_caps::native() };
    let run = resolve(&asked, &unsupported);
    assert_eq!(run.effective.ao.quality, AoQuality::Off);
    assert!(run.downgrade_of(Setting::AmbientOcclusion).is_some());
    assert_eq!(stored.effective(&unsupported).ao, AoQuality::Off);
    assert_eq!(stored.ao, AoQuality::High, "a downgrade preserves the stored request");
}
