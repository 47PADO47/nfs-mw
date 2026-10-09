//! Writing settings to the per-user config file.
//!
//! The file is shared with `game_dir` (read by `game-install`) and may hold keys this version does not know, so
//! a write reads the file, replaces the keys the layer sets and leaves every other key as it was. Comments and
//! formatting are not kept: the file is written back by the TOML serializer.

use std::path::Path;

use anyhow::{Context, Result};
use toml::{Table, Value};

use super::partial::Partial;

/// The text of the file after `changes` are applied to the table in `existing` (empty text is an empty file).
/// A file that is not valid TOML is an error: it is better left alone than overwritten.
pub fn merge(existing: &str, changes: &Partial) -> Result<String> {
    let mut table: Table = existing.parse().context("the config file is not valid TOML")?;
    let mut put = |key: &str, value: Value| {
        table.insert(key.to_owned(), value);
    };
    if let Some(v) = changes.deadzone_mode {
        put("deadzone_mode", Value::String(v.to_string()));
    }
    if let Some(v) = changes.steering_deadzone {
        put("steering_deadzone", Value::Integer(i64::from(v.percent())));
    }
    if let Some(v) = changes.camera_deadzone {
        put("camera_deadzone", Value::Integer(i64::from(v.percent())));
    }
    if let Some(v) = changes.trigger_deadzone {
        put("trigger_deadzone", Value::Integer(i64::from(v.percent())));
    }
    if let Some(v) = changes.steering_sensitivity {
        put("steering_sensitivity", Value::Integer(i64::from(v.percent())));
    }
    if let Some(v) = changes.camera_sensitivity {
        put("camera_sensitivity", Value::Integer(i64::from(v.percent())));
    }
    if let Some(v) = changes.mouse_sensitivity {
        put("mouse_sensitivity", Value::Integer(i64::from(v.percent())));
    }
    if let Some(v) = changes.invert_camera_y {
        put("invert_camera_y", Value::Boolean(v));
    }
    if let Some(v) = changes.backend {
        put("backend", Value::String(v.to_string()));
    }
    if let Some(v) = changes.vsync {
        put("vsync", Value::Boolean(v));
    }
    if let Some(v) = changes.max_fps {
        put("max_fps", Value::String(v.to_string()));
    }
    if let Some(v) = changes.show_metrics {
        put("show_metrics", Value::String(v.to_string()));
    }
    if let Some(v) = changes.window_mode {
        put("window_mode", Value::String(v.to_string()));
    }
    if let Some(v) = changes.monitor {
        put("monitor", Value::String(v.to_string()));
    }
    if let Some(v) = changes.resolution {
        put("resolution", Value::String(v.to_string()));
    }
    if let Some(v) = changes.show_readout {
        put("show_readout", Value::String(v.to_string()));
    }
    if let Some(v) = changes.master_volume {
        put("master_volume", Value::Integer(i64::from(v.0)));
    }
    if let Some(v) = changes.music_volume {
        put("music_volume", Value::Integer(i64::from(v.0)));
    }
    if let Some(v) = changes.sfx_volume {
        put("sfx_volume", Value::Integer(i64::from(v.0)));
    }
    if let Some(v) = changes.engine_volume {
        put("engine_volume", Value::Integer(i64::from(v.0)));
    }
    if let Some(v) = changes.speech_volume {
        put("speech_volume", Value::Integer(i64::from(v.0)));
    }
    if let Some(v) = changes.hud {
        put("hud", Value::Boolean(v));
    }
    if let Some(v) = changes.tire_smoke {
        put("tire_smoke", Value::Boolean(v));
    }
    if let Some(v) = changes.radio {
        put("radio", Value::Boolean(v));
    }
    if let Some(v) = changes.skid_marks {
        put("skid_marks", Value::Boolean(v));
    }
    if let Some(v) = changes.collision_sparks {
        put("collision_sparks", Value::Boolean(v));
    }
    if let Some(v) = changes.spark_style {
        put("spark_style", Value::String(v.to_string()));
    }
    if let Some(v) = changes.speed_trails {
        put("speed_trails", Value::Boolean(v));
    }
    if let Some(v) = changes.exhaust_flames {
        put("exhaust_flames", Value::Boolean(v));
    }
    if let Some(v) = changes.car_shading {
        put("car_shading", Value::String(v.to_string()));
    }
    if let Some(v) = changes.smoke_quality {
        put("smoke_quality", Value::String(v.to_string()));
    }
    if let Some(v) = changes.transmission {
        put("transmission", Value::String(v.to_string()));
    }
    if let Some(v) = changes.minimap {
        put("minimap", Value::String(v.to_string()));
    }
    if let Some(v) = changes.hud_layout {
        put("hud_layout", Value::String(v.to_string()));
    }
    if let Some(v) = changes.radio_hud {
        put("radio_hud", Value::String(v.to_string()));
    }
    if let Some(v) = changes.post_tonemap {
        put("post_tonemap", Value::String(v.to_string()));
    }
    if let Some(v) = changes.post_bloom {
        put("post_bloom", Value::String(v.to_string()));
    }
    if let Some(v) = changes.post_aa {
        put("post_aa", Value::String(v.to_string()));
    }
    if let Some(v) = changes.render_scale {
        put("render_scale", Value::Integer(i64::from(v.percent())));
    }
    if let Some(v) = changes.upscaler {
        put("upscaler", Value::String(v.to_string()));
    }
    if let Some(v) = changes.upscale_sharpness {
        put("upscale_sharpness", Value::Integer(i64::from(v.0)));
    }
    if let Some(v) = changes.paddle_up {
        put("paddle_up", Value::Integer(i64::from(v)));
    }
    if let Some(v) = changes.paddle_down {
        put("paddle_down", Value::Integer(i64::from(v)));
    }
    if let Some(v) = changes.manual_clutch {
        put("manual_clutch", Value::Boolean(v));
    }
    if let Some(v) = changes.h_shifter {
        put("h_shifter", Value::Boolean(v));
    }
    toml::to_string(&table).context("serializing the config file")
}

/// Applies `changes` to the file at `path`, creating it (and its folder) if needed.
pub fn write(path: &Path, changes: &Partial) -> Result<()> {
    let existing = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    let text = merge(&existing, changes).with_context(|| path.display().to_string())?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::app::pacing::MaxFps;
    use crate::devtools::{ShowMetrics, ShowReadout};
    use crate::settings::{Percent, file};

    #[test]
    fn tire_toggles_round_trip() {
        let changes = Partial {
            tire_smoke: Some(false),
            skid_marks: Some(true),
            smoke_quality: Some(crate::settings::SmokeQuality::High),
            ..Partial::default()
        };
        assert_eq!(file::parse(&merge("", &changes).unwrap(), "test"), changes);
    }

    #[test]
    fn display_settings_round_trip_without_dropping_other_keys() {
        use crate::settings::{Monitor, Resolution, WindowMode};
        let changes = Partial {
            window_mode: Some(WindowMode::Exclusive),
            monitor: Some(Monitor::Index(2)),
            resolution: Some(Resolution::pixels(1920, 1080).unwrap()),
            ..Partial::default()
        };
        let text = merge("future_option = 3", &changes).unwrap();
        assert_eq!(file::parse(&text, "test"), changes);
        assert!(text.contains("future_option = 3"));
    }

    #[test]
    fn keeps_game_dir_and_unknown_keys_and_replaces_the_changed_ones() {
        let before = "game_dir = 'D:/NFS'\nvsync = true\nfuture_option = 3\n";
        let changes = Partial { vsync: Some(false), master_volume: Some(Percent(40)), ..Partial::default() };
        let after = merge(before, &changes).unwrap();
        let table: Table = after.parse().unwrap();
        assert_eq!(table["game_dir"].as_str(), Some("D:/NFS"));
        assert_eq!(table["future_option"].as_integer(), Some(3));
        assert_eq!(table["vsync"].as_bool(), Some(false));
        assert_eq!(table["master_volume"].as_integer(), Some(40));
    }

    #[test]
    fn what_is_written_reads_back() {
        let changes = Partial {
            vsync: Some(false),
            max_fps: Some(MaxFps::from_str("120").unwrap()),
            show_metrics: Some(ShowMetrics::Advanced),
            show_readout: Some(ShowReadout::Full),
            master_volume: Some(Percent(10)),
            music_volume: Some(Percent(20)),
            sfx_volume: Some(Percent(30)),
            engine_volume: Some(Percent(40)),
            speech_volume: Some(Percent(50)),
            hud: Some(false),
            radio: Some(false),
            transmission: Some(crate::settings::Transmission::Manual),
            minimap: Some(crate::settings::MinimapMode::Rotating),
            paddle_up: Some(5),
            paddle_down: Some(6),
            manual_clutch: Some(true),
            h_shifter: Some(true),
            ..Partial::default()
        };
        let text = merge("", &changes).unwrap();
        assert_eq!(file::parse(&text, "test"), changes);
    }

    #[test]
    fn an_unchanged_layer_leaves_the_file_as_data() {
        let before = "game_dir = 'D:/NFS'\n";
        let after = merge(before, &Partial::default()).unwrap();
        assert_eq!(after.parse::<Table>().unwrap(), before.parse::<Table>().unwrap());
    }

    #[test]
    fn a_broken_file_is_not_overwritten() {
        assert!(merge("this is = = not toml", &Partial::default()).is_err());
    }

    #[test]
    fn writes_a_new_file_in_a_new_folder() {
        let dir = std::env::temp_dir().join(format!("nfsmw-settings-test-{}", std::process::id()));
        let path = dir.join("nested").join("config.toml");
        write(&path, &Partial { hud: Some(false), ..Partial::default() }).unwrap();
        assert_eq!(file::read(&path).hud, Some(false));
        write(&path, &Partial { vsync: Some(false), ..Partial::default() }).unwrap();
        let both = file::read(&path);
        assert_eq!((both.hud, both.vsync), (Some(false), Some(false)));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
