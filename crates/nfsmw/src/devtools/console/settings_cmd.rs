//! `get` and `set`: reading and changing [`Settings`] from the console. Changes last for this run;
//! the config file is not written.

use std::str::FromStr;

use crate::app::pacing::MaxFps;
use crate::devtools::ShowMetrics;
use crate::settings::{Percent, Settings, Transmission, parse_bool};

/// Settings the console can show.
const KEYS: [&str; 16] = [
    "backend",
    "vsync",
    "fps",
    "metrics",
    "window_mode",
    "monitor",
    "resolution",
    "volume",
    "music_volume",
    "sfx_volume",
    "engine_volume",
    "hud",
    "tire_smoke",
    "smoke_quality",
    "skid_marks",
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
        "volume" | "master_volume" => settings.master_volume.to_string(),
        "music_volume" => settings.music_volume.to_string(),
        "sfx_volume" => settings.sfx_volume.to_string(),
        "engine_volume" => settings.engine_volume.to_string(),
        "hud" => on_off(settings.hud).to_owned(),
        "tire_smoke" => on_off(settings.tire_smoke).to_owned(),
        "smoke_quality" => settings.smoke_quality.to_string(),
        "skid_marks" => on_off(settings.skid_marks).to_owned(),
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
pub fn set(settings: &mut Settings, key: &str, value: &str) -> Result<String, String> {
    match key {
        "vsync" => settings.vsync = parse_bool(value)?,
        "fps" | "max_fps" => settings.max_fps = MaxFps::from_str(value)?,
        "metrics" | "show_metrics" => settings.show_metrics = ShowMetrics::from_str(value)?,
        "window_mode" => settings.window_mode = value.parse()?,
        "monitor" => settings.monitor = value.parse()?,
        "resolution" => settings.resolution = value.parse()?,
        "volume" | "master_volume" => settings.master_volume = Percent::from_str(value)?,
        "music_volume" => settings.music_volume = Percent::from_str(value)?,
        "sfx_volume" => settings.sfx_volume = Percent::from_str(value)?,
        "engine_volume" => settings.engine_volume = Percent::from_str(value)?,
        "hud" => settings.hud = parse_bool(value)?,
        "tire_smoke" => settings.tire_smoke = parse_bool(value)?,
        "smoke_quality" => settings.smoke_quality = value.parse()?,
        "skid_marks" => settings.skid_marks = parse_bool(value)?,
        "transmission" => settings.transmission = Transmission::from_str(value)?,
        "backend" => return Err("the graphics backend cannot change while running; restart with --backend".into()),
        other => return Err(unknown(other)),
    }
    get(settings, key)
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
        assert!(!s.vsync);
        assert_eq!(s.show_metrics, ShowMetrics::Advanced);
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
