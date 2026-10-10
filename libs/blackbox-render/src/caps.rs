//! What the native renderer can run. Its feature set is frozen: FXAA, bloom, ACES tone mapping, a render
//! scale with bilinear upscaling, and FSR 1.

use blackbox_gfx::{
    AaSet, Antialiasing, Capabilities, GraphicsApi, RestartSet, RtSupport, TonemapSet, Upscaler, UpscalerSet,
};

/// The native renderer's capabilities on `api`.
///
/// `compressed_bc` and `hdr_targets` come from the adapter. Nothing needs a restart: every option the
/// renderer has is applied to the live chain. There is no ray tracing and no temporal method.
pub(crate) fn native_capabilities(api: GraphicsApi, compressed_bc: bool, hdr_targets: bool) -> Capabilities {
    Capabilities {
        renderer: "blackbox",
        api,
        compressed_bc,
        hdr_targets,
        antialiasing: AaSet::of(&[Antialiasing::Off, Antialiasing::Fxaa]),
        upscalers: UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear, Upscaler::Fsr1]),
        tonemaps: TonemapSet::all(),
        bloom: true,
        ray_tracing: RtSupport::None,
        render_scale: Capabilities::full_render_scale(),
        restart_required: RestartSet::empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_gfx::{GraphicsSettings, PostSettings, RayTracing, Setting, Tonemap, UpscaleQuality, resolve};

    fn caps() -> Capabilities {
        native_capabilities(GraphicsApi::Vulkan, true, true)
    }

    #[test]
    fn the_feature_set_is_exactly_the_frozen_one() {
        let caps = caps();
        assert_eq!(caps.renderer, "blackbox");
        assert_eq!(caps.antialiasing, AaSet::of(&[Antialiasing::Off, Antialiasing::Fxaa]));
        assert_eq!(caps.upscalers, UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear, Upscaler::Fsr1]));
        assert_eq!(caps.tonemaps, TonemapSet::all());
        assert!(caps.bloom);
        assert_eq!(caps.ray_tracing, RtSupport::None);
        assert!(caps.restart_required.is_empty());
        assert_eq!(caps.render_scale, Capabilities::full_render_scale());
    }

    #[test]
    fn the_adapter_decides_compressed_textures_and_hdr() {
        let weak = native_capabilities(GraphicsApi::Gl, false, false);
        assert!(!weak.compressed_bc && !weak.hdr_targets);
        assert_eq!(weak.api, GraphicsApi::Gl);
        assert!(caps().compressed_bc && caps().hdr_targets);
    }

    #[test]
    fn a_request_the_renderer_can_run_is_honoured() {
        let wanted = GraphicsSettings {
            post: PostSettings {
                tonemap: Tonemap::Aces,
                bloom_intensity: 0.5,
                antialiasing: Antialiasing::Fxaa,
                ..PostSettings::default()
            },
            upscaler: Upscaler::Fsr1,
            render_scale: 0.67,
            ..GraphicsSettings::default()
        };
        let resolved = resolve(&wanted, &caps());
        assert!(resolved.is_exact(), "{:?}", resolved.downgrades);
        assert_eq!(resolved.effective, wanted);
    }

    #[test]
    fn what_the_renderer_lacks_falls_back_with_a_reason() {
        let wanted = GraphicsSettings {
            post: PostSettings { antialiasing: Antialiasing::Taa, ..PostSettings::default() },
            upscaler: Upscaler::Dlss,
            ray_tracing: RayTracing::High,
            ..GraphicsSettings::default()
        };
        let resolved = resolve(&wanted, &caps());
        assert_eq!(resolved.effective.upscaler, Upscaler::Fsr1, "dlss > fsr3 > fsr1");
        assert_eq!(resolved.effective.ray_tracing, RayTracing::Off);
        assert_eq!(
            resolved.effective.post.antialiasing,
            Antialiasing::Fxaa,
            "dlss fell back to a spatial upscaler, so TAA falls back to FXAA"
        );
        for setting in [Setting::Upscaler, Setting::RayTracing, Setting::Antialiasing] {
            assert!(resolved.downgrade_of(setting).is_some(), "{setting}");
        }
        assert!(resolved.downgrade_of(Setting::Upscaler).unwrap().reason.contains("blackbox"));
    }

    #[test]
    fn a_temporal_quality_mode_does_not_drive_the_native_render_scale() {
        let wanted = GraphicsSettings {
            upscaler: Upscaler::Fsr3,
            upscale_quality: UpscaleQuality::Performance,
            ..GraphicsSettings::default()
        };
        let resolved = resolve(&wanted, &caps());
        assert_eq!(resolved.effective.upscaler, Upscaler::Fsr1);
        assert_eq!(resolved.effective.render_scale, UpscaleQuality::Performance.render_scale());
    }

    #[test]
    fn smaa_falls_back_to_fxaa() {
        let wanted = GraphicsSettings {
            post: PostSettings { antialiasing: Antialiasing::Smaa, ..PostSettings::default() },
            ..GraphicsSettings::default()
        };
        assert_eq!(resolve(&wanted, &caps()).effective.post.antialiasing, Antialiasing::Fxaa);
    }
}
