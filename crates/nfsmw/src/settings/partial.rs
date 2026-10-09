//! One layer of settings: every field may be unset.

use blackbox_render::Backend;

use super::{Deadzone, Monitor, Resolution, Sensitivity, SmokeQuality, Transmission, WindowMode};
use crate::app::pacing::MaxFps;
use crate::devtools::{ShowMetrics, ShowReadout};

/// A volume setting in percent, 0 to 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Percent(pub u8);

impl Percent {
    /// As a linear amplitude, 0 to 1.
    pub fn amplitude(self) -> f32 {
        f32::from(self.0) / 100.0
    }
}

impl std::str::FromStr for Percent {
    type Err = String;

    /// `70`, `70%` or `0.7`.
    fn from_str(s: &str) -> Result<Self, String> {
        let text = s.trim().trim_end_matches('%');
        let value: f32 = text.parse().map_err(|_| format!("expected a volume from 0 to 100, got {s:?}"))?;
        let percent = if text.contains('.') && value <= 1.0 { value * 100.0 } else { value };
        if !(0.0..=100.0).contains(&percent) {
            return Err(format!("a volume is 0 to 100, got {s:?}"));
        }
        Ok(Percent(percent.round() as u8))
    }
}

impl std::fmt::Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The settings one source (command line, environment, config file) sets. Unset fields fall
/// through to the next layer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Partial {
    pub deadzone_mode: Option<super::DeadzoneMode>,
    pub steering_deadzone: Option<Deadzone>,
    pub camera_deadzone: Option<Deadzone>,
    pub trigger_deadzone: Option<Deadzone>,
    pub steering_sensitivity: Option<Sensitivity>,
    pub camera_sensitivity: Option<Sensitivity>,
    pub mouse_sensitivity: Option<Sensitivity>,
    pub invert_camera_y: Option<bool>,
    pub backend: Option<Backend>,
    pub vsync: Option<bool>,
    pub max_fps: Option<MaxFps>,
    pub show_metrics: Option<ShowMetrics>,
    pub window_mode: Option<WindowMode>,
    pub monitor: Option<Monitor>,
    pub resolution: Option<Resolution>,
    pub show_readout: Option<ShowReadout>,
    pub master_volume: Option<Percent>,
    pub music_volume: Option<Percent>,
    pub sfx_volume: Option<Percent>,
    pub engine_volume: Option<Percent>,
    pub hud: Option<bool>,
    pub tire_smoke: Option<bool>,
    pub radio: Option<bool>,
    pub smoke_quality: Option<SmokeQuality>,
    pub skid_marks: Option<bool>,
    pub collision_sparks: Option<bool>,
    pub speed_trails: Option<bool>,
    pub transmission: Option<Transmission>,
    pub paddle_up: Option<u32>,
    pub paddle_down: Option<u32>,
}

impl Partial {
    /// This layer, with `lower` filling the fields it leaves unset.
    pub fn or(self, lower: Partial) -> Partial {
        Partial {
            deadzone_mode: self.deadzone_mode.or(lower.deadzone_mode),
            steering_deadzone: self.steering_deadzone.or(lower.steering_deadzone),
            camera_deadzone: self.camera_deadzone.or(lower.camera_deadzone),
            trigger_deadzone: self.trigger_deadzone.or(lower.trigger_deadzone),
            steering_sensitivity: self.steering_sensitivity.or(lower.steering_sensitivity),
            camera_sensitivity: self.camera_sensitivity.or(lower.camera_sensitivity),
            mouse_sensitivity: self.mouse_sensitivity.or(lower.mouse_sensitivity),
            invert_camera_y: self.invert_camera_y.or(lower.invert_camera_y),
            backend: self.backend.or(lower.backend),
            vsync: self.vsync.or(lower.vsync),
            max_fps: self.max_fps.or(lower.max_fps),
            show_metrics: self.show_metrics.or(lower.show_metrics),
            window_mode: self.window_mode.or(lower.window_mode),
            monitor: self.monitor.or(lower.monitor),
            resolution: self.resolution.or(lower.resolution),
            show_readout: self.show_readout.or(lower.show_readout),
            master_volume: self.master_volume.or(lower.master_volume),
            music_volume: self.music_volume.or(lower.music_volume),
            sfx_volume: self.sfx_volume.or(lower.sfx_volume),
            engine_volume: self.engine_volume.or(lower.engine_volume),
            hud: self.hud.or(lower.hud),
            tire_smoke: self.tire_smoke.or(lower.tire_smoke),
            radio: self.radio.or(lower.radio),
            smoke_quality: self.smoke_quality.or(lower.smoke_quality),
            skid_marks: self.skid_marks.or(lower.skid_marks),
            collision_sparks: self.collision_sparks.or(lower.collision_sparks),
            speed_trails: self.speed_trails.or(lower.speed_trails),
            transmission: self.transmission.or(lower.transmission),
            paddle_up: self.paddle_up.or(lower.paddle_up),
            paddle_down: self.paddle_down.or(lower.paddle_down),
        }
    }
}

/// Parse a boolean written as `1/0`, `true/false`, `on/off` or `yes/no`.
pub fn parse_bool(s: &str) -> Result<bool, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Ok(true),
        "0" | "false" | "off" | "no" => Ok(false),
        _ => Err(format!("expected on or off, got {s:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upper_layer_wins_and_gaps_fall_through() {
        let cli = Partial { backend: Some(Backend::Gl), ..Partial::default() };
        let file = Partial { backend: Some(Backend::Vulkan), vsync: Some(false), ..Partial::default() };
        let merged = cli.or(file);
        assert_eq!(merged.backend, Some(Backend::Gl));
        assert_eq!(merged.vsync, Some(false));
        assert_eq!(merged.max_fps, None);
    }

    #[test]
    fn volumes_read_percent_or_fraction() {
        assert_eq!("70".parse(), Ok(Percent(70)));
        assert_eq!("70%".parse(), Ok(Percent(70)));
        assert_eq!("0.5".parse(), Ok(Percent(50)));
        assert_eq!("1".parse(), Ok(Percent(1)));
        assert!("101".parse::<Percent>().is_err());
        assert!("loud".parse::<Percent>().is_err());
    }

    #[test]
    fn booleans() {
        assert_eq!(parse_bool("On"), Ok(true));
        assert_eq!(parse_bool("0"), Ok(false));
        assert!(parse_bool("maybe").is_err());
    }
}
