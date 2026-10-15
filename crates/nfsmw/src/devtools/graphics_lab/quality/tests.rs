use super::*;
use crate::settings::{Partial, Percent, RenderScale, VisualStyle};

#[test]
fn enhanced_high_matches_the_supplied_reference_controls() {
    let mut settings = Settings::from(Partial::default());
    settings.ao_compare = true;
    settings.post_bloom = PostBloom::High;
    settings.select_visual_style(VisualStyle::Enhanced);
    let options = Quality::High.apply(&mut settings);
    assert!(
        options.sun_shadows
            && options.soft_shadows
            && options.contact_shadows
            && options.clearcoat
            && options.car_skylight
    );
    assert!(!options.far_shadows && !options.world_ambient && !options.original_shadows && !options.freeze_sun);
    assert_eq!(options.lighting_debug, LightingDebug::Off);
    assert_eq!(options.shadow_resolution, 4096);
    assert_eq!(options.shadow_distance, 320.0);
    assert_eq!((options.shadow_filter, options.sun_angle, options.contact_length), (0.7, 0.51, 0.425));
    assert_eq!((options.sky_strength, options.ground_strength), (0.32, 0.55));
    assert_eq!(settings.reflections, ReflectionQuality::High);
    assert_eq!(settings.ao, AoQuality::High);
    assert_eq!(settings.ao_intensity, Percent(100));
    assert!(!settings.ao_compare);
    assert_eq!(settings.post_tonemap, PostTonemap::Aces);
    assert_eq!(settings.post_bloom, PostBloom::Off);
    assert_eq!(settings.post_aa, PostAa::Fxaa);
    assert_eq!(settings.visual_treatment, VisualTreatment::Low);
}

#[test]
fn high_to_low_and_balanced_replaces_all_profile_fields_without_altering_driving() {
    let mut baseline = Settings::from(Partial::default());
    baseline.select_visual_style(VisualStyle::Enhanced);
    for lower in [Quality::Low, Quality::Balanced] {
        let mut direct = baseline;
        let expected = lower.apply(&mut direct);
        let mut after = baseline;
        Quality::High.apply(&mut after);
        after.ao_compare = true;
        let actual = lower.apply(&mut after);
        assert_eq!(actual, expected);
        assert_eq!(after.reflections, direct.reflections);
        assert_eq!(after.ao, direct.ao);
        assert_eq!(after.post_tonemap, direct.post_tonemap);
        assert!(after.ao_compare, "cost selection preserves the active comparison");
        assert_eq!(after.camera_mode, baseline.camera_mode);
        assert_eq!(after.transmission, baseline.transmission);
        assert_eq!(after.render_scale, RenderScale::new(100).unwrap());
    }
}

#[test]
fn startup_request_uses_the_same_profile_as_live_selection_and_runs_once() {
    let mut app = App::new();
    let baseline = Settings::from(Partial::default());
    let mut expected = baseline;
    expected.select_visual_style(VisualStyle::Enhanced);
    let options = Quality::High.apply(&mut expected);
    app.insert_resource(baseline)
        .insert_resource(GraphicsLab { startup_quality: Some(Quality::High), ..Default::default() })
        .add_systems(PreUpdate, startup);
    app.update();
    assert_eq!(app.world().resource::<GraphicsLab>().options, options);
    assert_eq!(app.world().resource::<Settings>().reflections, expected.reflections);
    app.world_mut().resource_mut::<Settings>().post_tonemap = PostTonemap::Off;
    app.update();
    assert_eq!(app.world().resource::<Settings>().post_tonemap, PostTonemap::Off);
}
