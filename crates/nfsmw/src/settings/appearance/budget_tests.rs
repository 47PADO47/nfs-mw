use super::*;

#[test]
fn overall_and_feature_presets_share_budgets_and_custom_state() {
    let mut s = settings();
    s.select_visual_style(VisualStyle::Enhanced);
    for quality in Quality::ALL {
        s.select_overall_quality(quality);
        assert_eq!(s.overall_quality(), Some(quality));
        assert_eq!(s.reflection_budget(), Some(quality));
        assert_eq!(s.shadow_budget(), Some(quality));
    }
    s.select_overall_quality(Quality::High);
    let lighting = s.appearance.lighting;
    let ao = s.ao;
    s.select_reflection_budget(Some(Quality::Balanced));
    assert_eq!(s.overall_quality(), None);
    assert_eq!(s.reflection_budget(), Some(Quality::Balanced));
    assert_eq!(s.appearance.lighting, lighting);
    assert_eq!(s.ao, ao);
    let reflections = (s.reflections, s.reflection_cube_size, s.reflection_extra);
    s.select_shadow_budget(Some(Quality::Low));
    assert_eq!(s.shadow_budget(), Some(Quality::Low));
    assert_eq!((s.reflections, s.reflection_cube_size, s.reflection_extra), reflections);
    assert_eq!(s.appearance.lighting.sky_strength, lighting.sky_strength);
    s.select_overall_quality(Quality::High);
    assert_eq!(s.overall_quality(), Some(Quality::High));
    s.ao = AoQuality::Off;
    assert_eq!(s.overall_quality(), None);
    s.select_overall_quality(Quality::Balanced);
    assert_eq!(s.ao, AoQuality::Medium);
    assert_eq!(s.overall_quality(), Some(Quality::Balanced));
}

#[test]
fn feature_off_and_round_trip_do_not_reset_other_controls() {
    let mut s = settings();
    s.select_visual_style(VisualStyle::Enhanced);
    s.select_overall_quality(Quality::High);
    let original = s;
    s.select_shadow_budget(None);
    assert!(!s.appearance.lighting.sun_shadows && !s.appearance.lighting.contact_shadows);
    assert_eq!(s.shadows, crate::settings::ShadowQuality::Off);
    assert_eq!(s.reflection_budget(), Some(Quality::High));
    s.select_overall_quality(Quality::High);
    assert_eq!(s, original, "overall quality restores a disabled feature");
    s.select_reflection_budget(Some(Quality::Low));
    let text = super::super::super::write::merge("", &s.appearance_layer()).unwrap();
    let loaded = Settings::from(super::super::super::file::parse(&text, "feature preset"));
    assert_eq!(loaded.reflection_budget(), Some(Quality::Low));
    s.select_reflection_budget(None);
    assert_eq!(s.reflections, blackbox_gfx::ReflectionQuality::Off);
    assert_eq!(s.shadow_budget(), Some(Quality::High));
    assert_eq!(s.appearance.lighting.cached_reflections, original.appearance.lighting.cached_reflections);
}
