//! Settings from the per-user config file.
//!
//! The file is TOML and is shared with `game_dir` (read by `game-install`). Each key is read on its
//! own, so one bad value does not discard the others.

use std::path::Path;
use std::str::FromStr;

use toml::{Table, Value};

use super::partial::{Partial, Percent};
use super::{Deadzone, HudLayout, MinimapMode, RadioHudStyle, Sensitivity, Transmission};
use super::{Monitor, PostAa, PostBloom, PostTonemap, RenderScale, Resolution, SmokeQuality, UpscaleMode, WindowMode};
use crate::app::pacing::MaxFps;
use crate::devtools::{ShowMetrics, ShowReadout};

/// Read the layer from `path`. A missing file is an empty layer; a broken one is reported and ignored.
pub fn read(path: &Path) -> Partial {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text, &path.display().to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Partial::default(),
        Err(e) => {
            log::warn!("cannot read {}: {e}", path.display());
            Partial::default()
        }
    }
}

/// Parse the file's text. `origin` names the file in warnings.
pub fn parse(text: &str, origin: &str) -> Partial {
    let table = match text.parse::<Table>() {
        Ok(t) => t,
        Err(e) => {
            log::warn!("ignoring {origin}: {e}");
            return Partial::default();
        }
    };
    Partial {
        deadzone_mode: field(&table, origin, "deadzone_mode", |v| super::DeadzoneMode::from_str(text_of(v)?)),
        steering_deadzone: field(&table, origin, "steering_deadzone", |v| {
            Deadzone::from_str(&v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string()))
        }),
        camera_deadzone: field(&table, origin, "camera_deadzone", |v| {
            Deadzone::from_str(&v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string()))
        }),
        trigger_deadzone: field(&table, origin, "trigger_deadzone", |v| {
            Deadzone::from_str(&v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string()))
        }),
        steering_sensitivity: field(&table, origin, "steering_sensitivity", |v| {
            Sensitivity::from_str(&v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string()))
        }),
        camera_sensitivity: field(&table, origin, "camera_sensitivity", |v| {
            Sensitivity::from_str(&v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string()))
        }),
        mouse_sensitivity: field(&table, origin, "mouse_sensitivity", |v| {
            Sensitivity::from_str(&v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string()))
        }),
        invert_camera_y: field(&table, origin, "invert_camera_y", boolean),
        backend: field(&table, origin, "backend", |v| text_of(v)?.parse().map_err(|e| format!("{e}"))),
        vsync: field(&table, origin, "vsync", |v| v.as_bool().ok_or_else(|| "expected true or false".to_owned())),
        max_fps: field(&table, origin, "max_fps", |v| match v {
            Value::Integer(n) => MaxFps::from_str(&n.to_string()),
            other => MaxFps::from_str(text_of(other)?),
        }),
        show_metrics: field(&table, origin, "show_metrics", |v| ShowMetrics::from_str(text_of(v)?)),
        window_mode: field(&table, origin, "window_mode", |v| WindowMode::from_str(text_of(v)?)),
        monitor: field(&table, origin, "monitor", |v| match v {
            Value::Integer(n) => Monitor::from_str(&n.to_string()),
            other => Monitor::from_str(text_of(other)?),
        }),
        resolution: field(&table, origin, "resolution", |v| Resolution::from_str(text_of(v)?)),
        show_readout: field(&table, origin, "show_readout", |v| ShowReadout::from_str(text_of(v)?)),
        master_volume: field(&table, origin, "master_volume", percent),
        music_volume: field(&table, origin, "music_volume", percent),
        sfx_volume: field(&table, origin, "sfx_volume", percent),
        engine_volume: field(&table, origin, "engine_volume", percent),
        speech_volume: field(&table, origin, "speech_volume", percent),
        hud: field(&table, origin, "hud", |v| v.as_bool().ok_or_else(|| "expected true or false".to_owned())),
        tire_smoke: field(&table, origin, "tire_smoke", boolean),
        radio: field(&table, origin, "radio", boolean),
        skip_intro: field(&table, origin, "skip_intro", boolean),
        smoke_quality: field(&table, origin, "smoke_quality", |v| SmokeQuality::from_str(text_of(v)?)),
        skid_marks: field(&table, origin, "skid_marks", boolean),
        collision_sparks: field(&table, origin, "collision_sparks", boolean),
        spark_style: field(&table, origin, "spark_style", |v| super::SparkStyle::from_str(text_of(v)?)),
        speed_trails: field(&table, origin, "speed_trails", boolean),
        exhaust_flames: field(&table, origin, "exhaust_flames", boolean),
        transmission: field(&table, origin, "transmission", |v| Transmission::from_str(text_of(v)?)),
        minimap: field(&table, origin, "minimap", |v| MinimapMode::from_str(text_of(v)?)),
        hud_layout: field(&table, origin, "hud_layout", |v| HudLayout::from_str(text_of(v)?)),
        radio_hud: field(&table, origin, "radio_hud", |v| RadioHudStyle::from_str(text_of(v)?)),
        post_tonemap: field(&table, origin, "post_tonemap", |v| PostTonemap::from_str(text_of(v)?)),
        post_bloom: field(&table, origin, "post_bloom", |v| PostBloom::from_str(text_of(v)?)),
        post_aa: field(&table, origin, "post_aa", |v| PostAa::from_str(text_of(v)?)),
        render_scale: field(&table, origin, "render_scale", |v| match v {
            Value::Integer(n) => RenderScale::from_str(&n.to_string()),
            Value::Float(f) => RenderScale::from_str(&format!("{f:?}")),
            other => RenderScale::from_str(text_of(other)?),
        }),
        upscaler: field(&table, origin, "upscaler", |v| UpscaleMode::from_str(text_of(v)?)),
        upscale_sharpness: field(&table, origin, "upscale_sharpness", percent),
        paddle_up: field(&table, origin, "paddle_up", button_code),
        paddle_down: field(&table, origin, "paddle_down", button_code),
        manual_clutch: field(&table, origin, "manual_clutch", boolean),
        h_shifter: field(&table, origin, "h_shifter", boolean),
    }
}

/// Read `key` through `convert`; a value of the wrong kind is reported and ignored.
fn field<T>(table: &Table, origin: &str, key: &str, convert: impl Fn(&Value) -> Result<T, String>) -> Option<T> {
    convert(table.get(key)?).map_err(|e| log::warn!("{origin}: ignoring `{key}`: {e}")).ok()
}

fn boolean(v: &Value) -> Result<bool, String> {
    v.as_bool().ok_or_else(|| "expected true or false".to_owned())
}

/// A volume written as a number (`70`, `0.7`) or a string (`"70%"`).
fn percent(v: &Value) -> Result<Percent, String> {
    match v {
        Value::Integer(n) => Percent::from_str(&n.to_string()),
        Value::Float(f) => Percent::from_str(&format!("{f:?}")),
        other => Percent::from_str(text_of(other)?),
    }
}

/// A gamepad button code: a whole number from 0 up.
fn button_code(v: &Value) -> Result<u32, String> {
    let n = v.as_integer().ok_or_else(|| "expected a button code".to_owned())?;
    u32::try_from(n).map_err(|_| format!("{n} is not a button code"))
}

fn text_of(v: &Value) -> Result<&str, String> {
    v.as_str().ok_or_else(|| "expected a string".to_owned())
}

#[cfg(test)]
mod tests {
    use blackbox_render::Backend;

    use super::*;

    #[test]
    fn reads_values_and_leaves_game_dir_alone() {
        let p = parse("game_dir = 'D:/NFS'\nbackend = 'vulkan'\nvsync = false\nmax_fps = 144\n", "test");
        assert_eq!(p.backend, Some(Backend::Vulkan));
        assert_eq!(p.vsync, Some(false));
        assert_eq!(p.max_fps, Some(MaxFps::from_str("144").unwrap()));
        assert_eq!(parse("max_fps = 'unlocked'", "test").max_fps, Some(MaxFps::default()));
    }

    #[test]
    fn reads_the_skip_intro_switch() {
        assert_eq!(parse("skip_intro = true\n", "test").skip_intro, Some(true));
        assert_eq!(parse("skip_intro = 'yes'\n", "test").skip_intro, None);
    }

    #[test]
    fn reads_volumes_as_numbers_or_text() {
        let p = parse("master_volume = 70\nmusic_volume = 0.25\nsfx_volume = '40%'\nengine_volume = 300\n", "test");
        assert_eq!(
            (p.master_volume, p.music_volume, p.sfx_volume),
            (Some(Percent(70)), Some(Percent(25)), Some(Percent(40)))
        );
        assert_eq!(p.engine_volume, None);
        assert_eq!(parse("speech_volume = 60\n", "test").speech_volume, Some(Percent(60)));
    }

    #[test]
    fn reads_the_readout_level() {
        assert_eq!(parse("show_readout = 'off'", "test").show_readout, Some(ShowReadout::Off));
        assert_eq!(parse("show_readout = 3", "test").show_readout, None);
    }

    #[test]
    fn reads_the_hud_switch() {
        assert_eq!(
            parse(
                "hud = false
",
                "test"
            )
            .hud,
            Some(false)
        );
        assert_eq!(
            parse(
                "hud = 'maybe'
",
                "test"
            )
            .hud,
            None
        );
    }

    #[test]
    fn reads_the_minimap_mode() {
        assert_eq!(parse("minimap = 'rotating'", "test").minimap, Some(MinimapMode::Rotating));
        assert_eq!(parse("minimap = 'off'", "test").minimap, Some(MinimapMode::Off));
        assert_eq!(parse("minimap = true", "test").minimap, None);
    }

    #[test]
    fn reads_the_transmission() {
        assert_eq!(parse("transmission = 'manual'", "test").transmission, Some(Transmission::Manual));
        assert_eq!(parse("transmission = 'automatic'", "test").transmission, Some(Transmission::Automatic));
        assert_eq!(parse("transmission = 3", "test").transmission, None);
        let p = parse(
            "paddle_up = 7
paddle_down = -1",
            "test",
        );
        assert_eq!((p.paddle_up, p.paddle_down), (Some(7), None));
    }

    #[test]
    fn reads_the_wheel_switches() {
        let p = parse("manual_clutch = true\nh_shifter = false", "test");
        assert_eq!((p.manual_clutch, p.h_shifter), (Some(true), Some(false)));
        let bad = parse("manual_clutch = 'yes'\nh_shifter = 1", "test");
        assert_eq!((bad.manual_clutch, bad.h_shifter), (None, None));
    }

    #[test]
    fn a_bad_value_does_not_discard_the_others() {
        let p = parse("backend = 'dx11'\nvsync = false\n", "test");
        assert_eq!(p.backend, None);
        assert_eq!(p.vsync, Some(false));
    }

    #[test]
    fn broken_or_missing_files_are_empty() {
        assert_eq!(parse("this is = = not toml", "test"), Partial::default());
        assert_eq!(read(Path::new("does/not/exist.toml")), Partial::default());
    }
}
