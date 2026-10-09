use super::*;
use crate::{GraphicsApi, PostSettings, RayTracing, UpscaleQuality};

fn native() -> Capabilities {
    Capabilities {
        hdr_targets: true,
        antialiasing: AaSet::of(&[Antialiasing::Off, Antialiasing::Fxaa]),
        upscalers: UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear, Upscaler::Fsr1]),
        tonemaps: TonemapSet::all(),
        bloom: true,
        render_scale: Capabilities::full_render_scale(),
        ..Capabilities::baseline("blackbox", GraphicsApi::Vulkan)
    }
}

fn everything() -> Capabilities {
    Capabilities {
        antialiasing: AaSet::all(),
        upscalers: UpscalerSet::all(),
        ray_tracing: RtSupport::Available { denoiser: Some(Denoiser::DlssRr) },
        ..native()
    }
}

fn ask(upscaler: Upscaler) -> GraphicsSettings {
    GraphicsSettings { upscaler, ..GraphicsSettings::default() }
}

fn with_aa(aa: Antialiasing, upscaler: Upscaler) -> GraphicsSettings {
    GraphicsSettings { post: PostSettings { antialiasing: aa, ..PostSettings::default() }, ..ask(upscaler) }
}

#[test]
fn a_supported_request_comes_back_unchanged() {
    let request = GraphicsSettings {
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
    let r = resolve(&request, &native());
    assert!(r.is_exact() && r.notes.is_empty(), "{:?}", r.downgrades);
    assert_eq!(r.effective, request);
}

#[test]
fn the_default_request_resolves_exactly_even_on_the_baseline() {
    let caps = Capabilities {
        upscalers: UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear]),
        ..Capabilities::baseline("minimal", GraphicsApi::Gl)
    };
    let r = resolve(&GraphicsSettings::default(), &caps);
    assert!(r.is_exact(), "{:?}", r.downgrades);
    assert_eq!(r.effective, GraphicsSettings::default());
}

#[test]
fn fsr4_falls_back_through_fsr3_to_fsr1() {
    let mut caps = everything();
    let r = resolve(&ask(Upscaler::Fsr4), &caps);
    assert_eq!((r.effective.upscaler, r.is_exact()), (Upscaler::Fsr4, true));

    caps.upscalers.remove(Upscaler::Fsr4);
    assert_eq!(resolve(&ask(Upscaler::Fsr4), &caps).effective.upscaler, Upscaler::Fsr3);

    caps.upscalers.remove(Upscaler::Fsr3);
    let r = resolve(&ask(Upscaler::Fsr4), &caps);
    assert_eq!(r.effective.upscaler, Upscaler::Fsr1);
    let d = r.downgrade_of(Setting::Upscaler).expect("a downgrade");
    assert_eq!((d.requested.as_str(), d.effective.as_str()), ("fsr4", "fsr1"));
    assert!(d.reason.contains("blackbox"), "{}", d.reason);

    caps.upscalers.remove(Upscaler::Fsr1);
    assert_eq!(resolve(&ask(Upscaler::Fsr4), &caps).effective.upscaler, Upscaler::Bilinear);
    caps.upscalers.remove(Upscaler::Bilinear);
    assert_eq!(resolve(&ask(Upscaler::Fsr4), &caps).effective.upscaler, Upscaler::Off);
}

#[test]
fn dlss_falls_back_through_fsr3_to_fsr1_never_fsr4() {
    let mut caps = everything();
    caps.upscalers.remove(Upscaler::Dlss);
    assert_eq!(resolve(&ask(Upscaler::Dlss), &caps).effective.upscaler, Upscaler::Fsr3);
    caps.upscalers.remove(Upscaler::Fsr3);
    assert_eq!(resolve(&ask(Upscaler::Dlss), &caps).effective.upscaler, Upscaler::Fsr1, "fsr4 is still available");
    caps.upscalers.remove(Upscaler::Fsr1);
    assert_eq!(resolve(&ask(Upscaler::Dlss), &caps).effective.upscaler, Upscaler::Bilinear);
}

#[test]
fn the_native_renderer_cannot_run_temporal_upscalers() {
    for u in [Upscaler::Fsr3, Upscaler::Fsr4, Upscaler::Dlss] {
        let r = resolve(&ask(u), &native());
        assert_eq!(r.effective.upscaler, Upscaler::Fsr1, "{u:?}");
        assert!(r.downgrade_of(Setting::Upscaler).is_some());
    }
    let r = resolve(&ask(Upscaler::Fsr1), &native());
    assert!(r.is_exact());
}

#[test]
fn a_temporal_upscaler_replaces_anti_aliasing_with_a_downgrade() {
    for aa in [Antialiasing::Taa, Antialiasing::Fxaa, Antialiasing::Smaa] {
        let r = resolve(&with_aa(aa, Upscaler::Fsr3), &everything());
        assert_eq!(r.effective.upscaler, Upscaler::Fsr3);
        assert_eq!(r.effective.post.antialiasing, Antialiasing::Off, "{aa:?}");
        let d = r.downgrade_of(Setting::Antialiasing).expect("a note about the replaced AA");
        assert_eq!((d.requested.as_str(), d.effective.as_str()), (aa.name(), "off"));
        assert!(d.reason.contains("fsr3"), "{}", d.reason);
    }
    let r = resolve(&with_aa(Antialiasing::Off, Upscaler::Dlss), &everything());
    assert!(r.downgrade_of(Setting::Antialiasing).is_none(), "nothing to replace");
}

#[test]
fn anti_aliasing_is_kept_when_the_temporal_upscaler_falls_back_to_a_spatial_one() {
    let r = resolve(&with_aa(Antialiasing::Fxaa, Upscaler::Fsr3), &native());
    assert_eq!((r.effective.upscaler, r.effective.post.antialiasing), (Upscaler::Fsr1, Antialiasing::Fxaa));
}

#[test]
fn unavailable_taa_and_smaa_fall_back_to_fxaa() {
    for aa in [Antialiasing::Taa, Antialiasing::Smaa] {
        let r = resolve(&with_aa(aa, Upscaler::Bilinear), &native());
        assert_eq!(r.effective.post.antialiasing, Antialiasing::Fxaa, "{aa:?}");
        let d = r.downgrade_of(Setting::Antialiasing).unwrap();
        assert_eq!((d.requested.as_str(), d.effective.as_str()), (aa.name(), "fxaa"));
    }
    let mut caps = native();
    caps.antialiasing = AaSet::of(&[Antialiasing::Off]);
    assert_eq!(
        resolve(&with_aa(Antialiasing::Taa, Upscaler::Off), &caps).effective.post.antialiasing,
        Antialiasing::Off
    );
    assert_eq!(
        resolve(&with_aa(Antialiasing::Fxaa, Upscaler::Off), &caps).effective.post.antialiasing,
        Antialiasing::Off
    );
}

#[test]
fn supported_taa_is_kept() {
    let r = resolve(&with_aa(Antialiasing::Taa, Upscaler::Bilinear), &everything());
    assert!(r.is_exact());
    assert_eq!(r.effective.post.antialiasing, Antialiasing::Taa);
}

#[test]
fn ray_tracing_without_support_is_turned_off() {
    let request = GraphicsSettings { ray_tracing: RayTracing::High, ..GraphicsSettings::default() };
    let r = resolve(&request, &native());
    assert_eq!(r.effective.ray_tracing, RayTracing::Off);
    let d = r.downgrade_of(Setting::RayTracing).unwrap();
    assert_eq!((d.requested.as_str(), d.effective.as_str()), ("high", "off"));
    assert!(resolve(&GraphicsSettings::default(), &native()).is_exact());
}

#[test]
fn ray_tracing_without_a_denoiser_is_allowed_with_a_noisy_note() {
    let mut caps = everything();
    caps.ray_tracing = RtSupport::Available { denoiser: None };
    let request = GraphicsSettings { ray_tracing: RayTracing::Medium, ..GraphicsSettings::default() };
    let r = resolve(&request, &caps);
    assert!(r.is_exact(), "kept as asked");
    assert_eq!(r.effective.ray_tracing, RayTracing::Medium);
    assert_eq!(r.notes.len(), 1);
    assert_eq!(r.notes[0].setting, Setting::RayTracing);
    assert!(r.notes[0].text.contains("noisy"), "{}", r.notes[0]);

    caps.ray_tracing = RtSupport::Available { denoiser: Some(Denoiser::DlssRr) };
    assert!(resolve(&request, &caps).notes.is_empty());
}

#[test]
fn render_scale_is_clamped_to_the_renderers_range() {
    let mut caps = native();
    caps.render_scale = (0.5, 1.0);
    let low = GraphicsSettings { render_scale: 0.3, ..ask(Upscaler::Fsr1) };
    let r = resolve(&low, &caps);
    assert_eq!(r.effective.render_scale, 0.5);
    let d = r.downgrade_of(Setting::RenderScale).unwrap();
    assert_eq!((d.requested.as_str(), d.effective.as_str()), ("0.30", "0.50"));

    let high = GraphicsSettings { render_scale: 1.5, ..ask(Upscaler::Fsr1) };
    assert_eq!(resolve(&high, &caps).effective.render_scale, 1.0);
    let inside = GraphicsSettings { render_scale: 0.75, ..ask(Upscaler::Fsr1) };
    assert!(resolve(&inside, &caps).is_exact());
}

#[test]
fn wild_render_scales_are_sanitised_before_the_range_check() {
    let wild = GraphicsSettings { render_scale: 50.0, ..ask(Upscaler::Bilinear) };
    let r = resolve(&wild, &native());
    assert_eq!(r.effective.render_scale, crate::MAX_RENDER_SCALE);
    assert!(r.is_exact(), "the settings clamp is silent");
    let nan = GraphicsSettings { render_scale: f32::NAN, ..ask(Upscaler::Bilinear) };
    assert_eq!(resolve(&nan, &native()).effective.render_scale, 1.0);
}

#[test]
fn a_temporal_upscalers_quality_mode_decides_the_render_scale() {
    let request =
        GraphicsSettings { upscale_quality: UpscaleQuality::Performance, render_scale: 0.9, ..ask(Upscaler::Fsr3) };
    let r = resolve(&request, &everything());
    assert_eq!(r.effective.render_scale, 0.5);
    assert!(r.downgrade_of(Setting::RenderScale).is_none(), "ignored, not downgraded");
    let balanced = GraphicsSettings { upscale_quality: UpscaleQuality::Balanced, ..request };
    assert_eq!(resolve(&balanced, &everything()).effective.render_scale, UpscaleQuality::Balanced.render_scale());
}

#[test]
fn a_fallback_from_a_temporal_upscaler_keeps_the_quality_mode_scale() {
    let request = GraphicsSettings { upscale_quality: UpscaleQuality::Performance, ..ask(Upscaler::Dlss) };
    let r = resolve(&request, &native());
    assert_eq!((r.effective.upscaler, r.effective.render_scale), (Upscaler::Fsr1, 0.5));
}

#[test]
fn a_temporal_request_that_falls_all_the_way_to_off_draws_at_native_size() {
    let request = GraphicsSettings { upscale_quality: UpscaleQuality::UltraPerformance, ..ask(Upscaler::Fsr3) };
    let r = resolve(&request, &Capabilities::baseline("minimal", GraphicsApi::Gl));
    assert_eq!((r.effective.upscaler, r.effective.render_scale), (Upscaler::Off, 1.0));
    assert!(r.downgrade_of(Setting::RenderScale).is_none());
}

#[test]
fn upscaler_off_draws_at_the_output_size() {
    let request = GraphicsSettings { render_scale: 0.5, ..ask(Upscaler::Off) };
    let r = resolve(&request, &native());
    assert_eq!(r.effective.render_scale, 1.0);
    assert!(r.downgrade_of(Setting::RenderScale).is_some());
    assert!(resolve(&ask(Upscaler::Off), &native()).is_exact());
}

#[test]
fn missing_tone_mapping_and_bloom_are_turned_off() {
    let request = GraphicsSettings {
        post: PostSettings { tonemap: Tonemap::Aces, bloom_intensity: 0.5, ..PostSettings::default() },
        ..GraphicsSettings::default()
    };
    let r = resolve(&request, &Capabilities::baseline("minimal", GraphicsApi::Gl));
    assert_eq!((r.effective.post.tonemap, r.effective.post.bloom_intensity), (Tonemap::Off, 0.0));
    assert!(r.downgrade_of(Setting::Tonemap).is_some() && r.downgrade_of(Setting::Bloom).is_some());
}

#[test]
fn downgrades_list_in_rule_order_and_print_readably() {
    let request = GraphicsSettings {
        post: PostSettings { antialiasing: Antialiasing::Taa, ..PostSettings::default() },
        upscaler: Upscaler::Dlss,
        ray_tracing: RayTracing::Low,
        ..GraphicsSettings::default()
    };
    let r = resolve(&request, &native());
    let order: Vec<_> = r.downgrades.iter().map(|d| d.setting).collect();
    assert_eq!(order, [Setting::Upscaler, Setting::Antialiasing, Setting::RayTracing]);
    assert_eq!(
        r.downgrades[0].to_string(),
        "upscaler: dlss -> fsr1 (not supported by the blackbox renderer on vulkan)"
    );
}

#[test]
fn resolving_an_effective_result_again_changes_nothing() {
    let request = GraphicsSettings {
        post: PostSettings { antialiasing: Antialiasing::Taa, ..PostSettings::default() },
        upscaler: Upscaler::Dlss,
        upscale_quality: UpscaleQuality::Balanced,
        ray_tracing: RayTracing::High,
        ..GraphicsSettings::default()
    };
    let first = resolve(&request, &native());
    let second = resolve(&first.effective, &native());
    assert!(second.is_exact(), "{:?}", second.downgrades);
    assert_eq!(second.effective, first.effective);
}
