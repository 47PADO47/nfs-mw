//! The console's `hud.<key>` settings.

use super::console::KEYS;
use super::*;

#[test]
fn every_key_reads_back_what_it_was_set_to() {
    let mut s = HudSettings::default();
    for (key, value) in [
        ("hud.preset", "advanced"),
        ("hud.scale", "1.5"),
        ("hud.opacity", "0.5"),
        ("hud.text_size", "1.2"),
        ("hud.map_mode", "off"),
        ("hud.map_opacity", "0.4"),
        ("hud.map_zoom", "2"),
        ("hud.advanced_telemetry", "off"),
        ("hud.world_markers", "off"),
        ("hud.race_info", "off"),
        ("hud.pursuit_info", "off"),
        ("hud.free_roam_extras", "off"),
        ("hud.notifications", "important"),
        ("hud.high_contrast", "on"),
        ("hud.palette", "deuteranopia"),
        ("hud.motion_reduction", "on"),
        ("hud.camera_shake", "0"),
        ("hud.fov_intensity", "0.5"),
        ("hud.haptics", "off"),
        ("hud.hud_volume", "0.25"),
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
    assert_eq!(set(&mut s, "hud.scale", "0.1"), Ok(()));
    assert_eq!(s.scale, 0.75);
    assert_eq!(set(&mut s, "hud.hud_volume", "2"), Ok(()));
    assert_eq!(s.hud_volume, 1.0);
    let before = s;
    assert!(set(&mut s, "hud.scale", "big").is_err());
    assert!(set(&mut s, "hud.fov_intensity", "NaN").is_err());
    assert!(set(&mut s, "hud.haptics", "sometimes").is_err());
    assert!(set(&mut s, "hud.preset", "maximal").is_err());
    assert!(set(&mut s, "hud.unknown", "1").is_err());
    assert_eq!(s, before);
}

#[test]
fn switches_flip_and_the_others_have_a_syntax() {
    let mut s = HudSettings::default();
    for key in KEYS {
        let is_switch = flag(&mut s, key).is_some();
        assert!(is_switch != syntax(key).is_some(), "{key} is either a switch or has a syntax");
    }
    assert!(!is_key("hud") && !is_key("hud.enabled"));
}
