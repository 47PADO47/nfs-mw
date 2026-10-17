//! What the native renderer can run, including optional experimental depth-based AO.

use blackbox_gfx::{
    AaSet, Antialiasing, Capabilities, GraphicsApi, RestartSet, RtSupport, TonemapSet, Upscaler, UpscalerSet,
};

/// The native renderer's capabilities on `api`.
///
/// `compressed_bc` and `hdr_targets` come from the adapter, and so does `temporal`: whether it can run the FSR 3
/// upscaler, the one temporal method (see `gpu::init`). Nothing needs a restart: every option the renderer has is
/// applied to the live chain. There is no ray tracing.
pub(crate) fn native_capabilities(
    api: GraphicsApi,
    compressed_bc: bool,
    hdr_targets: bool,
    ambient_occlusion: bool,
    temporal: bool,
) -> Capabilities {
    let mut upscalers = UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear, Upscaler::Fsr1]);
    if temporal {
        upscalers.insert(Upscaler::Fsr3);
    }
    Capabilities {
        renderer: "blackbox",
        api,
        compressed_bc,
        max_texture_anisotropy: 16,
        hdr_targets,
        antialiasing: match api {
            GraphicsApi::Gl => AaSet::of(&[Antialiasing::Off, Antialiasing::Fxaa]),
            _ => AaSet::of(&[Antialiasing::Off, Antialiasing::Fxaa, Antialiasing::Smaa]),
        },
        upscalers,
        tonemaps: TonemapSet::all(),
        bloom: true,
        scene_treatment: true,
        radial_blur: true,
        frame_generation: false,
        local_reflections: true,
        projected_reflections: api != GraphicsApi::Gl,
        ambient_occlusion,
        soft_shadows: false,
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
        native_capabilities(GraphicsApi::Vulkan, true, true, true, false)
    }

    /// The same renderer on a device that can run the FSR 3 upscaler.
    fn temporal_caps() -> Capabilities {
        native_capabilities(GraphicsApi::Vulkan, true, true, true, true)
    }

    #[test]
    fn the_feature_set_is_exactly_the_frozen_one() {
        let caps = caps();
        assert_eq!(caps.renderer, "blackbox");
        assert_eq!(caps.antialiasing, AaSet::of(&[Antialiasing::Off, Antialiasing::Fxaa, Antialiasing::Smaa]));
        assert_eq!(caps.upscalers, UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear, Upscaler::Fsr1]));
        assert_eq!(caps.tonemaps, TonemapSet::all());
        assert!(caps.bloom);
        assert_eq!(caps.ray_tracing, RtSupport::None);
        assert!(caps.restart_required.is_empty());
        assert_eq!(caps.render_scale, Capabilities::full_render_scale());
    }

    #[test]
    fn the_adapter_decides_compressed_textures_and_hdr() {
        let weak = native_capabilities(GraphicsApi::Gl, false, false, false, false);
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
    fn fsr3_is_offered_only_where_the_device_can_run_it() {
        assert!(temporal_caps().upscalers.contains(Upscaler::Fsr3));
        assert!(!caps().upscalers.contains(Upscaler::Fsr3));
        let wanted = GraphicsSettings {
            post: PostSettings { antialiasing: Antialiasing::Fxaa, ..PostSettings::default() },
            upscaler: Upscaler::Fsr3,
            upscale_quality: UpscaleQuality::Performance,
            ..GraphicsSettings::default()
        };
        let resolved = resolve(&wanted, &temporal_caps());
        assert_eq!(resolved.effective.upscaler, Upscaler::Fsr3);
        assert_eq!(resolved.effective.render_scale, UpscaleQuality::Performance.render_scale());
        assert_eq!(resolved.effective.post.antialiasing, Antialiasing::Off, "FSR 3 does its own anti-aliasing");
        assert!(resolved.downgrade_of(Setting::Upscaler).is_none(), "{:?}", resolved.downgrades);
        assert!(resolved.downgrade_of(Setting::Antialiasing).is_some());
    }

    #[test]
    fn dlss_falls_back_to_fsr3_where_the_device_offers_it() {
        let wanted = GraphicsSettings { upscaler: Upscaler::Dlss, ..GraphicsSettings::default() };
        let resolved = resolve(&wanted, &temporal_caps());
        assert_eq!(resolved.effective.upscaler, Upscaler::Fsr3, "dlss > fsr3 > fsr1");
        assert_eq!(resolve(&wanted, &caps()).effective.upscaler, Upscaler::Fsr1);
    }

    #[test]
    fn spatial_smaa_is_honoured() {
        let wanted = GraphicsSettings {
            post: PostSettings { antialiasing: Antialiasing::Smaa, ..PostSettings::default() },
            ..GraphicsSettings::default()
        };
        let resolved = resolve(&wanted, &caps());
        assert!(resolved.is_exact());
        assert_eq!(resolved.effective.post.antialiasing, Antialiasing::Smaa);
    }
}
