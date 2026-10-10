//! Render scale and upscaler: from the settings to the renderer (docs/upscaling.md).

use blackbox_render::{Renderer, Upscaler, suggested_texture_lod_bias};

use crate::settings::Settings;

/// The per-axis render scale the renderer is given: 1.0 while the upscaler is off, otherwise the setting.
pub fn effective_scale(settings: &Settings) -> f32 {
    match settings.upscaler.upscaler() {
        Some(_) => settings.render_scale.factor(),
        None => 1.0,
    }
}

/// Whether the settings change what the renderer draws or how it upscales.
pub fn differs(a: &Settings, b: &Settings) -> bool {
    (a.render_scale, a.upscaler, a.upscale_sharpness) != (b.render_scale, b.upscaler, b.upscale_sharpness)
}

/// Give the renderer the render scale, upscaler and sharpness of `settings`, and sharpen the world's textures
/// to match the lower resolution.
pub fn apply(renderer: &mut Renderer, settings: &Settings) {
    let scale = effective_scale(settings);
    renderer.set_upscaler(settings.upscaler.upscaler().unwrap_or(Upscaler::Bilinear));
    renderer.set_upscale_sharpness(settings.upscale_sharpness.amplitude());
    renderer.set_render_scale(scale);
    renderer.set_texture_lod_bias(suggested_texture_lod_bias(scale));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Partial, RenderScale, UpscaleMode};

    fn settings(scale: u16, mode: UpscaleMode) -> Settings {
        Settings::from(Partial { render_scale: RenderScale::new(scale), upscaler: Some(mode), ..Partial::default() })
    }

    #[test]
    fn off_ignores_the_render_scale() {
        assert_eq!(effective_scale(&settings(50, UpscaleMode::Off)), 1.0);
        assert_eq!(effective_scale(&settings(50, UpscaleMode::Fsr1)), 0.5);
        assert_eq!(effective_scale(&settings(150, UpscaleMode::Bilinear)), 1.5);
        assert_eq!(effective_scale(&Settings::from(Partial::default())), 1.0);
    }

    #[test]
    fn only_the_upscaling_settings_count_as_a_difference() {
        let base = settings(67, UpscaleMode::Fsr1);
        assert!(!differs(&base, &base));
        assert!(differs(&base, &settings(77, UpscaleMode::Fsr1)));
        assert!(differs(&base, &settings(67, UpscaleMode::Bilinear)));
        let sharper = Settings { upscale_sharpness: crate::settings::Percent(10), ..base };
        assert!(differs(&base, &sharper));
        let louder = Settings { master_volume: crate::settings::Percent(1), ..base };
        assert!(!differs(&base, &louder));
    }
}
