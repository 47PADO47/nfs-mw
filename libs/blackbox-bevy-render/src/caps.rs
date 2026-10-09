//! What the Bevy renderer can run. The spike draws the world path only: FXAA and a bilinear render scale are the
//! only post features; tone mapping, bloom, temporal methods and ray tracing arrive with later PRs.

use blackbox_gfx::{
    AaSet, Antialiasing, Capabilities, GraphicsApi, MIN_RENDER_SCALE, RestartSet, RtSupport, TonemapSet, Upscaler,
    UpscalerSet,
};

/// The capabilities on `api`; `compressed_bc` comes from the device.
pub fn capabilities(api: GraphicsApi, compressed_bc: bool) -> Capabilities {
    Capabilities {
        renderer: "bevy",
        api,
        compressed_bc,
        // The scene is drawn straight into an 8-bit sRGB target for now.
        hdr_targets: false,
        antialiasing: AaSet::of(&[Antialiasing::Off, Antialiasing::Fxaa]),
        upscalers: UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear]),
        tonemaps: TonemapSet::of(&[blackbox_gfx::Tonemap::Off]),
        bloom: false,
        ray_tracing: RtSupport::None,
        // Rendering above the surface size is not wired up.
        render_scale: (MIN_RENDER_SCALE, 1.0),
        restart_required: RestartSet::empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_gfx::{GraphicsSettings, PostSettings, RayTracing, Setting, Tonemap, resolve};

    #[test]
    fn the_spike_offers_fxaa_and_bilinear_and_no_ray_tracing() {
        let caps = capabilities(GraphicsApi::Vulkan, true);
        assert_eq!(caps.renderer, "bevy");
        assert_eq!(caps.antialiasing, AaSet::of(&[Antialiasing::Off, Antialiasing::Fxaa]));
        assert_eq!(caps.upscalers, UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear]));
        assert!(!caps.bloom && !caps.hdr_targets && !caps.ray_tracing.is_available());
        assert!(caps.compressed_bc && !capabilities(GraphicsApi::Vulkan, false).compressed_bc);
    }

    #[test]
    fn what_is_missing_falls_back_with_a_reason_that_names_the_renderer() {
        let caps = capabilities(GraphicsApi::Vulkan, true);
        let wanted = GraphicsSettings {
            post: PostSettings { tonemap: Tonemap::Aces, antialiasing: Antialiasing::Taa, ..PostSettings::default() },
            upscaler: Upscaler::Fsr1,
            ray_tracing: RayTracing::Low,
            render_scale: 2.0,
            ..GraphicsSettings::default()
        };
        let resolved = resolve(&wanted, &caps);
        assert_eq!(resolved.effective.upscaler, Upscaler::Bilinear);
        assert_eq!(resolved.effective.post.tonemap, Tonemap::Off);
        assert_eq!(resolved.effective.post.antialiasing, Antialiasing::Fxaa);
        assert_eq!(resolved.effective.ray_tracing, RayTracing::Off);
        assert!(resolved.effective.render_scale <= 1.0);
        assert!(resolved.downgrade_of(Setting::Upscaler).unwrap().reason.contains("bevy"));
    }
}
