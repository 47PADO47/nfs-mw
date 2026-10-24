//! The resolved options: defaults, clamping and the presets.

use super::*;
use crate::settings::{Partial, Settings, WorldTuning};

const PRESETS: [HudPreset; 3] = [HudPreset::Minimal, HudPreset::Balanced, HudPreset::Advanced];

#[test]
fn the_defaults_are_the_balanced_hud() {
    let d = HudSettings::default();
    assert_eq!(d.preset, HudPreset::Balanced);
    assert_eq!((d.scale, d.opacity, d.text_size), (1.0, 0.9, 1.0));
    assert_eq!((d.map_mode, d.map_opacity, d.map_zoom), (MapMode::Centered, 0.35, 1.0));
    assert!(!d.advanced_telemetry && !d.free_roam_extras);
    assert!(d.world_markers && d.race_info && d.pursuit_info);
    assert_eq!(d.notifications, NotificationLevel::All);
    assert!(!d.high_contrast && !d.motion_reduction);
    assert_eq!(d.palette, HudPalette::Default);
    assert_eq!((d.camera_shake, d.fov_intensity), (1.0, 1.0));
    assert!(!d.haptics, "rumble only when the player turns it on");
    assert_eq!(d.hud_volume, 0.8);
    assert_eq!(Settings::from(Partial::default()).hud_options, d);
}

#[test]
fn numbers_outside_their_range_are_clamped() {
    let low = HudSettings::from_layer(HudLayer {
        scale: Some(0.1),
        opacity: Some(0.0),
        text_size: Some(0.5),
        map_opacity: Some(0.0),
        map_zoom: Some(0.1),
        camera_shake: Some(-1.0),
        fov_intensity: Some(-0.5),
        hud_volume: Some(-2.0),
        ..HudLayer::default()
    });
    assert_eq!((low.scale, low.opacity, low.text_size), (0.75, 0.2, 0.8));
    assert_eq!((low.map_opacity, low.map_zoom), (0.1, 0.5));
    assert_eq!((low.camera_shake, low.fov_intensity, low.hud_volume), (0.0, 0.0, 0.0));
    let high = HudSettings::from_layer(HudLayer {
        scale: Some(9.0),
        opacity: Some(2.0),
        text_size: Some(3.0),
        map_opacity: Some(1.0),
        map_zoom: Some(5.0),
        camera_shake: Some(4.0),
        fov_intensity: Some(1.5),
        hud_volume: Some(10.0),
        ..HudLayer::default()
    });
    assert_eq!((high.scale, high.opacity, high.text_size), (1.5, 1.0, 1.5));
    assert_eq!((high.map_opacity, high.map_zoom), (0.9, 2.0));
    assert_eq!((high.camera_shake, high.fov_intensity, high.hud_volume), (1.0, 1.0, 1.0));
    let nan = HudSettings::from_layer(HudLayer { scale: Some(f32::NAN), ..HudLayer::default() });
    assert_eq!(nan.scale, DEFAULT_SCALE, "a value that is not a number is the default");
}

#[test]
fn a_preset_fills_what_the_player_left_unset() {
    let minimal = HudSettings::from_layer(HudLayer { preset: Some(HudPreset::Minimal), ..HudLayer::default() });
    assert_eq!(minimal.preset, HudPreset::Minimal);
    assert!(!minimal.advanced_telemetry && !minimal.free_roam_extras);
    assert!(minimal.race_info && minimal.pursuit_info);
    assert_eq!((minimal.notifications, minimal.map_mode), (NotificationLevel::Important, MapMode::Compact));
    let advanced = HudSettings::from_layer(HudLayer { preset: Some(HudPreset::Advanced), ..HudLayer::default() });
    assert!(advanced.advanced_telemetry && advanced.free_roam_extras);
    assert_eq!((advanced.notifications, advanced.map_mode), (NotificationLevel::All, MapMode::Centered));
    let balanced = HudSettings::from_layer(HudLayer { preset: Some(HudPreset::Balanced), ..HudLayer::default() });
    assert_eq!(balanced, HudSettings::default());
}

#[test]
fn a_value_the_player_set_beats_the_preset() {
    let layer = HudLayer {
        preset: Some(HudPreset::Minimal),
        map_mode: Some(MapMode::Expanded),
        advanced_telemetry: Some(true),
        ..HudLayer::default()
    };
    let s = HudSettings::from_layer(layer);
    assert_eq!(s.map_mode, MapMode::Expanded);
    assert!(s.advanced_telemetry);
    assert_eq!(s.notifications, NotificationLevel::Important, "what the player left unset still comes from the preset");
}

#[test]
fn the_preset_sits_under_every_settings_layer() {
    // The command line picks the preset; the config file's own value still beats the preset's.
    let cli = Partial {
        hud_options: HudLayer { preset: Some(HudPreset::Advanced), ..HudLayer::default() },
        ..Partial::default()
    };
    let file = Partial {
        hud_options: HudLayer { notifications: Some(NotificationLevel::Off), ..HudLayer::default() },
        ..Partial::default()
    };
    let s = Settings::from(cli.or(file)).hud_options;
    assert_eq!(s.preset, HudPreset::Advanced);
    assert_eq!(s.notifications, NotificationLevel::Off);
    assert!(s.advanced_telemetry);
}

#[test]
fn applying_a_preset_replaces_only_the_options_it_covers() {
    let mut s = HudSettings::from_layer(HudLayer {
        preset: Some(HudPreset::Advanced),
        scale: Some(1.25),
        palette: Some(HudPalette::Tritanopia),
        ..HudLayer::default()
    });
    s.apply_preset(HudPreset::Minimal);
    assert_eq!(s.preset, HudPreset::Minimal);
    assert_eq!((s.map_mode, s.notifications), (MapMode::Compact, NotificationLevel::Important));
    assert!(!s.advanced_telemetry && !s.free_roam_extras);
    assert_eq!((s.scale, s.palette), (1.25, HudPalette::Tritanopia), "options outside the preset stay");
    s.apply_preset(HudPreset::Balanced);
    assert_eq!(s.map_mode, MapMode::Centered);
    assert_eq!(s.notifications, NotificationLevel::All);
}

#[test]
fn every_preset_covers_the_same_options() {
    let covered = |l: HudLayer| {
        [
            l.advanced_telemetry.is_some(),
            l.free_roam_extras.is_some(),
            l.race_info.is_some(),
            l.pursuit_info.is_some(),
            l.notifications.is_some(),
            l.map_mode.is_some(),
        ]
    };
    for preset in PRESETS {
        assert_eq!(covered(preset.layer()), [true; 6], "{preset}");
        let rest = HudLayer {
            advanced_telemetry: None,
            free_roam_extras: None,
            race_info: None,
            pursuit_info: None,
            notifications: None,
            map_mode: None,
            ..preset.layer()
        };
        assert_eq!(rest, HudLayer::default(), "{preset} sets nothing else");
    }
}

#[test]
fn the_choices_round_trip_by_name_and_reject_others() {
    for v in PRESETS {
        assert_eq!(v.to_string().parse::<HudPreset>(), Ok(v));
    }
    for v in [MapMode::Off, MapMode::Compact, MapMode::Centered, MapMode::Expanded] {
        assert_eq!(v.to_string().parse::<MapMode>(), Ok(v));
    }
    for v in [NotificationLevel::All, NotificationLevel::Important, NotificationLevel::Off] {
        assert_eq!(v.to_string().parse::<NotificationLevel>(), Ok(v));
    }
    for v in [HudPalette::Default, HudPalette::Deuteranopia, HudPalette::Tritanopia] {
        assert_eq!(v.to_string().parse::<HudPalette>(), Ok(v));
    }
    assert!("Minimal".parse::<HudPreset>().is_err());
    assert!("center".parse::<MapMode>().is_err());
    assert!("some".parse::<NotificationLevel>().is_err());
    assert!("protanopia".parse::<HudPalette>().is_err());
}

#[test]
fn the_world_reads_the_camera_multipliers() {
    let mut settings = Settings::from(Partial::default());
    assert_eq!(WorldTuning::from(&settings).camera_shake, 1.0);
    assert_eq!(WorldTuning::default().fov_intensity, 1.0);
    settings.hud_options.camera_shake = 0.0;
    settings.hud_options.fov_intensity = 0.25;
    let tuning = WorldTuning::from(&settings);
    assert_eq!((tuning.camera_shake, tuning.fov_intensity), (0.0, 0.25));
}
