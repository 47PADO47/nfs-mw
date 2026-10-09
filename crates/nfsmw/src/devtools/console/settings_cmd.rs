//! `get` and `set`: reading and changing [`Settings`] from the console. Changes last for this run;
//! the config file is not written.

use std::str::FromStr;

use crate::app::pacing::MaxFps;
use crate::devtools::{ShowMetrics, ShowReadout};
use crate::settings::{Percent, Settings, Transmission, parse_bool};

/// Settings the console can show.
const KEYS: [&str; 20] = [
    "backend",
    "vsync",
    "fps",
    "metrics",
    "readout",
    "window_mode",
    "monitor",
    "resolution",
    "volume",
    "music_volume",
    "sfx_volume",
    "engine_volume",
    "hud",
    "tire_smoke",
    "radio",
    "smoke_quality",
    "skid_marks",
    "collision_sparks",
    "speed_trails",
    "transmission",
];

/// The text for `get <key>`, or an error naming the valid keys.
pub fn get(settings: &Settings, key: &str) -> Result<String, String> {
    let value = match key {
        "backend" => settings.backend.to_string(),
        "vsync" => on_off(settings.vsync).to_owned(),
        "fps" | "max_fps" => settings.max_fps.to_string(),
        "metrics" | "show_metrics" => settings.show_metrics.to_string(),
        "window_mode" => settings.window_mode.to_string(),
        "monitor" => settings.monitor.to_string(),
        "resolution" => settings.resolution.to_string(),
        "readout" | "show_readout" => settings.show_readout.to_string(),
        "volume" | "master_volume" => settings.master_volume.to_string(),
        "music_volume" => settings.music_volume.to_string(),
        "sfx_volume" => settings.sfx_volume.to_string(),
        "engine_volume" => settings.engine_volume.to_string(),
        "hud" => on_off(settings.hud).to_owned(),
        "tire_smoke" => on_off(settings.tire_smoke).to_owned(),
        "radio" => on_off(settings.radio).to_owned(),
        "smoke_quality" => settings.smoke_quality.to_string(),
        "skid_marks" => on_off(settings.skid_marks).to_owned(),
        "collision_sparks" => on_off(settings.collision_sparks).to_owned(),
        "speed_trails" => on_off(settings.speed_trails).to_owned(),
        "transmission" => settings.transmission.to_string(),
        other => return Err(unknown(other)),
    };
    Ok(format!("{key} = {value}"))
}

/// Every setting, one per line.
pub fn get_all(settings: &Settings) -> String {
    KEYS.iter().filter_map(|k| get(settings, k).ok()).collect::<Vec<_>>().join("\n")
}

/// Change a setting. The message says what happened.
/// An empty `value` is a `set` without one: a switch flips, anything else answers with its usage.
pub fn set(settings: &mut Settings, key: &str, value: &str) -> Result<String, String> {
    if value.is_empty() {
        return set_without_value(settings, key);
    }
    match key {
        "vsync" => settings.vsync = parse_bool(value)?,
        "fps" | "max_fps" => settings.max_fps = MaxFps::from_str(value)?,
        "metrics" | "show_metrics" => settings.show_metrics = ShowMetrics::from_str(value)?,
        "window_mode" => settings.window_mode = value.parse()?,
        "monitor" => settings.monitor = value.parse()?,
        "resolution" => settings.resolution = value.parse()?,
        "readout" | "show_readout" => settings.show_readout = ShowReadout::from_str(value)?,
        "volume" | "master_volume" => settings.master_volume = Percent::from_str(value)?,
        "music_volume" => settings.music_volume = Percent::from_str(value)?,
        "sfx_volume" => settings.sfx_volume = Percent::from_str(value)?,
        "engine_volume" => settings.engine_volume = Percent::from_str(value)?,
        "hud" => settings.hud = parse_bool(value)?,
        "tire_smoke" => settings.tire_smoke = parse_bool(value)?,
        "radio" => settings.radio = parse_bool(value)?,
        "smoke_quality" => settings.smoke_quality = value.parse()?,
        "skid_marks" => settings.skid_marks = parse_bool(value)?,
        "collision_sparks" => settings.collision_sparks = parse_bool(value)?,
        "speed_trails" => settings.speed_trails = parse_bool(value)?,
        "transmission" => settings.transmission = Transmission::from_str(value)?,
        "backend" => return Err("the graphics backend cannot change while running; restart with --backend".into()),
        other => return Err(unknown(other)),
    }
    get(settings, key)
}

/// The on/off settings, which `set <key>` flips.
fn switch<'a>(settings: &'a mut Settings, key: &str) -> Option<&'a mut bool> {
    match key {
        "vsync" => Some(&mut settings.vsync),
        "hud" => Some(&mut settings.hud),
        "tire_smoke" => Some(&mut settings.tire_smoke),
        "skid_marks" => Some(&mut settings.skid_marks),
        "collision_sparks" => Some(&mut settings.collision_sparks),
        "speed_trails" => Some(&mut settings.speed_trails),
        "radio" => Some(&mut settings.radio),
        _ => None,
    }
}

/// What a setting accepts, for the usage line.
fn syntax(key: &str) -> Option<&'static str> {
    Some(match key {
        "fps" | "max_fps" => "<unlocked|number>",
        "metrics" | "show_metrics" => "<off|basic|advanced>",
        "readout" | "show_readout" => "<off|minimal|full>",
        "window_mode" => "<windowed|borderless|exclusive>",
        "monitor" => "<current|primary|index>",
        "resolution" => "<WIDTHxHEIGHT|native>",
        "volume" | "master_volume" | "music_volume" | "sfx_volume" | "engine_volume" => "<0-100>",
        "smoke_quality" => "<standard|high>",
        "transmission" => "<automatic|manual>",
        _ => return None,
    })
}

fn set_without_value(settings: &mut Settings, key: &str) -> Result<String, String> {
    if let Some(flag) = switch(settings, key) {
        *flag = !*flag;
        return get(settings, key);
    }
    if key == "backend" {
        return Err("the graphics backend cannot change while running; restart with --backend".into());
    }
    let Some(syntax) = syntax(key) else { return Err(unknown(key)) };
    let now = get(settings, key)?;
    let now = now.split_once(" = ").map_or(now.as_str(), |(_, value)| value);
    Err(format!("usage: set {key} {syntax} (now {now})"))
}

fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

fn unknown(key: &str) -> String {
    format!("unknown setting {key:?} (settings: {})", KEYS.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Partial;

    fn defaults() -> Settings {
        Settings::from(Partial::default())
    }

    #[test]
    fn sets_and_reads_back() {
        let mut s = defaults();
        assert_eq!(set(&mut s, "fps", "60").unwrap(), "fps = 60");
        assert_eq!(set(&mut s, "vsync", "off").unwrap(), "vsync = off");
        assert_eq!(set(&mut s, "metrics", "advanced").unwrap(), "metrics = advanced");
        assert_eq!(set(&mut s, "readout", "full").unwrap(), "readout = full");
        assert!(!s.vsync);
        assert_eq!(s.show_metrics, ShowMetrics::Advanced);
        assert_eq!(s.show_readout, ShowReadout::Full);
        assert_eq!(set(&mut s, "fps", "unlocked").unwrap(), "fps = unlocked");
    }

    #[test]
    fn window_preferences_can_change_and_invalid_values_do_not_mutate_them() {
        let mut s = defaults();
        assert_eq!(set(&mut s, "window_mode", "exclusive").unwrap(), "window_mode = exclusive");
        assert_eq!(set(&mut s, "monitor", "0").unwrap(), "monitor = 0");
        assert_eq!(set(&mut s, "resolution", "1920x1080").unwrap(), "resolution = 1920x1080");
        let before = s;
        assert!(set(&mut s, "window_mode", "bad").is_err());
        assert!(set(&mut s, "monitor", "-1").is_err());
        assert!(set(&mut s, "resolution", "0x0").is_err());
        assert_eq!(s, before);
    }

    #[test]
    fn the_transmission_is_set_by_name() {
        let mut s = defaults();
        assert_eq!(get(&s, "transmission").unwrap(), "transmission = automatic");
        assert_eq!(set(&mut s, "transmission", "manual").unwrap(), "transmission = manual");
        assert_eq!(s.transmission, Transmission::Manual);
        assert!(set(&mut s, "transmission", "sport").is_err());
        assert_eq!(s.transmission, Transmission::Manual);
    }

    #[test]
    fn set_without_a_value_flips_a_switch_and_explains_anything_else() {
        let mut s = defaults();
        assert_eq!(set(&mut s, "vsync", "").unwrap(), "vsync = off");
        assert_eq!(set(&mut s, "vsync", "").unwrap(), "vsync = on");
        assert_eq!(set(&mut s, "radio", "").unwrap(), "radio = off");
        assert!(!s.radio);
        let before = s;
        assert_eq!(set(&mut s, "fps", "").unwrap_err(), "usage: set fps <unlocked|number> (now unlocked)");
        assert_eq!(set(&mut s, "metrics", "").unwrap_err(), "usage: set metrics <off|basic|advanced> (now off)");
        assert!(set(&mut s, "volume", "").unwrap_err().starts_with("usage: set volume <0-100> (now 80)"));
        assert!(set(&mut s, "backend", "").unwrap_err().contains("restart"));
        assert!(set(&mut s, "bass", "").unwrap_err().contains("unknown setting"));
        assert_eq!(s, before);
    }

    #[test]
    fn every_setting_has_a_switch_or_a_syntax() {
        let mut s = defaults();
        for key in KEYS.into_iter().filter(|k| *k != "backend") {
            let answer = set(&mut s, key, "");
            assert!(answer.is_ok() || answer.unwrap_err().starts_with("usage: set "), "{key}");
        }
    }

    #[test]
    fn bad_values_and_keys_leave_settings_alone() {
        let mut s = defaults();
        assert!(set(&mut s, "fps", "fast").is_err());
        assert!(set(&mut s, "vsync", "maybe").is_err());
        assert!(set(&mut s, "backend", "gl").unwrap_err().contains("restart"));
        assert!(set(&mut s, "volume", "loud").is_err());
        assert!(set(&mut s, "bass", "11").unwrap_err().contains("unknown setting"));
        assert_eq!(s, defaults());
    }

    #[test]
    fn get_all_lists_every_key() {
        let text = get_all(&defaults());
        assert_eq!(text.lines().count(), KEYS.len());
        assert!(text.contains("vsync = on"));
    }
}
