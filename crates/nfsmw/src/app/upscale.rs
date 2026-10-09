//! Render scale and upscaler: from the settings to the renderer (docs/upscaling.md).

use blackbox_gfx::RenderBackend;

use crate::settings::Settings;

/// Whether the settings change what the renderer draws or how it upscales.
pub fn differs(a: &Settings, b: &Settings) -> bool {
    (a.render_scale, a.upscaler, a.upscale_sharpness) != (b.render_scale, b.upscaler, b.upscale_sharpness)
}

/// Give the renderer the graphics settings. The renderer maps them onto what it can run, and sharpens the
/// world's textures to match a lower resolution.
pub fn apply(renderer: &mut dyn RenderBackend, settings: &Settings) {
    renderer.apply_graphics(&settings.graphics());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Partial, RenderScale, UpscaleMode};

    fn settings(scale: u16, mode: UpscaleMode) -> Settings {
        Settings::from(Partial { render_scale: RenderScale::new(scale), upscaler: Some(mode), ..Partial::default() })
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
