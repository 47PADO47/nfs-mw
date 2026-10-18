use super::*;
use crate::settings::{GraphicsQuality, Partial, RendererKind, TextureFiltering, VisualStyle};

#[test]
fn video_vsync_control_is_live_and_participates_in_comparison_and_reset() {
    let ctx = egui::Context::default();
    let mut lab = GraphicsLab {
        open: true,
        tab: profiles::Tab::Video,
        caps: Some(crate::settings::test_caps::native()),
        profile_path: None,
        ..Default::default()
    };
    let mut settings = Settings::from(Partial::default());
    let original = settings.vsync;
    super::tests::click(&ctx, &mut lab, &mut settings, "Store A");
    super::tests::click(&ctx, &mut lab, &mut settings, "VSync");
    assert_eq!(settings.vsync, !original);
    super::tests::click(&ctx, &mut lab, &mut settings, "Swap A/B");
    assert_eq!(settings.vsync, original);
    super::tests::click(&ctx, &mut lab, &mut settings, "VSync");
    super::tests::click(&ctx, &mut lab, &mut settings, "Reset section");
    assert_eq!(settings.vsync, original);
}

#[test]
fn opening_video_retains_unavailable_dlss_and_custom_window_preferences() {
    let ctx = egui::Context::default();
    let caps = crate::settings::test_caps::native();
    let mut settings = Settings::from(Partial::default());
    settings.upscaler = crate::settings::UpscaleMode::Dlss;
    settings.upscale_quality = crate::settings::UpscaleQuality::Native;
    settings.monitor = crate::settings::Monitor::Index(12);
    settings.max_fps = "117".parse().unwrap();
    settings.resolution = crate::settings::Resolution::pixels(1792, 1008).unwrap();
    let before = settings;
    let lab = GraphicsLab { caps: Some(caps), ..Default::default() };
    for _ in 0..3 {
        let mut output = ctx.run_ui(Default::default(), |ui| video::show(ui, &lab, &mut settings));
        output.textures_delta.clear();
        assert_eq!(settings, before);
    }
    assert!(!video::upscalers(Some(&caps)).contains(&crate::settings::UpscaleMode::Dlss));
    let mut dlss = blackbox_gfx::Capabilities::mock_dlss("blackbox", blackbox_gfx::GraphicsApi::Vulkan);
    assert!(video::upscalers(Some(&dlss)).contains(&crate::settings::UpscaleMode::Dlss));
    dlss.upscalers.remove(blackbox_gfx::Upscaler::Dlss);
    assert!(!video::upscalers(Some(&dlss)).contains(&crate::settings::UpscaleMode::Dlss));
}

#[test]
fn active_renderer_hides_inapplicable_controls_without_losing_stored_values() {
    for native in [true, false] {
        let ctx = egui::Context::default();
        let caps = match native {
            true => crate::settings::test_caps::native(),
            false => blackbox_gfx::Capabilities::mock_dlss("bevy", blackbox_gfx::GraphicsApi::Vulkan),
        };
        let mut lab = GraphicsLab {
            open: true,
            caps: Some(caps),
            profile_path: None,
            stats: ExperimentStats { supported: native, ..Default::default() },
            ..Default::default()
        };
        let mut settings = Settings::from(Partial::default());
        settings.select_visual_style(VisualStyle::Enhanced);
        // Pending renderer selection must not change the currently running UI.
        settings.renderer = match native {
            true => RendererKind::Bevy,
            false => RendererKind::Blackbox,
        };
        let before = settings;
        for tab in [profiles::Tab::Lighting, profiles::Tab::Materials, profiles::Tab::Effects] {
            lab.tab = tab;
            super::tests::pass(&ctx, &mut lab, &mut settings, vec![]);
            let output = super::tests::pass(&ctx, &mut lab, &mut settings, vec![]);
            let texts: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(t) => Some(t.galley.job.text.as_str()),
                    _ => None,
                })
                .collect();
            let exclusive = match tab {
                profiles::Tab::Lighting => "Wide-detail cascades",
                profiles::Tab::Materials => "Car shading",
                _ => "AO method",
            };
            assert_eq!(texts.contains(&exclusive), native, "{exclusive}: {texts:?}");
            if !native {
                assert!(!texts.contains(&"Depth-prefiltered AO"));
                assert!(!texts.contains(&"Road reflection resolution"));
            }
        }
        assert_eq!(settings, before, "merely opening tabs must preserve hidden requests");
    }
}

#[test]
fn very_high_is_explicit_and_lowering_quality_releases_8k_detail_without_changing_the_look() {
    let mut settings = Settings::from(Partial::default());
    settings.select_visual_style(VisualStyle::Enhanced);
    settings.visual_treatment = VisualTreatment::Reference;
    settings.appearance.treatment_strength = Some(0.8);
    let display = settings.appearance.display;
    for quality in GraphicsQuality::ALL {
        let mut direct = settings;
        direct.select_graphics_quality(quality);
        let mut after = settings;
        after.select_graphics_quality(GraphicsQuality::VeryHigh);
        assert_eq!(after.appearance.lighting.shadow_detail_resolution, 8192);
        after.select_graphics_quality(quality);
        assert_eq!(after, direct);
        assert_eq!(after.texture_filtering, TextureFiltering::Anisotropic16);
        assert_eq!(after.appearance.display, display);
        assert_eq!(after.visual_treatment, VisualTreatment::Reference);
        assert_eq!(after.appearance.treatment_strength, Some(0.8));
    }
}

#[test]
fn bevy_quality_and_section_reset_do_not_rewrite_blackbox_experiments() {
    let mut settings = Settings::from(Partial::default());
    settings.select_visual_style(VisualStyle::Enhanced);
    settings.renderer = RendererKind::Bevy;
    let lighting = settings.appearance.lighting;
    for quality in GraphicsQuality::ALL {
        settings.select_graphics_quality(quality);
        assert_eq!(settings.appearance.lighting, lighting);
    }
    let mut lab = GraphicsLab {
        caps: Some(blackbox_gfx::Capabilities::mock_dlss("bevy", blackbox_gfx::GraphicsApi::Vulkan)),
        options: lighting,
        ..Default::default()
    };
    for tab in profiles::Tab::ALL {
        lab.tab = tab;
        profiles::reset(&mut lab, &mut settings);
        assert_eq!(lab.options, lighting);
        assert_eq!(settings.appearance.lighting, lighting);
    }
}

#[test]
fn section_reset_preserves_hdr_and_other_sections() {
    let mut settings = Settings::from(Partial::default());
    settings.select_visual_style(VisualStyle::Enhanced);
    settings.appearance.display.scene_white_nits = 260.0;
    settings.appearance.lighting.sun_angle = 1.4;
    settings.ao_intensity.0 = 27;
    settings.post_aa = PostAa::Off;
    let mut lab = GraphicsLab { options: settings.appearance.lighting, tab: profiles::Tab::Look, ..Default::default() };
    profiles::reset(&mut lab, &mut settings);
    assert_eq!(settings.post_aa, PostAa::Smaa);
    assert_eq!(settings.appearance.display.scene_white_nits, 260.0);
    assert_eq!(settings.ao_intensity.0, 27);
    assert_eq!(lab.options.sun_angle, 1.4);
}

#[test]
fn high_quality_chooses_xegtao_at_eighty_percent_without_enabling_disabled_ao() {
    for quality in [GraphicsQuality::High, GraphicsQuality::VeryHigh] {
        let mut settings = Settings::from(Partial::default());
        settings.ao = blackbox_gfx::AoQuality::Low;
        settings.ao_method = crate::settings::AoMethod::Ssao;
        settings.ao_intensity.0 = 29;
        settings.select_graphics_quality(quality);
        assert_eq!(settings.ao, blackbox_gfx::AoQuality::High);
        assert_eq!(settings.ao_method, crate::settings::AoMethod::XeGtao);
        assert_eq!(settings.ao_intensity.0, 80);
        assert!(settings.ao_depth_mips);
        assert_eq!(settings.detail.world, crate::settings::WorldDetail::Maximum);
        settings.ao = blackbox_gfx::AoQuality::Off;
        settings.ao_method = crate::settings::AoMethod::Ssao;
        settings.ao_intensity.0 = 29;
        settings.select_graphics_quality(quality);
        assert_eq!(settings.ao, blackbox_gfx::AoQuality::Off);
        assert_eq!(settings.ao_method, crate::settings::AoMethod::Ssao);
        assert_eq!(settings.ao_intensity.0, 29);
        settings.renderer = RendererKind::Bevy;
        settings.ao = blackbox_gfx::AoQuality::High;
        settings.select_graphics_quality(quality);
        assert_eq!(settings.ao_method, crate::settings::AoMethod::Ssao);
        assert_eq!(settings.ao_intensity.0, 29);
    }
}

#[test]
fn new_ao_defaults_and_explicit_saved_overrides_are_distinct() {
    let defaults = Settings::from(Partial::default());
    assert_eq!(defaults.ao_method, crate::settings::AoMethod::XeGtao);
    assert_eq!(defaults.ao_intensity.0, 80);
    let mut settings =
        Settings::from(crate::settings::file::parse("ao='high'\nao_method='ssao'\nao_intensity=35", "test"));
    assert_eq!(settings.ao_method, crate::settings::AoMethod::Ssao);
    assert_eq!(settings.ao_intensity.0, 35);
    let mut lab = GraphicsLab { tab: profiles::Tab::Effects, ..Default::default() };
    profiles::reset(&mut lab, &mut settings);
    assert_eq!(settings.ao_method, crate::settings::AoMethod::XeGtao);
    assert_eq!(settings.ao_intensity.0, 80);
}

#[test]
fn bevy_retains_shared_detail_and_traffic_tabs_and_resets() {
    let ctx = egui::Context::default();
    let mut settings = Settings::from(Partial::default());
    settings.renderer = RendererKind::Bevy;
    let mut lab = GraphicsLab { open: true, ..Default::default() };
    super::tests::click(&ctx, &mut lab, &mut settings, "Traffic");
    super::tests::click(&ctx, &mut lab, &mut settings, "30%");
    assert_eq!(settings.traffic_policy.density_percent, 30);
    super::tests::click(&ctx, &mut lab, &mut settings, "Reset section");
    assert_eq!(settings.traffic_policy, crate::settings::TrafficPolicy::default());
    super::tests::click(&ctx, &mut lab, &mut settings, "Detail");
    super::tests::click(&ctx, &mut lab, &mut settings, "Full scenery");
    assert!(settings.detail.highest_world);
    super::tests::click(&ctx, &mut lab, &mut settings, "Reset section");
    assert_eq!(settings.detail, profiles::defaults(&settings).detail);
}
