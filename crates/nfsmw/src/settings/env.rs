//! Settings from environment variables.

use std::str::FromStr;

use super::partial::{Partial, Percent, parse_bool};
use super::{Deadzone, Sensitivity, Transmission};
use super::{Monitor, Resolution, SmokeQuality, WindowMode};
use crate::app::pacing::MaxFps;
use crate::devtools::{ShowMetrics, ShowReadout};

pub const BACKEND: &str = "NFSMW_BACKEND";
pub const VSYNC: &str = "NFSMW_VSYNC";
pub const MAX_FPS: &str = "NFSMW_MAX_FPS";
pub const SHOW_METRICS: &str = "NFSMW_SHOW_METRICS";
pub const WINDOW_MODE: &str = "NFSMW_WINDOW_MODE";
pub const MONITOR: &str = "NFSMW_MONITOR";
pub const RESOLUTION: &str = "NFSMW_RESOLUTION";
pub const SHOW_READOUT: &str = "NFSMW_SHOW_READOUT";
pub const MASTER_VOLUME: &str = "NFSMW_MASTER_VOLUME";
pub const MUSIC_VOLUME: &str = "NFSMW_MUSIC_VOLUME";
pub const SFX_VOLUME: &str = "NFSMW_SFX_VOLUME";
pub const ENGINE_VOLUME: &str = "NFSMW_ENGINE_VOLUME";
pub const HUD: &str = "NFSMW_HUD";
pub const TIRE_SMOKE: &str = "NFSMW_TIRE_SMOKE";
pub const RADIO: &str = "NFSMW_RADIO";
pub const TRAFFIC: &str = "NFSMW_TRAFFIC";
pub const COP_SHARE: &str = "NFSMW_COP_SHARE";
pub const TRAFFIC_LIGHTS: &str = "NFSMW_TRAFFIC_LIGHTS";
pub const SMOKE_QUALITY: &str = "NFSMW_SMOKE_QUALITY";
pub const SKID_MARKS: &str = "NFSMW_SKID_MARKS";
pub const TRANSMISSION: &str = "NFSMW_TRANSMISSION";
pub const PADDLE_UP: &str = "NFSMW_PADDLE_UP";
pub const PADDLE_DOWN: &str = "NFSMW_PADDLE_DOWN";

/// Read the layer through `get`, so tests need not touch the process environment. A value that
/// does not parse is reported and ignored.
pub fn read(get: impl Fn(&str) -> Option<String>) -> Partial {
    Partial {
        deadzone_mode: value(&get, "NFSMW_DEADZONE_MODE", super::DeadzoneMode::from_str),
        steering_deadzone: value(&get, "NFSMW_STEERING_DEADZONE", Deadzone::from_str),
        camera_deadzone: value(&get, "NFSMW_CAMERA_DEADZONE", Deadzone::from_str),
        trigger_deadzone: value(&get, "NFSMW_TRIGGER_DEADZONE", Deadzone::from_str),
        steering_sensitivity: value(&get, "NFSMW_STEERING_SENSITIVITY", Sensitivity::from_str),
        camera_sensitivity: value(&get, "NFSMW_CAMERA_SENSITIVITY", Sensitivity::from_str),
        mouse_sensitivity: value(&get, "NFSMW_MOUSE_SENSITIVITY", Sensitivity::from_str),
        invert_camera_y: value(&get, "NFSMW_INVERT_CAMERA_Y", parse_bool),
        backend: value(&get, BACKEND, |s| s.parse().map_err(|e| format!("{e}"))),
        vsync: value(&get, VSYNC, parse_bool),
        max_fps: value(&get, MAX_FPS, MaxFps::from_str),
        show_metrics: value(&get, SHOW_METRICS, ShowMetrics::from_str),
        window_mode: value(&get, WINDOW_MODE, WindowMode::from_str),
        monitor: value(&get, MONITOR, Monitor::from_str),
        resolution: value(&get, RESOLUTION, Resolution::from_str),
        show_readout: value(&get, SHOW_READOUT, ShowReadout::from_str),
        master_volume: value(&get, MASTER_VOLUME, Percent::from_str),
        music_volume: value(&get, MUSIC_VOLUME, Percent::from_str),
        sfx_volume: value(&get, SFX_VOLUME, Percent::from_str),
        engine_volume: value(&get, ENGINE_VOLUME, Percent::from_str),
        hud: value(&get, HUD, parse_bool),
        tire_smoke: value(&get, TIRE_SMOKE, parse_bool),
        radio: value(&get, RADIO, parse_bool),
        traffic: value(&get, TRAFFIC, |s| s.trim().parse::<u32>().map_err(|e| e.to_string())),
        cop_share: value(&get, COP_SHARE, Percent::from_str),
        traffic_lights: value(&get, TRAFFIC_LIGHTS, parse_bool),
        smoke_quality: value(&get, SMOKE_QUALITY, SmokeQuality::from_str),
        skid_marks: value(&get, SKID_MARKS, parse_bool),
        transmission: value(&get, TRANSMISSION, Transmission::from_str),
        paddle_up: value(&get, PADDLE_UP, |s| s.trim().parse::<u32>().map_err(|e| e.to_string())),
        paddle_down: value(&get, PADDLE_DOWN, |s| s.trim().parse::<u32>().map_err(|e| e.to_string())),
    }
}

fn value<T>(get: &impl Fn(&str) -> Option<String>, name: &str, parse: impl Fn(&str) -> Result<T, String>) -> Option<T> {
    let text = get(name)?;
    match parse(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            log::warn!("ignoring {name}: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use blackbox_render::Backend;

    use super::*;

    fn layer(pairs: &[(&str, &str)]) -> Partial {
        let map: HashMap<_, _> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        read(|name| map.get(name).cloned())
    }

    #[test]
    fn reads_every_setting() {
        let p = layer(&[(BACKEND, "dx12"), (VSYNC, "off"), (MAX_FPS, "60"), (SHOW_METRICS, "advanced")]);
        assert_eq!(p.show_metrics, Some(ShowMetrics::Advanced));
        assert_eq!(p.backend, Some(Backend::Dx12));
        assert_eq!(p.vsync, Some(false));
        assert_eq!(p.max_fps, Some("60".parse::<MaxFps>().unwrap()));
    }

    #[test]
    fn reads_the_readout_level() {
        assert_eq!(layer(&[(SHOW_READOUT, "full")]).show_readout, Some(ShowReadout::Full));
        assert_eq!(layer(&[(SHOW_READOUT, "loud")]).show_readout, None);
    }

    #[test]
    fn reads_the_volumes() {
        let p = layer(&[(MASTER_VOLUME, "50"), (MUSIC_VOLUME, "20%"), (SFX_VOLUME, "loud")]);
        assert_eq!((p.master_volume, p.music_volume), (Some(Percent(50)), Some(Percent(20))));
        assert_eq!((p.sfx_volume, p.engine_volume), (None, None));
    }

    #[test]
    fn reads_the_transmission() {
        assert_eq!(layer(&[(TRANSMISSION, "manual")]).transmission, Some(Transmission::Manual));
        assert_eq!(layer(&[(TRANSMISSION, "sport")]).transmission, None);
        let p = layer(&[(PADDLE_UP, "12"), (PADDLE_DOWN, "left")]);
        assert_eq!((p.paddle_up, p.paddle_down), (Some(12), None));
    }

    #[test]
    fn unset_and_invalid_values_stay_unset() {
        assert_eq!(layer(&[]), Partial::default());
        assert_eq!(layer(&[(BACKEND, "dx11"), (VSYNC, "maybe"), (MAX_FPS, "fast")]), Partial::default());
    }
}
