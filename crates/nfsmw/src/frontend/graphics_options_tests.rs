use super::*;
use crate::frontend::options::Setting;
use crate::settings::ReflectionSize;
use blackbox_gfx::reflection::brightness::VehicleBrightness;

#[test]
fn shadows_are_shown_only_on_supported_renderers_and_record_changes() {
    let mut s = Settings::from(Partial::default());
    let mut changed = Partial::default();
    assert!(GraphicsSetting::Shadows.shown(&crate::settings::test_caps::native()));
    assert!(GraphicsSetting::Shadows.shown(&crate::settings::test_caps::full()));
    assert!(GraphicsSetting::Shadows.step(&mut s, &mut changed, true));
    assert_eq!(s.shadows, crate::settings::ShadowQuality::Low);
    assert_eq!(changed.shadows, Some(crate::settings::ShadowQuality::Low));
}

#[test]
fn the_front_end_shadow_row_is_shown_on_bevy_and_toggles_and_records() {
    let (mut s, mut changed) = (Settings::from(Partial::default()), Partial::default());
    assert!(!GraphicsSetting::FeShadows.shown(&crate::settings::test_caps::native()));
    assert!(GraphicsSetting::FeShadows.shown(&crate::settings::test_caps::full()));
    assert_eq!(GraphicsSetting::FeShadows.data(&s), Data::Text("Off".to_owned()));
    assert!(GraphicsSetting::FeShadows.step(&mut s, &mut changed, true));
    assert!(s.fe_shadows.enabled);
    assert_eq!(changed.fe_shadows.enabled, Some(true));
    assert_eq!(GraphicsSetting::FeShadows.data(&s), Data::Text("On".to_owned()));
    assert!(GraphicsSetting::FeShadows.step(&mut s, &mut changed, false));
    assert!(!s.fe_shadows.enabled);
    assert_eq!(changed.fe_shadows.enabled, Some(false));
}

#[test]
fn ao_rows_cycle_clamp_and_persist_independently() {
    let (mut s, mut changed) = (Settings::from(Partial::default()), Partial::default());
    for quality in [AoQuality::Low, AoQuality::Medium, AoQuality::High, AoQuality::Off] {
        GraphicsSetting::AmbientOcclusion.step(&mut s, &mut changed, true);
        assert_eq!((s.ao, changed.ao), (quality, Some(quality)));
    }
    GraphicsSetting::AmbientOcclusion.step(&mut s, &mut changed, false);
    assert_eq!(s.ao, AoQuality::High);
    for _ in 0..12 {
        GraphicsSetting::AoIntensity.step(&mut s, &mut changed, true);
    }
    assert_eq!(
        Setting::Graphics(GraphicsSetting::AoIntensity).control(&s),
        crate::frontend::options::Control::Slider(100)
    );
    assert!(!GraphicsSetting::AoIntensity.step(&mut s, &mut changed, true));
    for _ in 0..12 {
        GraphicsSetting::AoIntensity.step(&mut s, &mut changed, false);
    }
    assert_eq!(changed.ao_intensity, Some(Percent(0)));
    GraphicsSetting::AoComparison.step(&mut s, &mut changed, true);
    assert_eq!((s.ao_compare, changed.ao_compare), (true, Some(true)));
}

#[test]
fn the_preset_row_applies_a_preset_and_records_all_its_keys() {
    let mut s = Settings::from(Partial::default());
    let mut changed = Partial::default();
    assert_eq!(GraphicsSetting::Preset.data(&s), Data::Text("Custom".into()));
    assert!(GraphicsSetting::Preset.step(&mut s, &mut changed, true));
    assert_eq!(GraphicsSetting::Preset.data(&s), Data::Text("Low".into()));
    assert_eq!((s.car_shading, s.render_scale.percent()), (CarShading::Simple, 75));
    assert_eq!((changed.graphics_preset, changed.car_shading), (Some(GraphicsPreset::Low), Some(CarShading::Simple)));
    assert!(changed.render_scale.is_some() && changed.post_aa.is_some());
    assert!(GraphicsSetting::Preset.step(&mut s, &mut changed, false));
    assert_eq!(s.graphics_preset, GraphicsPreset::Custom);
    assert_eq!(s.car_shading, CarShading::Simple, "custom keeps what the last preset set");
}

#[test]
fn the_reflection_rows_step_and_follow_car_reflections() {
    let mut s = Settings::from(Partial::default());
    let mut changed = Partial::default();
    assert_eq!(GraphicsSetting::CubeSize.data(&s), Data::Text("100%".into()));
    assert!(GraphicsSetting::CubeSize.step(&mut s, &mut changed, true));
    assert_eq!(s.reflection_cube_size, ReflectionSize::Scale(1.5));
    assert_eq!(changed.reflection_cube_size, Some(ReflectionSize::Scale(1.5)));
    assert!(GraphicsSetting::MirrorTint.step(&mut s, &mut changed, true));
    assert_eq!(GraphicsSetting::MirrorTint.data(&s), Data::Text("Stock".into()));
    s.reflections = ReflectionQuality::Off;
    assert!(!GraphicsSetting::Pow2.enabled(&s), "no cube capture to round");
    assert!(GraphicsSetting::MirrorSize.enabled(&s), "the mirror does not need car reflections");
}

#[test]
fn the_extra_reflection_rows_cycle_and_record_their_keys() {
    let mut s = Settings::from(Partial::default());
    let mut changed = Partial::default();
    assert_eq!(GraphicsSetting::VehicleBrightness.data(&s), Data::Text("100%".into()));
    assert!(GraphicsSetting::VehicleBrightness.step(&mut s, &mut changed, true));
    assert_eq!(changed.vehicle_reflection_brightness, Some(VehicleBrightness(1.5)));
    assert_eq!(GraphicsSetting::ReflectionDetail.data(&s), Data::Text("Capture".into()));
    assert!(GraphicsSetting::ReflectionDetail.step(&mut s, &mut changed, true));
    assert_eq!(GraphicsSetting::ReflectionDetail.data(&s), Data::Text("Main View".into()));
    assert!(GraphicsSetting::ReflectionContent.step(&mut s, &mut changed, true));
    assert_eq!(changed.reflection_content, Some(ReflectionContent::Restored));
    assert!(GraphicsSetting::ReflectionDistance.step(&mut s, &mut changed, true));
    assert_eq!(GraphicsSetting::ReflectionDistance.data(&s), Data::Text("Original".into()));
    assert_eq!(changed.reflection_distance, Some(ReflectionDistance::Original));
    s.car_shading = CarShading::Simple;
    assert!(!GraphicsSetting::VehicleBrightness.enabled(&s), "simple shading has no glossy material");
    assert!(GraphicsSetting::ReflectionDistance.enabled(&s), "the mirror reach does not need the car cube");
}

#[test]
fn car_shading_toggles_and_records_only_its_change() {
    let mut s = Settings::from(Partial::default());
    let mut changed = Partial::default();
    assert_eq!(GraphicsSetting::CarShading.data(&s), Data::Text("Custom Glossy".into()));
    assert!(GraphicsSetting::CarShading.step(&mut s, &mut changed, true));
    assert_eq!(GraphicsSetting::CarShading.data(&s), Data::Text("Original PC".into()));
    assert_eq!(changed, Partial { car_shading: Some(CarShading::Original), ..Partial::default() });
    assert!(GraphicsSetting::Reflections.enabled(&s));
    assert!(GraphicsSetting::CarShading.step(&mut s, &mut changed, false));
    assert_eq!(s.car_shading, CarShading::Glossy);
}

#[test]
fn changing_a_covered_setting_after_a_preset_makes_the_row_custom() {
    let (mut s, mut changes) = (Settings::from(Partial::default()), Partial::default());
    s.apply_preset(GraphicsPreset::High);
    Setting::TireSmoke.step(&mut s, &mut changes, true);
    assert_eq!(s.graphics_preset, GraphicsPreset::Custom);
    assert_eq!(changes.graphics_preset, Some(GraphicsPreset::Custom));
    let mut changes = Partial::default();
    s.apply_preset(GraphicsPreset::Low);
    Setting::Vsync.step(&mut s, &mut changes, true);
    assert_eq!((s.graphics_preset, changes.graphics_preset), (GraphicsPreset::Low, None), "unrelated rows keep it");
}

use crate::settings::{GraphicsQuality, VisualStyle};
#[test]
fn normal_menu_style_quality_and_shadows_update_the_shared_request() {
    let caps = crate::settings::test_caps::native();
    let mut s = Settings::from(Partial::default());
    let mut changes = Partial::default();
    assert!(GraphicsSetting::Style.step(&mut s, &mut changes, true));
    assert_eq!(s.visual_style(), VisualStyle::OriginalPc);
    assert_eq!(changes.appearance, Some(s.appearance));
    let art = (s.post_aa, s.post_tonemap, s.visual_treatment);
    assert!(GraphicsSetting::Quality.step(&mut s, &mut changes, true));
    assert_eq!(s.appearance.quality, GraphicsQuality::Low);
    assert_eq!((s.post_aa, s.post_tonemap, s.visual_treatment), art);
    assert!(crate::frontend::options::Setting::Graphics(GraphicsSetting::Shadows).advance(
        &mut s,
        &mut changes,
        true,
        &caps
    ));
    assert!(s.appearance.lighting.sun_shadows, "native shadows must reach the active experiment channel");
    assert_eq!(s.visual_style(), VisualStyle::Custom);
}
