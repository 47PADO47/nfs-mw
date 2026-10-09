//! The native backend's capabilities, `resolve` and `apply_graphics`, on a real renderer.

use blackbox_gfx::{
    AaSet, Antialiasing, GraphicsApi, GraphicsSettings, PostSettings, RayTracing, RenderBackend, RtSupport, Setting,
    Tonemap, TonemapSet, UpscaleQuality, Upscaler, UpscalerSet, scaled_size, suggested_texture_lod_bias,
};

use super::SIZE;
use crate::Renderer;
use crate::gpu::test_support;

fn renderer() -> Option<(std::sync::MutexGuard<'static, ()>, Renderer)> {
    let guard = test_support::serial();
    let renderer = test_support::headless((SIZE[0], SIZE[1]))?;
    Some((guard, renderer))
}

#[test]
#[ignore = "needs a GPU"]
fn the_native_capabilities_are_the_frozen_set() {
    let Some((_gpu, mut renderer)) = renderer() else { return };
    let caps = *renderer.capabilities();
    assert_eq!(caps.renderer, "blackbox");
    assert_eq!(caps.antialiasing, AaSet::of(&[Antialiasing::Off, Antialiasing::Fxaa]));
    assert_eq!(caps.upscalers, UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear, Upscaler::Fsr1]));
    assert_eq!(caps.tonemaps, TonemapSet::all());
    assert!(caps.bloom);
    assert_eq!(caps.ray_tracing, RtSupport::None);
    assert!(caps.restart_required.is_empty());

    let info = renderer.info().clone();
    assert_eq!(info.renderer, "blackbox");
    assert_eq!(caps.api, info.api);
    assert_ne!(caps.api, GraphicsApi::Auto, "the API in use is named");
    assert!(!info.adapter.is_empty());
    assert_eq!(caps.compressed_bc, renderer.supports_bc());

    // HDR targets are reported truthfully: bloom draws the scene in HDR exactly when they exist.
    let bloom = GraphicsSettings {
        post: PostSettings { bloom_intensity: 0.5, ..PostSettings::default() },
        ..GraphicsSettings::default()
    };
    assert!(renderer.apply_graphics(&bloom).is_exact());
    assert_eq!(renderer.is_hdr(), caps.hdr_targets);
}

#[test]
#[ignore = "needs a GPU"]
fn a_request_the_renderer_can_run_is_applied_as_asked() {
    let Some((_gpu, mut renderer)) = renderer() else { return };
    let wanted = GraphicsSettings {
        post: PostSettings {
            tonemap: Tonemap::Aces,
            exposure: 1.25,
            bloom_intensity: 0.4,
            antialiasing: Antialiasing::Fxaa,
            ..PostSettings::default()
        },
        upscaler: Upscaler::Fsr1,
        render_scale: 0.5,
        upscale_sharpness: 0.4,
        ..GraphicsSettings::default()
    };
    let resolved = renderer.apply_graphics(&wanted);
    assert!(resolved.is_exact() && resolved.effective == wanted, "{:?}", resolved.downgrades);
    assert_eq!(renderer.graphics(), &wanted);
    assert_eq!(renderer.post_effects(), wanted.post);
    assert_eq!(renderer.upscaler(), Upscaler::Fsr1);
    assert_eq!(renderer.render_scale(), 0.5);
    assert_eq!(renderer.upscale_sharpness(), 0.4);
    assert_eq!(renderer.texture_lod_bias(), suggested_texture_lod_bias(0.5));
    assert!(renderer.fsr1_active() && !renderer.draws_directly());
    let (w, h) = scaled_size((SIZE[0], SIZE[1]), 0.5);
    assert_eq!(RenderBackend::render_size(&renderer), [w, h]);
    assert_eq!(RenderBackend::surface_size(&renderer), SIZE);

    // Asking for nothing puts everything back, and the scene goes straight to the output again.
    assert!(renderer.apply_graphics(&GraphicsSettings::default()).is_exact());
    assert!(renderer.draws_directly() && !renderer.fsr1_active());
    assert_eq!(renderer.texture_lod_bias(), 0.0);
    assert_eq!(RenderBackend::render_size(&renderer), SIZE);
}

#[test]
#[ignore = "needs a GPU"]
fn what_the_renderer_lacks_is_downgraded_reported_and_applied() {
    let Some((_gpu, mut renderer)) = renderer() else { return };
    let wanted = GraphicsSettings {
        post: PostSettings { antialiasing: Antialiasing::Taa, ..PostSettings::default() },
        upscaler: Upscaler::Dlss,
        upscale_quality: UpscaleQuality::Quality,
        ray_tracing: RayTracing::High,
        ..GraphicsSettings::default()
    };
    let resolved = renderer.apply_graphics(&wanted);
    assert!(!resolved.is_exact());
    for setting in [Setting::Upscaler, Setting::Antialiasing, Setting::RayTracing] {
        let downgrade = resolved.downgrade_of(setting).unwrap_or_else(|| panic!("{setting} is downgraded"));
        assert!(downgrade.reason.contains("blackbox"), "{downgrade}");
    }
    let effective = resolved.effective;
    assert_eq!(
        (effective.upscaler, effective.post.antialiasing, effective.ray_tracing),
        (Upscaler::Fsr1, Antialiasing::Fxaa, RayTracing::Off)
    );
    // The quality mode of the requested temporal upscaler decided the scale; the renderer runs what was left.
    assert_eq!(effective.render_scale, UpscaleQuality::Quality.render_scale());
    assert_eq!(renderer.graphics(), &effective);
    assert_eq!(renderer.upscaler(), Upscaler::Fsr1);
    assert_eq!(renderer.post_effects().antialiasing, Antialiasing::Fxaa);
    assert_eq!(renderer.render_scale(), effective.render_scale);

    // Resolving what was effective changes nothing.
    assert!(renderer.apply_graphics(&effective).is_exact());
    assert_eq!(renderer.graphics(), &effective);
}

#[test]
#[ignore = "needs a GPU"]
fn no_upscaler_means_the_output_size() {
    let Some((_gpu, mut renderer)) = renderer() else { return };
    let wanted = GraphicsSettings { upscaler: Upscaler::Off, render_scale: 0.5, ..GraphicsSettings::default() };
    let resolved = renderer.apply_graphics(&wanted);
    assert!(resolved.downgrade_of(Setting::RenderScale).is_some());
    assert_eq!(resolved.effective.render_scale, 1.0);
    assert_eq!(RenderBackend::render_size(&renderer), SIZE);
    assert!(renderer.draws_directly());
}
