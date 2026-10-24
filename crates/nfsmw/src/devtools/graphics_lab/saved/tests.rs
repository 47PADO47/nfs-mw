use super::*;

#[test]
fn pursuit_limit_enhancements_round_trip_through_startup_and_named_snapshot_payloads() {
    let mut settings = Settings::from(crate::settings::Partial::default());
    settings.pursuit_options = crate::settings::PursuitOptions { enhanced: true, max_active_cops: 24, heat_cap: 7 };
    let text = payload(&settings).unwrap();
    let wanted = settings.pursuit_options;
    settings.pursuit_options = Default::default();
    settings.master_volume = crate::settings::Percent(17);
    restore(&mut settings, &text).unwrap();
    assert_eq!(settings.pursuit_options, wanted);
    assert_eq!(settings.master_volume, crate::settings::Percent(17));
    let merged =
        crate::settings::write::merge("[pursuit_options]\nfuture_limit=42", &settings.graphics_layer()).unwrap();
    assert_eq!(merged.parse::<toml::Table>().unwrap()["pursuit_options"]["future_limit"].as_integer(), Some(42));
    let before = settings;
    assert!(restore(&mut settings, "[pursuit_options]\nmax_active_cops='bad'").is_err());
    assert_eq!(settings, before);
}

#[test]
fn f8_traffic_presets_round_trip_without_changing_audio_or_renderer() {
    let mut settings = Settings::from(Partial::default());
    settings.select_traffic_preset(crate::settings::TrafficPreset::Extended);
    settings.select_population_preset(crate::settings::PopulationPreset::VeryHigh);
    settings.traffic_policy.density_percent = 30;
    let text = payload(&settings).unwrap();
    let wanted = settings.graphics_layer();
    settings.select_traffic_preset(crate::settings::TrafficPreset::Reference);
    settings.traffic = 5;
    settings.traffic_policy.density_percent = 100;
    settings.master_volume = Percent(17);
    restore(&mut settings, &text).unwrap();
    assert_eq!(settings.graphics_layer(), wanted);
    assert_eq!(settings.master_volume, Percent(17));
    let before = settings;
    assert!(restore(&mut settings, "[traffic_tuning]\nspawn_max='bad'").is_err());
    assert_eq!(settings, before, "bad traffic payloads must not partially apply");
}
use crate::settings::PostAa;
use crate::settings::{Partial, Percent, RenderScale, RendererKind, UpscaleMode};
use blackbox_gfx::ExperimentStats;

#[test]
fn saved_blackbox_profile_restores_complete_requests_and_preserves_runtime_preferences() {
    let mut settings = Settings::from(Partial::default());
    settings.renderer = RendererKind::Blackbox;
    settings.select_visual_style(crate::settings::VisualStyle::Enhanced);
    settings.render_scale = RenderScale::new(67).unwrap();
    settings.upscaler = UpscaleMode::Fsr1;
    settings.upscale_sharpness = Percent(31);
    settings.upscale_quality = crate::settings::UpscaleQuality::Performance;
    settings.appearance.display.peak_nits = 1350.0;
    settings.appearance.display.ui_white_nits = 150.0;
    settings.appearance.lighting.shadow_filter = 1.85;
    settings.appearance.lighting.dense_shadow_filter = true;
    settings.appearance.lighting.near_shadow_focus = true;
    settings.appearance.lighting.wide_shadow_detail = true;
    settings.appearance.lighting.shadow_detail_resolution = 8192;
    settings.appearance.lighting.shadow_resolution = 8192;
    settings.speed_trails = true;
    settings.tire_smoke = false;
    settings.ao_depth_mips = true;
    settings.ao_method = crate::settings::AoMethod::XeGtao;
    settings.collision_sparks = true;
    settings.reflection_cube_size = "200%".parse().unwrap();
    settings.reflection_mirror_size = "512".parse().unwrap();
    settings.reflection_pow2 = true;
    settings.mirror_tint = "on".parse().unwrap();
    settings.reflection_extra.distance = "original".parse().unwrap();
    settings.reflection_extra.lod = "view".parse().unwrap();
    settings.reflection_extra.content = "restored".parse().unwrap();
    settings.reflection_extra.brightness = "150%".parse().unwrap();
    settings.fe_shadows.enabled = true;
    settings.fe_shadows.bias = 1.5;
    settings.motion_blur = false;
    settings.motion_blur_strength = 0.4;
    settings.rain_droplet_scale = 0.3;
    settings.backend = blackbox_gfx::GraphicsApi::Vulkan;
    settings.window_mode = crate::settings::WindowMode::Borderless;
    settings.monitor = crate::settings::Monitor::Index(2);
    settings.resolution = crate::settings::Resolution::pixels(3440, 1440).unwrap();
    settings.vsync = false;
    settings.max_fps = "165".parse().unwrap();
    settings.show_metrics = crate::devtools::ShowMetrics::Advanced;
    settings.custom_fonts.enabled = true;
    settings.hd_content.fonts = false;
    settings.hd_content.cursor = false;
    settings.hd_content.fmv = false;
    settings.detail.world = crate::settings::WorldDetail::Medium;
    settings.detail.car = crate::settings::CarGeometry::Low;
    settings.detail.visibility_range = crate::settings::detail::DetailRange::Extended;
    settings.detail.geometry_range = crate::settings::detail::DetailRange::Double;
    settings.detail.highest_player = true;
    let text = payload(&settings).unwrap();
    let wanted = settings.graphics_layer();
    let expected_request = settings.graphics();
    let mut restored = Settings::from(Partial::default());
    restored.renderer = RendererKind::Blackbox;
    restored.camera_mode = crate::settings::CameraMode::Hood;
    restored.master_volume = Percent(17);
    restored.always_rain = true;
    restore(&mut restored, &text).unwrap();
    assert_eq!(restored.graphics_layer(), wanted);
    assert_eq!(restored.graphics(), expected_request);
    assert_eq!(restored.renderer, RendererKind::Blackbox);
    assert_eq!(restored.camera_mode, crate::settings::CameraMode::Hood);
    assert_eq!(restored.master_volume, Percent(17));
    assert!(restored.always_rain);
    let caps = crate::settings::test_caps::native();
    assert_eq!(restored.effective(&caps).upscaler, UpscaleMode::Fsr1);
    assert_eq!(restored.effective(&caps).render_scale, settings.render_scale);
}

#[test]
fn legacy_saved_profiles_leave_absent_controls_intact_and_invalid_profiles_do_not_apply() {
    let mut settings = Settings::from(Partial::default());
    settings.render_scale = RenderScale::new(75).unwrap();
    settings.upscaler = UpscaleMode::Fsr1;
    restore(&mut settings, "post_aa='smaa'").unwrap();
    assert_eq!(settings.render_scale.percent(), 75);
    assert_eq!(settings.upscaler, UpscaleMode::Fsr1);
    assert_eq!(settings.post_aa, PostAa::Smaa);
    let before = settings;
    for text in ["[appearance]\nversion=99", "profile_version=99", "broken=[", "[detail]\nworld='bogus'"] {
        assert!(restore(&mut settings, text).is_err());
        assert_eq!(settings, before);
    }
}

#[test]
fn real_f8_save_and_load_buttons_work_with_blackbox_and_do_not_replace_other_config() {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("nfsmw-f8-{}-{stamp}.toml", std::process::id()));
    std::fs::write(&path, "game_dir='keep-me'\nmaster_volume=17\nfuture=42\n").unwrap();
    let mut lab = GraphicsLab {
        open: true,
        profile_path: Some(path.clone()),
        caps: Some(crate::settings::test_caps::native()),
        stats: ExperimentStats { supported: true, ..Default::default() },
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let mut settings = Settings::from(Partial::default());
    settings.renderer = RendererKind::Blackbox;
    settings.select_visual_style(crate::settings::VisualStyle::Enhanced);
    settings.render_scale = RenderScale::new(67).unwrap();
    settings.upscaler = UpscaleMode::Fsr1;
    let wanted = settings.graphics_layer();
    super::super::tests::click(&ctx, &mut lab, &mut settings, "Save for startup");
    assert!(lab.message.as_ref().unwrap().starts_with("Saved for next launch"));
    settings.select_visual_style(crate::settings::VisualStyle::OriginalPc);
    settings.render_scale = RenderScale::default();
    settings.upscaler = UpscaleMode::Off;
    super::super::tests::click(&ctx, &mut lab, &mut settings, "Revert saved");
    assert_eq!(settings.graphics_layer(), wanted);
    let table: toml::Table = std::fs::read_to_string(&path).unwrap().parse().unwrap();
    assert_eq!(table["game_dir"].as_str(), Some("keep-me"));
    assert_eq!(table["master_volume"].as_integer(), Some(17));
    assert_eq!(table["future"].as_integer(), Some(42));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn named_snapshot_loads_immediately_and_save_as_also_persists_startup() {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let folder = std::env::temp_dir().join(format!("nfsmw-snapshot-{}-{stamp}", std::process::id()));
    let path = folder.join("config.toml");
    let mut settings = Settings::from(Partial::default());
    settings.select_visual_style(crate::settings::VisualStyle::Enhanced);
    settings.select_graphics_quality(crate::settings::GraphicsQuality::VeryHigh);
    let named = folder.join("graphics-presets.toml");
    crate::preset_store::save(&named, "Default test", &payload(&settings).unwrap()).unwrap();
    crate::preset_store::save(&named, "Keep", "post_aa='off'").unwrap();
    let wanted = snapshot(&settings);
    settings.select_graphics_quality(crate::settings::GraphicsQuality::Low);
    let mut lab = GraphicsLab {
        open: true,
        profile_path: Some(path.clone()),
        caps: Some(crate::settings::test_caps::native()),
        stats: ExperimentStats { supported: true, ..Default::default() },
        ..Default::default()
    };
    let ctx = egui::Context::default();
    super::super::tests::click(&ctx, &mut lab, &mut settings, "Load preset…");
    super::super::tests::click(&ctx, &mut lab, &mut settings, "Default test");
    assert_eq!(snapshot(&settings), wanted);
    assert!(!path.exists(), "selecting a preset previews without changing startup");
    super::super::tests::click(&ctx, &mut lab, &mut settings, "Save as…");
    lab.disk.name = "New comparison".into();
    super::super::tests::click(&ctx, &mut lab, &mut settings, "Save preset + startup");
    assert_eq!(lab.disk.startup, Some(wanted));
    assert!(path.exists());
    assert_eq!(crate::preset_store::load(&named, "Keep").unwrap(), "post_aa='off'");
    assert_eq!(crate::preset_store::load(&named, "Default test").unwrap(), payload(&settings).unwrap());
    assert_eq!(crate::preset_store::load(&named, "New comparison").unwrap(), payload(&settings).unwrap());
    let mut launched = Settings::from(crate::settings::file::read(&path));
    assert_eq!(snapshot(&launched), wanted);
    launched.post_aa = PostAa::Off;
    restore(&mut launched, &crate::preset_store::load(&named, "New comparison").unwrap()).unwrap();
    assert_eq!(snapshot(&launched), wanted);
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn replacing_a_named_snapshot_retains_same_schema_fields_from_newer_builds() {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let folder = std::env::temp_dir().join(format!("nfsmw-future-{stamp}"));
    let mut lab = GraphicsLab { profile_path: Some(folder.join("config.toml")), ..Default::default() };
    lab.disk.name = "Future".into();
    let path = preset_path(&lab).unwrap();
    crate::preset_store::save(
        &path,
        "Future",
        "profile_version=1\nfuture=42\n[appearance]\nversion=1\n[appearance.lighting]\nfuture_light=17",
    )
    .unwrap();
    let settings = Settings::from(Partial::default());
    save_named(&lab, &settings).unwrap();
    let table: toml::Table = crate::preset_store::load(&path, "Future").unwrap().parse().unwrap();
    assert_eq!(table["future"].as_integer(), Some(42));
    assert_eq!(table["appearance"]["lighting"]["future_light"].as_integer(), Some(17));
    crate::preset_store::save(&path, "Future", "profile_version=99").unwrap();
    assert!(save_named(&lab, &settings).is_err());
    assert_eq!(crate::preset_store::load(&path, "Future").unwrap(), "profile_version=99");
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn older_lighting_profiles_preserve_newer_fields_missing_from_the_document() {
    let mut settings = Settings::from(Partial::default());
    settings.select_visual_style(crate::settings::VisualStyle::Enhanced);
    settings.select_graphics_quality(crate::settings::GraphicsQuality::VeryHigh);
    settings.appearance.display.scene_white_nits = 260.0;
    settings.ao_method = crate::settings::AoMethod::XeGtao;
    settings.ao_depth_mips = true;
    restore(
        &mut settings,
        "profile_version=1\n[appearance]\nversion=1\nstyle='enhanced'\n[appearance.lighting]\nshadow_filter=1.75",
    )
    .unwrap();
    assert_eq!(settings.appearance.lighting.shadow_filter, 1.75);
    assert_eq!(settings.appearance.lighting.shadow_detail_resolution, 8192);
    assert!(settings.appearance.lighting.wide_shadow_detail);
    assert!(settings.appearance.lighting.dense_shadow_filter);
    assert_eq!(settings.appearance.display.scene_white_nits, 260.0);
    assert_eq!(settings.ao_method, crate::settings::AoMethod::XeGtao);
    assert!(settings.ao_depth_mips);
}

#[test]
fn revert_older_startup_file_discards_unsaved_new_fields_but_keeps_runtime_diagnostics() {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("nfsmw-revert-{stamp}.toml"));
    let old = "ao='high'\n[appearance]\nversion=1\nstyle='custom'\n[appearance.lighting]\nsun_shadows=true";
    std::fs::write(&path, old).unwrap();
    let mut settings = Settings::from(crate::settings::file::parse(old, "test"));
    let original = snapshot(&settings);
    let mut lab = GraphicsLab { open: true, profile_path: Some(path.clone()), ..Default::default() };
    let ctx = egui::Context::default();
    super::super::tests::pass(&ctx, &mut lab, &mut settings, vec![]);
    settings.appearance.lighting.wide_shadow_detail = true;
    settings.appearance.lighting.shadow_detail_resolution = 8192;
    settings.appearance.lighting.freeze_sun = true;
    settings.appearance.lighting.lighting_debug = blackbox_gfx::LightingDebug::Cascades;
    settings.ao_method = crate::settings::AoMethod::XeGtao;
    super::super::tests::click(&ctx, &mut lab, &mut settings, "Revert saved");
    assert_eq!(snapshot(&settings), original);
    assert!(settings.appearance.lighting.freeze_sun);
    assert_eq!(settings.appearance.lighting.lighting_debug, blackbox_gfx::LightingDebug::Cascades);
    std::fs::remove_file(path).unwrap();
}
