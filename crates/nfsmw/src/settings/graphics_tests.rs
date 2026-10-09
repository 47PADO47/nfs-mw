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
fn smaa_and_taa_are_requests_the_native_renderer_runs_as_fxaa() {
    for (aa, wanted) in [(PostAa::Smaa, Antialiasing::Smaa), (PostAa::Taa, Antialiasing::Taa)] {
        let asked = with_aa(aa);
        assert_eq!(asked.post.antialiasing, wanted);
        let native = resolve(&asked, &test_caps::native());
        assert_eq!(native.effective.post.antialiasing, Antialiasing::Fxaa);
        let downgrade = native.downgrade_of(Setting::Antialiasing).expect("a logged downgrade");
        assert_eq!((downgrade.requested.as_str(), downgrade.effective.as_str()), (wanted.name(), "fxaa"));
        assert!(resolve(&asked, &test_caps::full()).is_exact(), "a full renderer runs {aa}");
    }
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
