//! The `dlss-mock` stand-in: no Bevy DLSS types. A DLSS request runs as TAA plus the FSR 1 pass, at the render scale
//! `resolve` already chose from the quality mode. The picture is not DLSS's; the log line says so once.

use std::sync::Once;

use blackbox_gfx::{Antialiasing, Upscaler};

use crate::ops::CameraSettings;

static LOGGED: Once = Once::new();

/// What a DLSS request runs as on this build. Other requests pass through unchanged.
///
/// `resolve` turns temporal anti-aliasing off whenever the upscaler is temporal (it is DLSS's own job), so the
/// stand-in turns it back on, and the temporal texture bias goes with it.
pub fn stand_in(mut settings: CameraSettings) -> CameraSettings {
    if settings.upscaler != Upscaler::Dlss {
        return settings;
    }
    LOGGED.call_once(|| log::info!("DLSS mock: using TAA + spatial upscale stand-in"));
    settings.post.antialiasing = Antialiasing::Taa;
    settings.upscaler = Upscaler::Fsr1;
    settings.mip_bias = crate::post::scale::mip_bias(Antialiasing::Taa, settings.render_scale);
    settings
}

#[cfg(test)]
mod tests {
    use blackbox_gfx::{
        Antialiasing, GraphicsApi, GraphicsSettings, UpscaleQuality, Upscaler, resolve,
        suggested_temporal_texture_lod_bias,
    };

    use super::stand_in;
    use crate::caps::capabilities_with_dlss;
    use crate::ops::CameraSettings;

    /// The settings `BevyBackend::apply_graphics` builds from a request, without the facade around it.
    fn settings_for(request: &GraphicsSettings) -> CameraSettings {
        let caps = capabilities_with_dlss(GraphicsApi::Vulkan, true, true);
        let effective = resolve(request, &caps).effective;
        CameraSettings {
            post: effective.post,
            upscaler: effective.upscaler,
            upscale_sharpness: effective.upscale_sharpness,
            render_scale: effective.render_scale,
            mip_bias: crate::post::scale::mip_bias(effective.post.antialiasing, effective.render_scale),
            ..CameraSettings::default()
        }
    }

    fn dlss_request(quality: UpscaleQuality) -> GraphicsSettings {
        GraphicsSettings { upscaler: Upscaler::Dlss, upscale_quality: quality, ..GraphicsSettings::default() }
    }

    #[test]
    fn the_mock_offers_dlss_whatever_bevy_reports() {
        assert!(capabilities_with_dlss(GraphicsApi::Vulkan, true, false).upscalers.contains(Upscaler::Dlss));
        assert!(capabilities_with_dlss(GraphicsApi::Vulkan, true, true).upscalers.contains(Upscaler::Dlss));
    }

    #[test]
    fn a_dlss_request_applies_as_taa_and_fsr1_at_the_performance_scale() {
        let settings = stand_in(settings_for(&dlss_request(UpscaleQuality::Performance)));
        assert_eq!(settings.post.antialiasing, Antialiasing::Taa);
        assert_eq!(settings.upscaler, Upscaler::Fsr1);
        assert_eq!(settings.render_scale, 0.5);
        assert_eq!(settings.mip_bias, suggested_temporal_texture_lod_bias(0.5));
    }

    #[test]
    fn every_quality_mode_keeps_its_table_scale_through_the_stand_in() {
        for quality in UpscaleQuality::ALL {
            let settings = stand_in(settings_for(&dlss_request(quality)));
            assert_eq!(settings.render_scale, quality.render_scale(), "{quality:?}");
            assert_eq!(settings.post.antialiasing, Antialiasing::Taa, "{quality:?}");
        }
    }

    #[test]
    fn other_upscalers_pass_through_unchanged() {
        let request = GraphicsSettings {
            upscaler: Upscaler::Fsr1,
            render_scale: 0.67,
            post: blackbox_gfx::PostSettings { antialiasing: Antialiasing::Taa, ..Default::default() },
            ..GraphicsSettings::default()
        };
        let before = settings_for(&request);
        assert_eq!(stand_in(before), before);
    }
}
