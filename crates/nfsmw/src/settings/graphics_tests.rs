//! `Settings::graphics()`: from the settings to the renderer's request, and what the renderers make of it.

use blackbox_gfx::{Upscaler, resolve};

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
