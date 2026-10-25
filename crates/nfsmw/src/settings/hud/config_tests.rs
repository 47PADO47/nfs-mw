//! The `[custom_hud]` table: reading, and writing through the config file's per-key merge.

use toml::Table;

use super::*;
use crate::settings::write::merge;
use crate::settings::{Partial, Settings, file};

fn table(text: &str) -> Table {
    text.parse().unwrap()
}

#[test]
fn the_table_is_read() {
    let layer = read(
        &table(
            "[custom_hud]\npreset = 'advanced'\nscale = 1.25\nopacity = 1\ntext_size = 1.1\nmap_mode = 'expanded'\n\
             map_opacity = 0.5\nmap_zoom = 1.5\nadvanced_telemetry = false\nworld_markers = false\nrace_info = false\n\
             pursuit_info = false\nfree_roam_extras = true\nnotifications = 'off'\nhigh_contrast = true\n\
             palette = 'deuteranopia'\nmotion_reduction = true\ncamera_shake = 0.5\nfov_intensity = 0\n\
             haptics = false\nhud_volume = 0.6\n",
        ),
        "test",
    );
    assert_eq!(
        layer,
        HudLayer {
            preset: Some(HudPreset::Advanced),
            scale: Some(1.25),
            opacity: Some(1.0),
            text_size: Some(1.1),
            map_mode: Some(MapMode::Expanded),
            map_opacity: Some(0.5),
            map_zoom: Some(1.5),
            advanced_telemetry: Some(false),
            world_markers: Some(false),
            race_info: Some(false),
            pursuit_info: Some(false),
            free_roam_extras: Some(true),
            notifications: Some(NotificationLevel::Off),
            high_contrast: Some(true),
            palette: Some(HudPalette::Deuteranopia),
            motion_reduction: Some(true),
            camera_shake: Some(0.5),
            fov_intensity: Some(0.0),
            haptics: Some(false),
            hud_volume: Some(0.6),
        }
    );
}

#[test]
fn a_bad_value_drops_only_its_own_key() {
    let layer = read(
        &table("[custom_hud]\nscale = 'big'\nopacity = 0.5\nmap_mode = 'huge'\nhaptics = 1\nrace_info = false\n"),
        "test",
    );
    assert_eq!(
        layer,
        HudLayer { opacity: Some(0.5), race_info: Some(false), ..HudLayer::default() },
        "scale, map_mode and haptics are dropped, the rest still apply"
    );
}

#[test]
fn a_file_without_the_table_changes_nothing() {
    assert_eq!(read(&table("vsync = true\n"), "test"), HudLayer::default());
    assert_eq!(read(&table("custom_hud = 3\n"), "test"), HudLayer::default());
    assert_eq!(Settings::from(file::parse("hud = false\n", "test")).hud_options, HudSettings::default());
}

#[test]
fn the_hud_switch_is_its_own_key() {
    let p = file::parse("hud = false\n[custom_hud]\nmap_mode = 'off'\n", "test");
    assert_eq!((p.hud, p.hud_options.map_mode), (Some(false), Some(MapMode::Off)));
}

#[test]
fn only_the_keys_the_layer_sets_are_written() {
    let mut t = table("vsync = true\n[custom_hud]\nscale = 1.2\n");
    merge_into(&mut t, &HudLayer { map_zoom: Some(1.5), ..HudLayer::default() });
    let section = t["custom_hud"].as_table().unwrap();
    assert_eq!(section.len(), 2);
    assert_eq!(section["scale"].as_float(), Some(1.2), "a key the layer leaves unset is kept");
    assert_eq!(section["map_zoom"].as_float(), Some(1.5));
    assert_eq!(t["vsync"].as_bool(), Some(true));
    let mut untouched = table("vsync = true\n");
    merge_into(&mut untouched, &HudLayer::default());
    assert!(!untouched.contains_key("custom_hud"), "an empty layer writes no table");
}

#[test]
fn the_options_round_trip_through_the_config_merge() {
    let changes = Partial {
        hud: Some(false),
        hud_options: HudLayer {
            preset: Some(HudPreset::Minimal),
            scale: Some(1.25),
            opacity: Some(0.6),
            map_mode: Some(MapMode::Expanded),
            map_opacity: Some(0.35),
            notifications: Some(NotificationLevel::Off),
            palette: Some(HudPalette::Tritanopia),
            high_contrast: Some(true),
            camera_shake: Some(0.3),
            fov_intensity: Some(0.9),
            hud_volume: Some(0.8),
            ..HudLayer::default()
        },
        ..Partial::default()
    };
    let text = merge("game_dir = 'D:/NFS'\nhud = true\n", &changes).unwrap();
    assert!(text.contains("game_dir"));
    assert_eq!(text.parse::<Table>().unwrap()["hud"].as_bool(), Some(false), "the switch stays a plain key");
    assert_eq!(file::parse(&text, "test"), changes);
    let resolved = Settings::from(file::parse(&text, "test"));
    assert!(!resolved.hud);
    assert_eq!(resolved.hud_options.map_mode, MapMode::Expanded, "the player's value beats the preset");
    assert_eq!(resolved.hud_options.notifications, NotificationLevel::Off);
    assert!(!resolved.hud_options.advanced_telemetry, "the preset fills the rest");
}

#[test]
fn every_resolved_option_round_trips() {
    let all = HudSettings::from_layer(HudLayer {
        preset: Some(HudPreset::Advanced),
        palette: Some(HudPalette::Deuteranopia),
        map_zoom: Some(0.75),
        ..HudLayer::default()
    });
    let changes = Partial { hud_options: HudLayer::from(all), ..Partial::default() };
    let text = merge("", &changes).unwrap();
    assert_eq!(Settings::from(file::parse(&text, "test")).hud_options, all);
}
