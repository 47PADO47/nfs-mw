//! Tests for the HUD options page's rows: each steps through its values, clamps or wraps, writes the live settings and
//! records the key for the config file.

use super::super::shown;
use super::*;

fn defaults() -> (Settings, Partial) {
    (Settings::from(Partial::default()), Partial::default())
}

#[test]
fn a_number_steps_on_its_grid_and_stops_at_its_range() {
    let (mut s, mut c) = defaults();
    assert_eq!(HudSetting::Scale.data(&s), Data::Text("100%".into()));
    assert!(HudSetting::Scale.step(&mut s, &mut c, true));
    assert_eq!(s.hud_options.scale, 1.05);
    assert_eq!(c.hud_options.scale, Some(1.05), "the key is recorded for the file");
    for _ in 0..20 {
        HudSetting::Scale.step(&mut s, &mut c, true);
    }
    assert_eq!(s.hud_options.scale, SCALE_RANGE.1, "the top of the range");
    assert!(!HudSetting::Scale.step(&mut s, &mut c, true), "no change past the end");
    for _ in 0..40 {
        HudSetting::Scale.step(&mut s, &mut c, false);
    }
    assert_eq!(s.hud_options.scale, SCALE_RANGE.0, "the bottom of the range");
    for _ in 0..3 {
        HudSetting::MapOpacity.step(&mut s, &mut c, true);
    }
    assert_eq!(s.hud_options.map_opacity, 0.5, "0.35 in steps of 0.05, with no drift");
    assert_eq!(HudSetting::MapOpacity.data(&s), Data::Text("50%".into()));
}

#[test]
fn the_unit_rows_are_sliders_in_tenths() {
    let (mut s, mut c) = defaults();
    assert_eq!(HudSetting::HudVolume.control(&s), Control::Slider(80));
    assert_eq!(HudSetting::Scale.control(&s), Control::Toggle, "the scale is a stepper");
    HudSetting::HudVolume.step(&mut s, &mut c, false);
    assert_eq!((s.hud_options.hud_volume, c.hud_options.hud_volume), (0.7, Some(0.7)));
    for _ in 0..12 {
        HudSetting::CameraShake.step(&mut s, &mut c, false);
    }
    assert_eq!(s.hud_options.camera_shake, 0.0, "0 is a steady camera");
    assert_eq!(HudSetting::CameraShake.control(&s), Control::Slider(0));
}

#[test]
fn a_switch_flips_either_way_and_records_it() {
    let (mut s, mut c) = defaults();
    assert_eq!(HudSetting::HighContrast.data(&s), shown("Off"));
    assert!(HudSetting::HighContrast.step(&mut s, &mut c, true));
    assert!(s.hud_options.high_contrast);
    assert_eq!(c.hud_options.high_contrast, Some(true));
    assert_eq!(HudSetting::HighContrast.data(&s), shown("On"));
    HudSetting::WorldMarkers.step(&mut s, &mut c, false);
    assert_eq!((s.hud_options.world_markers, c.hud_options.world_markers), (false, Some(false)));
}

#[test]
fn picking_a_preset_applies_its_six_options_and_records_them() {
    let (mut s, mut c) = defaults();
    s.hud_options.world_markers = false;
    assert_eq!(HudSetting::Preset.data(&s), shown("Balanced"));
    assert!(HudSetting::Preset.step(&mut s, &mut c, true));
    let h = s.hud_options;
    assert_eq!(h.preset, HudPreset::Advanced);
    assert!(h.advanced_telemetry && h.free_roam_extras);
    assert_eq!(h.map_mode, MapMode::Centered);
    assert!(!h.world_markers, "an option the preset does not cover stays");
    assert_eq!(c.hud_options.preset, Some(HudPreset::Advanced));
    assert_eq!(c.hud_options.advanced_telemetry, Some(true));
    assert_eq!(c.hud_options.world_markers, None, "and is not recorded");
    assert!(Settings::from(c).hud_options.advanced_telemetry, "the file reads back as the page shows");
    HudSetting::Preset.step(&mut s, &mut c, false);
    HudSetting::Preset.step(&mut s, &mut c, false);
    assert_eq!(s.hud_options.preset, HudPreset::Minimal);
    assert_eq!(s.hud_options.notifications, NotificationLevel::Important);
    assert_eq!(s.hud_options.map_mode, MapMode::Compact);
    assert_eq!(c.hud_options.map_mode, Some(MapMode::Compact));
}

#[test]
fn the_choices_cycle_and_wrap() {
    let (mut s, mut c) = defaults();
    HudSetting::MapMode.step(&mut s, &mut c, true);
    assert_eq!(s.hud_options.map_mode, MapMode::Off, "Centered wraps to Off");
    assert_eq!(HudSetting::MapMode.data(&s), shown("Off"));
    assert!(!HudSetting::MapZoom.enabled(&s), "no zoom without a map");
    assert!(HudSetting::MapZoom.enabled(&defaults().0));
    s.hud_options.map_mode = MapMode::Expanded;
    assert_eq!(HudSetting::MapMode.data(&s), shown("Centered"), "the expanded view reads as Centered");
    HudSetting::MapMode.step(&mut s, &mut c, false);
    assert_eq!(s.hud_options.map_mode, MapMode::Compact, "and steps on from there");
    HudSetting::Notifications.step(&mut s, &mut c, false);
    assert_eq!(s.hud_options.notifications, NotificationLevel::Off);
    HudSetting::Palette.step(&mut s, &mut c, true);
    assert_eq!(
        (s.hud_options.palette, c.hud_options.palette),
        (HudPalette::Deuteranopia, Some(HudPalette::Deuteranopia))
    );
}

#[test]
fn the_map_orientation_is_the_minimap_setting() {
    let (mut s, mut c) = defaults();
    s.minimap = MinimapMode::Fixed;
    assert_eq!(HudSetting::MapOrientation.data(&s), shown("North up"));
    HudSetting::MapOrientation.step(&mut s, &mut c, true);
    assert_eq!((s.minimap, c.minimap), (MinimapMode::Rotating, Some(MinimapMode::Rotating)));
    assert_eq!(HudSetting::MapOrientation.data(&s), shown("Heading up"));
    assert_eq!(c.hud_options, HudLayer::default(), "not a key of the table");
}

#[test]
fn every_row_has_a_title_and_a_value() {
    let s = defaults().0;
    for setting in HudSetting::ALL {
        let Title::Key(key) = setting.title() else { panic!("{setting:?} has a catalogue title") };
        assert!(!crate::i18n::text(key, &Args::new()).is_empty(), "{setting:?}");
        assert_ne!(setting.data(&s), Data::Text(String::new()), "{setting:?} reads a value");
    }
}

#[test]
fn the_preview_reads_the_result_in_words() {
    let (mut s, mut c) = defaults();
    s.ui_style = UiStyle::Custom;
    assert_eq!(note(&s), "Centered map, balanced information");
    HudSetting::HighContrast.step(&mut s, &mut c, true);
    assert_eq!(note(&s), "Centered map, balanced information, high contrast");
    HudSetting::Preset.step(&mut s, &mut c, false);
    assert!(note(&s).starts_with("Compact map, minimal information"));
    HudSetting::FreeRoamExtras.step(&mut s, &mut c, true);
    assert!(note(&s).contains("custom information"), "a preset option changed by hand");
    s.ui_style = UiStyle::Original;
    assert!(!note(&s).contains("map,"), "under another style the line says what applies");
}
