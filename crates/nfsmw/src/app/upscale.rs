//! Render scale and upscaler: from the settings to the renderer (docs/upscaling.md).

use blackbox_gfx::{GraphicsSettings, RenderBackend, Upscaler};

use crate::settings::{Settings, post_effects};

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

/// What the settings ask of the renderer: the post effects, the upscaler with its render scale and sharpness.
pub fn graphics(settings: &Settings) -> GraphicsSettings {
    GraphicsSettings {
        post: post_effects(settings),
        upscaler: settings.upscaler.upscaler().unwrap_or(Upscaler::Bilinear),
        render_scale: effective_scale(settings),
        upscale_sharpness: settings.upscale_sharpness.amplitude(),
        ..GraphicsSettings::default()
    }
}

/// Give the renderer the graphics settings. The renderer maps them onto what it can run, and sharpens the
/// world's textures to match a lower resolution.
pub fn apply(renderer: &mut dyn RenderBackend, settings: &Settings) {
    renderer.apply_graphics(&graphics(settings));
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
    fn the_graphics_settings_carry_the_post_effects_and_the_upscaler() {
        let asked = graphics(&settings(67, UpscaleMode::Fsr1));
        assert_eq!((asked.upscaler, asked.render_scale), (Upscaler::Fsr1, 0.67));
        assert!(asked.post.effects().is_empty());
        let off = graphics(&settings(67, UpscaleMode::Off));
        assert_eq!((off.upscaler, off.render_scale), (Upscaler::Bilinear, 1.0), "off draws at the output size");
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
