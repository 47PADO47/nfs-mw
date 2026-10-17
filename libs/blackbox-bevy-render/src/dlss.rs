//! DLSS Super Resolution on the Bevy backend (PR 11, `feat/gfx-bevy-dlss`). Two builds, one caps and apply path:
//!
//! - `dlss` without `dlss-mock`: Bevy's own `Dlss` camera component, through [`sdk`]. Needs the NVIDIA DLSS SDK at
//!   build time (`DLSS_SDK`, `VULKAN_SDK`, clang) and an RTX GPU under Vulkan. Offered only when Bevy reports
//!   `DlssSuperResolutionSupported` on the machine. Not built in the development environment (no SDK there).
//! - `dlss-mock`: no Bevy DLSS types ([`mock`]). A DLSS request runs as TAA plus the FSR 1 pass at the same render
//!   scale, so the caps, the settings and the apply path can be exercised without an RTX GPU. Always offered.
//!
//! Both use the one render-scale table, `blackbox_gfx::UpscaleQuality::render_scale`, which the resolver applies
//! to the request before the backend ever sees it.

#[cfg(all(feature = "dlss", not(feature = "dlss-mock")))]
pub(crate) mod sdk;

#[cfg(feature = "dlss-mock")]
pub(crate) mod mock;

use blackbox_gfx::{Upscaler, UpscalerSet};

/// Offer DLSS in the caps: on the mock always, on the SDK build only when Bevy found DLSS Super Resolution.
#[cfg(feature = "dlss-mock")]
pub fn offer(upscalers: &mut UpscalerSet, _supported: bool) {
    upscalers.insert(Upscaler::Dlss);
}

/// Offer DLSS in the caps: on the mock always, on the SDK build only when Bevy found DLSS Super Resolution.
#[cfg(all(feature = "dlss", not(feature = "dlss-mock")))]
pub fn offer(upscalers: &mut UpscalerSet, supported: bool) {
    if !supported {
        return;
    }
    upscalers.insert(Upscaler::Dlss);
}

#[cfg(test)]
mod tests {
    use blackbox_gfx::{GraphicsApi, GraphicsSettings, UpscaleQuality, Upscaler, resolve};

    use crate::caps::capabilities_with_dlss;

    fn dlss_request(quality: UpscaleQuality) -> GraphicsSettings {
        GraphicsSettings { upscaler: Upscaler::Dlss, upscale_quality: quality, ..GraphicsSettings::default() }
    }

    #[test]
    fn quality_modes_render_at_the_shared_table_scales() {
        // The table is `UpscaleQuality::render_scale` (per-axis 1 / ratio, AMD's FSR ratios). NVIDIA's own
        // Balanced scale is 0.58 against 0.588 here; see the report on the Bevy DLSS PR.
        let scales = UpscaleQuality::ALL.map(UpscaleQuality::render_scale);
        let expected = [1.0 / 1.5, 1.0, 1.0 / 1.5, 1.0 / 1.7, 0.5, 1.0 / 3.0];
        for (got, want) in scales.iter().zip(expected) {
            assert!((got - want).abs() < 1e-6, "{got} != {want}");
        }
        assert_eq!(UpscaleQuality::Quality.render_scale(), UpscaleQuality::Auto.render_scale());
    }

    #[test]
    fn a_dlss_request_renders_at_the_table_scale_for_every_quality_mode() {
        let caps = capabilities_with_dlss(GraphicsApi::Vulkan, true, true);
        for quality in UpscaleQuality::ALL {
            let resolved = resolve(&dlss_request(quality), &caps);
            assert_eq!(resolved.effective.upscaler, Upscaler::Dlss, "{quality:?}");
            assert_eq!(resolved.effective.render_scale, quality.render_scale(), "{quality:?}");
        }
    }

    #[test]
    fn dlss_is_offered_on_the_bevy_caps_when_bevy_supports_it() {
        let caps = capabilities_with_dlss(GraphicsApi::Vulkan, true, true);
        assert!(caps.upscalers.contains(Upscaler::Dlss));
    }

    #[cfg(all(feature = "dlss", not(feature = "dlss-mock")))]
    #[test]
    fn the_sdk_build_offers_dlss_only_when_bevy_found_it() {
        let without = capabilities_with_dlss(GraphicsApi::Vulkan, true, false);
        assert!(!without.upscalers.contains(Upscaler::Dlss));
        let with = capabilities_with_dlss(GraphicsApi::Vulkan, true, true);
        assert!(with.upscalers.contains(Upscaler::Dlss));
    }
}
