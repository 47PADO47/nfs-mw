//! What the Bevy renderer can run. PR 9 drew the world path (glossy shading, redirects, the effect and UI
//! layers); PR 10 adds bloom, tone mapping, FXAA, SMAA, TAA and the FSR 1 pass. Temporal upscalers and ray
//! tracing still arrive with later PRs.

use blackbox_gfx::{
    AaSet, Antialiasing, Capabilities, GraphicsApi, MIN_RENDER_SCALE, RestartSet, RtSupport, Setting, Tonemap,
    TonemapSet, Upscaler, UpscalerSet,
};

/// The capabilities on `api`; `compressed_bc` comes from the device.
pub fn capabilities(api: GraphicsApi, compressed_bc: bool) -> Capabilities {
    Capabilities {
        renderer: "bevy",
        api,
        compressed_bc,
        // Bloom and tone mapping need an `Hdr` camera (`PostSettings::needs_hdr`); set only when either runs.
        hdr_targets: true,
        antialiasing: AaSet::of(&[Antialiasing::Off, Antialiasing::Fxaa, Antialiasing::Smaa, Antialiasing::Taa]),
        upscalers: UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear, Upscaler::Fsr1]),
        tonemaps: TonemapSet::of(&[Tonemap::Off, Tonemap::Aces]),
        bloom: true,
        ray_tracing: RtSupport::None,
        // Rendering above the surface size is not wired up.
        render_scale: (MIN_RENDER_SCALE, 1.0),
        // Ray tracing is chosen when the device is created (PR 12); nothing else needs a restart.
        restart_required: RestartSet::of(&[Setting::RayTracing]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_gfx::{GraphicsSettings, PostSettings, RayTracing, resolve};

    #[test]
    fn pr10_offers_every_post_effect_but_no_ray_tracing_or_temporal_upscaler() {
        let caps = capabilities(GraphicsApi::Vulkan, true);
        assert_eq!(caps.renderer, "bevy");
        assert_eq!(
            caps.antialiasing,
            AaSet::of(&[Antialiasing::Off, Antialiasing::Fxaa, Antialiasing::Smaa, Antialiasing::Taa])
        );
        assert_eq!(caps.upscalers, UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear, Upscaler::Fsr1]));
        assert_eq!(caps.tonemaps, TonemapSet::of(&[Tonemap::Off, Tonemap::Aces]));
        assert!(caps.bloom && caps.hdr_targets);
        assert!(!caps.ray_tracing.is_available());
        assert_eq!(caps.restart_required, RestartSet::of(&[Setting::RayTracing]));
        assert!(caps.compressed_bc && !capabilities(GraphicsApi::Vulkan, false).compressed_bc);
    }

    #[test]
    fn only_ray_tracing_and_temporal_upscalers_are_missing() {
        let caps = capabilities(GraphicsApi::Vulkan, true);
        let wanted = GraphicsSettings {
            post: PostSettings { tonemap: Tonemap::Aces, antialiasing: Antialiasing::Taa, ..PostSettings::default() },
            upscaler: Upscaler::Fsr3,
            ray_tracing: RayTracing::Low,
            render_scale: 2.0,
            ..GraphicsSettings::default()
        };
        let resolved = resolve(&wanted, &caps);
        // FSR 3 is not offered yet, so it falls back along the chain to FSR 1, the next best spatial
        // upscaler this renderer now has; TAA and ACES are both offered too and go straight through.
        assert_eq!(resolved.effective.upscaler, Upscaler::Fsr1);
        assert_eq!(resolved.effective.post.tonemap, Tonemap::Aces);
        assert_eq!(resolved.effective.post.antialiasing, Antialiasing::Taa);
        assert_eq!(resolved.effective.ray_tracing, RayTracing::Off);
        assert!(resolved.effective.render_scale <= 1.0);
        assert!(resolved.downgrade_of(Setting::Upscaler).unwrap().reason.contains("bevy"));
        assert!(resolved.downgrade_of(Setting::RayTracing).unwrap().reason.contains("bevy"));
    }
}
