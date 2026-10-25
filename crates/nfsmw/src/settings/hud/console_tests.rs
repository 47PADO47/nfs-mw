//! The console's `custom_hud.<key>` settings.

use super::console::KEYS;
use super::*;

#[test]
fn every_key_reads_back_what_it_was_set_to() {
    let mut s = HudSettings::default();
    for (key, value) in [
        ("custom_hud.preset", "advanced"),
        ("custom_hud.scale", "1.5"),
        ("custom_hud.opacity", "0.5"),
        ("custom_hud.text_size", "1.2"),
        ("custom_hud.map_mode", "off"),
        ("custom_hud.map_opacity", "0.4"),
        ("custom_hud.map_zoom", "2"),
        ("custom_hud.advanced_telemetry", "off"),
        ("custom_hud.world_markers", "off"),
        ("custom_hud.race_info", "off"),
        ("custom_hud.pursuit_info", "off"),
        ("custom_hud.free_roam_extras", "off"),
        ("custom_hud.notifications", "important"),
        ("custom_hud.high_contrast", "on"),
        ("custom_hud.palette", "deuteranopia"),
        ("custom_hud.motion_reduction", "on"),
        ("custom_hud.camera_shake", "0"),
        ("custom_hud.fov_intensity", "0.5"),
        ("custom_hud.haptics", "off"),
        ("custom_hud.hud_volume", "0.25"),
    ] {
        assert!(is_key(key), "{key}");
        assert_eq!(set(&mut s, key, value), Ok(()), "{key}");
        assert_eq!(get(&s, key), value, "{key}");
    }
    assert_eq!(KEYS.len(), 20);
}

#[test]
fn numbers_are_clamped_and_bad_values_change_nothing() {
    let mut s = HudSettings::default();
    assert_eq!(set(&mut s, "custom_hud.scale", "0.1"), Ok(()));
    assert_eq!(s.scale, 0.75);
    assert_eq!(set(&mut s, "custom_hud.hud_volume", "2"), Ok(()));
    assert_eq!(s.hud_volume, 1.0);
    let before = s;
    assert!(set(&mut s, "custom_hud.scale", "big").is_err());
    assert!(set(&mut s, "custom_hud.fov_intensity", "NaN").is_err());
    assert!(set(&mut s, "custom_hud.haptics", "sometimes").is_err());
    assert!(set(&mut s, "custom_hud.preset", "maximal").is_err());
    assert!(set(&mut s, "custom_hud.unknown", "1").is_err());
    assert_eq!(s, before);
}

#[test]
fn switches_flip_and_the_others_have_a_syntax() {
    let mut s = HudSettings::default();
    for key in KEYS {
        let is_switch = flag(&mut s, key).is_some();
        assert!(is_switch != syntax(key).is_some(), "{key} is either a switch or has a syntax");
    }
    assert!(!is_key("hud") && !is_key("hud.scale"));
}
