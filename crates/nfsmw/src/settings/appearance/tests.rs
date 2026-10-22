use super::*;
fn settings() -> Settings {
    Settings::from(Partial::default())
}

#[test]
fn sky_tuning_round_trips_and_sanitizes_without_requiring_new_keys_in_old_profiles() {
    let mut value = Appearance::read(&toml::from_str("version=1").unwrap(), "old").unwrap();
    assert_eq!(value.lighting.sky_sun_size, 2.0);
    value.lighting.sky_wind_speed = 12.0;
    value.lighting.sky_sun_size = 2.0;
    value.lighting.sky_sun_brightness = 0.5;
    value.lighting.sky_brightness = 1.25;
    value.lighting.sky_sunset_warmth = 1.5;
    assert_eq!(Appearance::read(&value.value().unwrap(), "round-trip").unwrap(), value);
    value.lighting.sky_wind_speed = f32::NAN;
    value.lighting.sky_sun_size = f32::INFINITY;
    value.lighting.sky_sun_brightness = -1.0;
    value.lighting.sky_brightness = 99.0;
    value.lighting.sky_sunset_warmth = -1.0;
    let actual = Appearance::read(&value.value().unwrap(), "bounded").unwrap().lighting;
    assert_eq!(
        (
            actual.sky_wind_speed,
            actual.sky_sun_size,
            actual.sky_sun_brightness,
            actual.sky_brightness,
            actual.sky_sunset_warmth
        ),
        (2.0, 2.0, 0.0, 2.0, 0.0)
    );
}

#[test]
fn older_original_profiles_keep_the_authored_sky_and_explicit_overrides_survive() {
    let mut s = settings();
    s.select_visual_style(VisualStyle::OriginalPc);
    let mut value = s.appearance.value().unwrap();
    value.get_mut("lighting").unwrap().as_table_mut().unwrap().remove("enhanced_sky");
    assert!(!Appearance::read(&value, "test").unwrap().lighting.enhanced_sky);
    value
        .get_mut("lighting")
        .unwrap()
        .as_table_mut()
        .unwrap()
        .insert("enhanced_sky".into(), toml::Value::Boolean(true));
    assert!(Appearance::read(&value, "test").unwrap().lighting.enhanced_sky);
    value["style"] = toml::Value::String("custom".into());
    value.get_mut("lighting").unwrap().as_table_mut().unwrap().remove("enhanced_sky");
    assert!(Appearance::read(&value, "test").unwrap().lighting.enhanced_sky);
}
#[test]
fn quality_changes_cost_without_changing_the_selected_art_or_gameplay() {
    for style in [VisualStyle::OriginalPc, VisualStyle::Enhanced] {
        let mut s = settings();
        s.select_visual_style(style);
        let art = (s.post_aa, s.post_bloom, s.post_tonemap, s.visual_treatment);
        let gameplay = (s.camera_mode, s.game_speed, s.always_rain, s.rain_amount, s.master_volume, s.max_fps);
        for quality in Quality::ALL {
            s.select_graphics_quality(quality);
            assert_eq!((s.post_aa, s.post_bloom, s.post_tonemap, s.visual_treatment), art);
            assert_eq!(
                (s.camera_mode, s.game_speed, s.always_rain, s.rain_amount, s.master_volume, s.max_fps),
                gameplay
            );
            assert_eq!(s.visual_style(), style);
        }
    }
}
#[test]
fn original_target_disables_enhancements_but_is_not_claimed_complete() {
    let mut s = settings();
    s.select_visual_style(VisualStyle::Enhanced);
    assert!(s.appearance.lighting.enhanced_sky);
    assert!(s.appearance.lighting.sun_shadows && s.appearance.lighting.clearcoat);
    s.select_visual_style(VisualStyle::OriginalPc);
    assert!(!s.appearance.lighting.enhanced_sky);
    assert!(
        s.appearance.lighting.sun_shadows
            && s.appearance.lighting.shadow_projection.is_some()
            && !s.appearance.lighting.contact_shadows
            && !s.appearance.lighting.clearcoat
    );
    assert!(s.appearance.lighting.original_shadows);
    assert_eq!(s.ao, AoQuality::Off);
    assert_eq!(s.post_tonemap, PostTonemap::Off);
    assert_eq!(s.visual_treatment, VisualTreatment::High);
    assert!(VisualStyle::OriginalPc.label().contains("incomplete"));
    assert!(registry::FEATURES.iter().any(|f| f.name == "World materials" && f.coverage == "Partial"));
}
#[test]
fn custom_overrides_survive_a_round_trip_and_quality_selection() {
    let mut s = settings();
    s.select_visual_style(VisualStyle::Enhanced);
    s.post_aa = PostAa::Smaa;
    s.appearance.lighting.sky_strength = 0.41;
    s.appearance.lighting.cached_reflections = true;
    s.appearance.lighting.cached_shadows = true;
    s.appearance.lighting.screen_reflections = true;
    s.appearance.lighting.paint_response = false;
    s.appearance.treatment_strength = Some(0.73);
    s.appearance.lighting.freeze_sun = true;
    s.appearance.lighting.lighting_debug = blackbox_gfx::LightingDebug::Cascades;
    assert_eq!(s.visual_style(), VisualStyle::Custom);
    let text = super::super::write::merge(
        "game_dir='keep-me'
[unrelated]
value=7
",
        &s.appearance_layer(),
    )
    .unwrap();
    assert!(text.contains("keep-me"));
    assert!(text.contains("value = 7"));
    assert!(!text.contains("freeze_sun"));
    assert!(!text.contains("lighting_debug"));
    let mut loaded = Settings::from(super::super::file::parse(&text, "test"));
    assert_eq!(loaded.post_aa, PostAa::Smaa);
    assert_eq!(loaded.appearance.treatment_strength, Some(0.73));
    assert_eq!(loaded.appearance.lighting.sky_strength, 0.41);
    assert!(loaded.appearance.lighting.cached_reflections && loaded.appearance.lighting.screen_reflections);
    assert!(!loaded.appearance.lighting.freeze_sun);
    assert!(loaded.appearance.lighting.cached_shadows);
    loaded.select_graphics_quality(Quality::Balanced);
    assert_eq!(loaded.post_aa, PostAa::Smaa);
    assert_eq!(loaded.appearance.lighting.sky_strength, 0.41);
    assert!(loaded.appearance.lighting.cached_reflections && loaded.appearance.lighting.screen_reflections);
    assert_eq!(loaded.appearance.treatment_strength, Some(0.73));
    assert!((loaded.graphics().post.colour_treatment - 0.73).abs() < 1e-6);
}
#[test]
fn legacy_preferences_load_without_auto_applying_a_style() {
    let text = "post_aa='smaa'
visual_treatment='off'
post_tonemap='off'
reflections='low'
";
    let s = Settings::from(super::super::file::parse(text, "legacy"));
    assert_eq!(s.visual_style(), VisualStyle::Custom);
    assert_eq!(s.post_aa, PostAa::Smaa);
    assert_eq!(s.post_tonemap, PostTonemap::Off);
    assert_eq!(s.reflections, ReflectionQuality::Low);
    assert_eq!(s.appearance.lighting, Appearance::default().lighting);
}
#[test]
fn invalid_and_future_profiles_do_not_destroy_legacy_settings() {
    for block in [
        "version=99
style='enhanced'",
        "version=1
style='bad'",
        "version=1
[appearance.lighting]
shadow_distance='bad'",
    ] {
        let text = format!(
            "post_aa='smaa'
[appearance]
{block}
"
        );
        let layer = super::super::file::parse(&text, "test");
        assert!(layer.appearance.is_none());
        assert_eq!(Settings::from(layer).post_aa, PostAa::Smaa);
    }
    let s = settings();
    assert!(
        super::super::write::merge(
            "[appearance]
version=99
",
            &s.appearance_layer()
        )
        .is_err()
    );
    let text = super::super::write::merge(
        "[appearance]
version=1
future='keep'
[appearance.lighting]
future_light=9
",
        &s.appearance_layer(),
    )
    .unwrap();
    assert!(text.contains("future = "));
    assert!(text.contains("future_light = 9"));
}

#[test]
fn save_payload_keeps_gameplay_values_at_root_when_profile_tables_are_present() {
    let mut s = settings();
    s.select_visual_style(VisualStyle::Enhanced);
    s.game_speed = 1.5;
    s.animation_speed = 0.75;
    s.always_rain = true;
    s.rain_amount = Percent(37);
    let payload = crate::saves::settings_payload(&s).unwrap();
    let restored = crate::saves::settings_from_payload(&payload).unwrap();
    assert_eq!(restored.appearance, s.appearance);
    assert_eq!(restored.game_speed, s.game_speed);
    assert_eq!(restored.animation_speed, s.animation_speed);
    assert_eq!(restored.always_rain, s.always_rain);
    assert_eq!(restored.rain_amount, s.rain_amount);
}

#[test]
fn saving_a_baseline_removes_the_previous_treatment_override() {
    let mut s = settings();
    s.select_visual_style(VisualStyle::Enhanced);
    s.appearance.treatment_strength = Some(0.73);
    let custom = super::super::write::merge("", &s.appearance_layer()).unwrap();
    assert!(custom.contains("treatment_strength"));
    s.select_visual_style(VisualStyle::OriginalPc);
    let reset = super::super::write::merge(&custom, &s.appearance_layer()).unwrap();
    assert!(!reset.contains("treatment_strength"));
    let loaded = Settings::from(super::super::file::parse(&reset, "test"));
    assert_eq!(loaded.appearance.treatment_strength, None);
    assert_eq!(loaded.visual_style(), VisualStyle::OriginalPc);
}

#[test]
fn original_material_selection_persists_and_quality_never_changes_it() {
    let mut s = settings();
    s.select_visual_style(VisualStyle::OriginalPc);
    assert!(s.appearance.lighting.original_world_materials);
    assert_eq!(s.car_shading, CarShading::Original);
    for quality in Quality::ALL {
        s.select_graphics_quality(quality);
        assert!(s.appearance.lighting.original_world_materials);
    }
    let text = super::super::write::merge("", &s.appearance_layer()).unwrap();
    let restored = Settings::from(super::super::file::parse(&text, "test"));
    assert!(restored.appearance.lighting.original_world_materials);
    assert_eq!(restored.appearance.lighting.shadow_projection, Some(super::shadows::PROJECTION));
    s.select_visual_style(VisualStyle::Enhanced);
    assert!(s.appearance.lighting.original_world_materials);
    assert_eq!(s.appearance.lighting.shadow_projection, None);
    assert!(s.appearance.lighting.contact_shadows);
}

#[test]
fn old_material_profiles_migrate_without_losing_other_tuning() {
    let text = "post_aa='smaa'\n[appearance]\nversion=1\nstyle='custom'\n[appearance.lighting]\noriginal_world_materials=false\nshadow_distance=237.0\ncontact_length=0.42\n";
    let s = Settings::from(super::super::file::parse(text, "old profile"));
    assert!(s.appearance.lighting.original_world_materials);
    assert_eq!(s.appearance.lighting.shadow_distance, 237.0);
    assert_eq!(s.appearance.lighting.contact_length, 0.42);
    assert_eq!(s.post_aa, PostAa::Smaa);
}
