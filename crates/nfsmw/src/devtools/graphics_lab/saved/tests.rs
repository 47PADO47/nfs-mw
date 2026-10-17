use super::*;
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
    settings.speed_trails = true;
    settings.tire_smoke = false;
    settings.collision_sparks = true;
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
    for text in ["[appearance]\nversion=99", "profile_version=99", "broken=["] {
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
    super::super::tests::click(&ctx, &mut lab, &mut settings, "Save profile");
    assert!(lab.message.as_ref().unwrap().starts_with("Profile saved"));
    settings.select_visual_style(crate::settings::VisualStyle::OriginalPc);
    settings.render_scale = RenderScale::default();
    settings.upscaler = UpscaleMode::Off;
    super::super::tests::click(&ctx, &mut lab, &mut settings, "Load saved");
    assert_eq!(settings.graphics_layer(), wanted);
    let table: toml::Table = std::fs::read_to_string(&path).unwrap().parse().unwrap();
    assert_eq!(table["game_dir"].as_str(), Some("keep-me"));
    assert_eq!(table["master_volume"].as_integer(), Some(17));
    assert_eq!(table["future"].as_integer(), Some(42));
    std::fs::remove_file(path).unwrap();
}
