//! What the Bevy renderer can run. PR 9 drew the world path (glossy shading, redirects, the effect and UI
//! layers); PR 10 adds bloom, tone mapping, FXAA, SMAA, TAA and the FSR 1 pass; PR 11 adds DLSS Super Resolution
//! (`dlss.rs`, offered only where Bevy supports it, or always on `dlss-mock`); PR 12 reports ray tracing from the
//! device's features (`solari.rs`); PR 13b offers FSR 3 on an `fsr3` build when the device can run it (`fsr3.rs`).
//! FSR 4 still arrives with a later PR.

use blackbox_gfx::{
    AaSet, Antialiasing, Capabilities, GraphicsApi, MIN_RENDER_SCALE, RestartSet, RtSupport, Setting, Tonemap,
    TonemapSet, Upscaler, UpscalerSet,
};

/// The capabilities on `api` without DLSS, for tests; the factory uses [`capabilities_with_dlss`].
#[cfg(test)]
pub fn capabilities(api: GraphicsApi, compressed_bc: bool) -> Capabilities {
    capabilities_with_dlss(api, compressed_bc, false)
}

/// The capabilities on `api`. `dlss_supported` is whether Bevy found DLSS Super Resolution on this machine; the
/// `dlss-mock` build offers DLSS regardless. No ray tracing and no FSR 3. The factory uses
/// [`capabilities_with_ray_tracing`].
#[cfg(test)]
pub fn capabilities_with_dlss(api: GraphicsApi, compressed_bc: bool, dlss_supported: bool) -> Capabilities {
    capabilities_with_ray_tracing(api, compressed_bc, dlss_supported, RtSupport::None, false)
}

/// The capabilities on `api` with `ray_tracing` as the device reports it (see `solari::support`), and FSR 3 offered
/// when `fsr3_supported` (`fsr3::support`, only ever true in an `fsr3` build).
pub fn capabilities_with_ray_tracing(
    api: GraphicsApi,
    compressed_bc: bool,
    dlss_supported: bool,
    ray_tracing: RtSupport,
    fsr3_supported: bool,
) -> Capabilities {
    let mut upscalers = UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear, Upscaler::Fsr1]);
    offer_dlss(&mut upscalers, dlss_supported);
    offer_fsr3(&mut upscalers, fsr3_supported);
    Capabilities {
        renderer: "bevy",
        api,
        compressed_bc,
        max_texture_anisotropy: 16,
        // Bloom and tone mapping need an `Hdr` camera (`PostSettings::needs_hdr`); set only when either runs.
        hdr_targets: true,
        antialiasing: AaSet::of(&[Antialiasing::Off, Antialiasing::Fxaa, Antialiasing::Smaa, Antialiasing::Taa]),
        upscalers,
        tonemaps: TonemapSet::of(&[Tonemap::Off, Tonemap::Aces]),
        bloom: true,
        scene_treatment: false,
        // The speed and pursuit blur is a post pass here too (`post::radial_blur`).
        radial_blur: true,
        frame_generation: false,
        local_reflections: false,
        projected_reflections: false,
        ambient_occlusion: false,
        soft_shadows: !matches!(api, GraphicsApi::Gl),
        // Bevy builds its sun shadows from the main list, so its off-screen casters must stay in it.
        separate_shadow_casters: false,
        view_capture_without_ui: false,
        ray_tracing,
        // Rendering above the surface size is not wired up.
        render_scale: (MIN_RENDER_SCALE, 1.0),
        // Texture samplers are fixed before render-only image uploads; ray tracing also needs a new device.
        restart_required: RestartSet::of(&[Setting::RayTracing, Setting::TextureFiltering]),
    }
}

#[cfg(any(feature = "dlss", feature = "dlss-mock"))]
fn offer_dlss(upscalers: &mut UpscalerSet, supported: bool) {
    crate::dlss::offer(upscalers, supported);
}

/// Builds without DLSS offer nothing extra.
#[cfg(not(any(feature = "dlss", feature = "dlss-mock")))]
fn offer_dlss(_upscalers: &mut UpscalerSet, _supported: bool) {}

/// FSR 3 is offered when the build has it and the device can run it (`fsr3::support`).
#[cfg(feature = "fsr3")]
fn offer_fsr3(upscalers: &mut UpscalerSet, supported: bool) {
    crate::fsr3::offer(upscalers, supported);
}

/// Builds without FSR 3 offer nothing extra.
#[cfg(not(feature = "fsr3"))]
fn offer_fsr3(_upscalers: &mut UpscalerSet, _supported: bool) {}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_gfx::{Denoiser, GraphicsSettings, PostSettings, RayTracing, resolve};

    #[test]
    fn ray_tracing_is_reported_as_the_device_offers_it() {
        let offered = RtSupport::Available { denoiser: Some(Denoiser::DlssRr) };
        let caps = capabilities_with_ray_tracing(GraphicsApi::Vulkan, true, false, offered, false);
        assert_eq!(caps.ray_tracing, offered);
        // The default constructors never offer it: nothing said the device has the features.
        assert_eq!(capabilities(GraphicsApi::Vulkan, true).ray_tracing, RtSupport::None);
        assert_eq!(
            capabilities_with_ray_tracing(GraphicsApi::Vulkan, true, false, offered, false).ray_tracing,
            offered
        );
        assert_eq!(capabilities_with_dlss(GraphicsApi::Vulkan, true, true).ray_tracing, RtSupport::None);
    }

    #[test]
    fn pr10_offers_every_post_effect_but_no_ray_tracing_or_temporal_upscaler() {
        let caps = capabilities(GraphicsApi::Vulkan, true);
        assert_eq!(caps.renderer, "bevy");
        assert_eq!(
            caps.antialiasing,
            AaSet::of(&[Antialiasing::Off, Antialiasing::Fxaa, Antialiasing::Smaa, Antialiasing::Taa])
        );
        // The `dlss-mock` build also offers DLSS (`dlss::offer`); no build offers it without a supporting GPU here.
        let mut upscalers = UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear, Upscaler::Fsr1]);
        if cfg!(feature = "dlss-mock") {
            upscalers.insert(Upscaler::Dlss);
        }
        assert_eq!(caps.upscalers, upscalers);
        assert_eq!(caps.tonemaps, TonemapSet::of(&[Tonemap::Off, Tonemap::Aces]));
        assert!(caps.bloom && caps.hdr_targets);
        assert!(!caps.ray_tracing.is_available());
        assert_eq!(caps.restart_required, RestartSet::of(&[Setting::RayTracing, Setting::TextureFiltering]));
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
        // These caps do not offer FSR 3 (only the device-checked factory path does, see `capabilities_with_ray_tracing`),
        // so it falls back along the chain to FSR 1, the next best upscaler this renderer has; TAA and ACES are both
        // offered too and go straight through.
        assert_eq!(resolved.effective.upscaler, Upscaler::Fsr1);
        assert_eq!(resolved.effective.post.tonemap, Tonemap::Aces);
        assert_eq!(resolved.effective.post.antialiasing, Antialiasing::Taa);
        assert_eq!(resolved.effective.ray_tracing, RayTracing::Off);
        assert!(resolved.effective.render_scale <= 1.0);
        assert!(resolved.downgrade_of(Setting::Upscaler).unwrap().reason.contains("bevy"));
        assert!(resolved.downgrade_of(Setting::RayTracing).unwrap().reason.contains("bevy"));
    }

    #[test]
    fn soft_shadows_need_a_supported_api_and_change_without_restart() {
        let wanted =
            blackbox_gfx::GraphicsSettings { shadows: blackbox_gfx::ShadowQuality::High, ..Default::default() };
        for api in [GraphicsApi::Vulkan, GraphicsApi::Dx12] {
            let caps = capabilities(api, true);
            assert!(blackbox_gfx::resolve(&wanted, &caps).is_exact());
            assert!(caps.restart_needed(&Default::default(), &wanted).is_empty());
        }
        let caps = capabilities(GraphicsApi::Gl, true);
        let resolved = blackbox_gfx::resolve(&wanted, &caps);
        assert_eq!(resolved.effective.shadows, blackbox_gfx::ShadowQuality::Off);
        assert!(resolved.downgrade_of(Setting::Shadows).is_some());
    }
}
