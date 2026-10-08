//! `get` and `set`: reading and changing [`Settings`] from the console. Changes last for this run;
//! the config file is not written.

use std::str::FromStr;

use crate::app::pacing::MaxFps;
use crate::devtools::ShowMetrics;
use crate::settings::{Percent, Settings, parse_bool};

/// Settings the console can show.
const KEYS: [&str; 8] = ["backend", "vsync", "fps", "metrics", "volume", "music_volume", "sfx_volume", "engine_volume"];

/// The text for `get <key>`, or an error naming the valid keys.
pub fn get(settings: &Settings, key: &str) -> Result<String, String> {
    let value = match key {
        "backend" => settings.backend.to_string(),
        "vsync" => on_off(settings.vsync).to_owned(),
        "fps" | "max_fps" => settings.max_fps.to_string(),
        "metrics" | "show_metrics" => settings.show_metrics.to_string(),
        "volume" | "master_volume" => settings.master_volume.to_string(),
        "music_volume" => settings.music_volume.to_string(),
        "sfx_volume" => settings.sfx_volume.to_string(),
        "engine_volume" => settings.engine_volume.to_string(),
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
        "volume" | "master_volume" => settings.master_volume = Percent::from_str(value)?,
        "music_volume" => settings.music_volume = Percent::from_str(value)?,
        "sfx_volume" => settings.sfx_volume = Percent::from_str(value)?,
        "engine_volume" => settings.engine_volume = Percent::from_str(value)?,
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
