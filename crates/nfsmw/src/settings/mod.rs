//! Settings, layered: command line > environment > per-user config file > defaults.
//!
//! Each source produces a [`Partial`]; [`Settings::load`] merges them. The in-game settings menu
//! (milestone 6) and the developer console write the config file layer.

mod controls;
#[cfg(test)]
mod controls_tests;
mod env;
#[cfg(test)]
mod exhaust_flames_tests;
mod file;
mod hud_layout;
#[cfg(test)]
mod hud_layout_tests;
mod minimap;
mod partial;
mod smoke_quality;
#[cfg(test)]
mod smoke_quality_tests;
mod spark_style;
#[cfg(test)]
mod tire_tests;
mod transmission;
#[cfg(test)]
mod vehicle_effects_tests;
mod wheel;
mod window;
mod write;

use blackbox_render::Backend;

pub use controls::{Controls, Deadzone, DeadzoneMode, Sensitivity};
pub use hud_layout::HudLayout;
pub use minimap::MinimapMode;
pub use partial::{Partial, Percent, parse_bool};
pub use smoke_quality::SmokeQuality;
pub use spark_style::SparkStyle;
pub use transmission::Transmission;
pub use wheel::WheelOptions;
pub use window::{Monitor, Resolution, WindowMode};
pub use write::write as write_file;

use crate::app::pacing::MaxFps;
use crate::devtools::{ShowMetrics, ShowReadout};

/// The resolved settings.
#[derive(bevy_ecs::resource::Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub controls: Controls,
    pub backend: Backend,
    pub vsync: bool,
    pub max_fps: MaxFps,
    pub show_metrics: ShowMetrics,
    pub window_mode: WindowMode,
    pub monitor: Monitor,
    pub resolution: Resolution,
    /// How much of the scene's debug readout is drawn next to the original HUD.
    pub show_readout: ShowReadout,
    pub master_volume: Percent,
    pub music_volume: Percent,
    pub sfx_volume: Percent,
    pub engine_volume: Percent,
    /// Draw the in-game HUD while driving (the free camera never shows it).
    pub hud: bool,
    /// Draw smoke from the driven car's loaded tire contacts.
    pub tire_smoke: bool,
    /// Play the radio while driving (the `radio` console command still works when it is off).
    pub radio: bool,
    /// Optional smoke presentation quality; standard retains the default cost and look.
    pub smoke_quality: SmokeQuality,
    /// Draw bounded, ground-following tire marks.
    pub skid_marks: bool,
    /// Collision particles, using the selected stock or experimental style.
    pub collision_sparks: bool,
    pub spark_style: SparkStyle,
    /// Experimental wind trails at high speed.
    pub speed_trails: bool,
    /// Flames at the tail pipes: gear-change blow-off and lift-off backfire. Nothing is loaded when off.
    pub exhaust_flames: bool,
    /// Who changes gear: the box (default) or the player.
    pub transmission: Transmission,
    /// How the HUD's minimap is shown: fixed (default), rotating or off.
    pub minimap: MinimapMode,
    /// Placement and scale of the in-game HUD.
    pub hud_layout: HudLayout,
    /// Gamepad button codes of a steering wheel's shift paddles (`GamepadButton::Other`), if the player gave them.
    pub paddle_up: Option<u32>,
    pub paddle_down: Option<u32>,
    /// A clutch pedal is bound (the `clutch` action): while it is pressed the clutch stays open. Off by default.
    pub manual_clutch: bool,
    /// A gear selector that holds a gear (an H-pattern shifter) is bound to the `gear_*` actions. Off by default.
    pub h_shifter: bool,
}

impl From<Partial> for Settings {
    /// Fill what no layer set with the defaults.
    fn from(p: Partial) -> Self {
        Self {
            controls: Controls {
                deadzone_mode: p.deadzone_mode.unwrap_or_default(),
                steering_deadzone: p.steering_deadzone.unwrap_or(Controls::default().steering_deadzone),
                camera_deadzone: p.camera_deadzone.unwrap_or(Controls::default().camera_deadzone),
                trigger_deadzone: p.trigger_deadzone.unwrap_or(Controls::default().trigger_deadzone),
                steering_sensitivity: p.steering_sensitivity.unwrap_or(Controls::default().steering_sensitivity),
                camera_sensitivity: p.camera_sensitivity.unwrap_or(Controls::default().camera_sensitivity),
                mouse_sensitivity: p.mouse_sensitivity.unwrap_or(Controls::default().mouse_sensitivity),
                invert_camera_y: p.invert_camera_y.unwrap_or(Controls::default().invert_camera_y),
            },
            backend: p.backend.unwrap_or_default(),
            vsync: p.vsync.unwrap_or(true),
            max_fps: p.max_fps.unwrap_or_default(),
            show_metrics: p.show_metrics.unwrap_or_default(),
            window_mode: p.window_mode.unwrap_or_default(),
            monitor: p.monitor.unwrap_or_default(),
            resolution: p.resolution.unwrap_or_default(),
            show_readout: p.show_readout.unwrap_or_default(),
            master_volume: p.master_volume.unwrap_or(Percent(80)),
            music_volume: p.music_volume.unwrap_or(Percent(60)),
            sfx_volume: p.sfx_volume.unwrap_or(Percent(90)),
            engine_volume: p.engine_volume.unwrap_or(Percent(90)),
            hud: p.hud.unwrap_or(true),
            tire_smoke: p.tire_smoke.unwrap_or(true),
            radio: p.radio.unwrap_or(true),
            smoke_quality: p.smoke_quality.unwrap_or_default(),
            skid_marks: p.skid_marks.unwrap_or(true),
            collision_sparks: p.collision_sparks.unwrap_or(false),
            spark_style: p.spark_style.unwrap_or_default(),
            speed_trails: p.speed_trails.unwrap_or(false),
            exhaust_flames: p.exhaust_flames.unwrap_or(true),
            transmission: p.transmission.unwrap_or_default(),
            minimap: p.minimap.unwrap_or_default(),
            hud_layout: p.hud_layout.unwrap_or_default(),
            paddle_up: p.paddle_up,
            paddle_down: p.paddle_down,
            manual_clutch: p.manual_clutch.unwrap_or(false),
            h_shifter: p.h_shifter.unwrap_or(false),
        }
    }
}

impl Settings {
    /// What a steering wheel's extra controls are switched on.
    pub fn wheel_options(&self) -> WheelOptions {
        WheelOptions { manual_clutch: self.manual_clutch, h_shifter: self.h_shifter }
    }

    /// The per-user config file (it may not exist yet).
    pub fn config_path() -> Option<std::path::PathBuf> {
        game_install::config_file_path(&nfsmw_data::game::SPEC)
    }

    /// Writes the settings a layer sets to the config file, keeping its other keys.
    pub fn save(changes: &Partial) -> anyhow::Result<std::path::PathBuf> {
        let path = Self::config_path().ok_or_else(|| anyhow::anyhow!("there is no per-user config folder"))?;
        write_file(&path, changes)?;
        Ok(path)
    }

    /// Resolve the settings: `cli` over the process environment over the config file over the defaults.
    pub fn load(cli: Partial) -> Self {
        let env = env::read(|name| std::env::var(name).ok());
        let file =
            game_install::config_file_path(&nfsmw_data::game::SPEC).map(|path| file::read(&path)).unwrap_or_default();
        cli.or(env).or(file).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let s = Settings::from(Partial::default());
        assert_eq!((s.backend, s.vsync, s.max_fps), (Backend::Auto, true, MaxFps::default()));
        assert_eq!(s.transmission, Transmission::Automatic, "automatic, as in the original");
        assert_eq!(s.minimap, MinimapMode::Fixed, "fixed, the original's free roam mode");
        assert_eq!(s.wheel_options(), WheelOptions::default(), "no clutch pedal and no H-shifter unless asked for");
        assert_eq!((s.paddle_up, s.paddle_down), (None, None));
    }

    #[test]
    fn layers_resolve_in_order() {
        let cli = Partial { vsync: Some(false), ..Partial::default() };
        let env = env::read(|n| (n == env::BACKEND).then(|| "gl".to_owned()));
        let file = file::parse("backend = 'vulkan'\nmax_fps = 90\n", "test");
        let s: Settings = cli.or(env).or(file).into();
        assert!(!s.vsync, "command line");
        assert_eq!(s.backend, Backend::Gl, "environment over file");
        assert_eq!(s.max_fps, "90".parse().unwrap(), "file over default");
    }

    #[test]
    fn window_layers_resolve_independently_and_bad_values_fall_through() {
        let cli = Partial { window_mode: Some(WindowMode::Exclusive), ..Partial::default() };
        let env = env::read(|n| match n {
            env::WINDOW_MODE => Some("borderless".into()),
            env::MONITOR => Some("1".into()),
            env::RESOLUTION => Some("bad size".into()),
            _ => None,
        });
        let file = file::parse("window_mode = 'windowed'\nmonitor = 'primary'\nresolution = '1920x1080'", "test");
        let s: Settings = cli.or(env).or(file).into();
        assert_eq!(s.window_mode, WindowMode::Exclusive);
        assert_eq!(s.monitor, Monitor::Index(1));
        assert_eq!(s.resolution, Resolution::pixels(1920, 1080).unwrap());
        let bad = file::parse("window_mode = 'bad'\nmonitor = -1\nresolution = '0x0'", "test");
        assert_eq!(bad, Partial::default());
        assert_eq!(file::parse("monitor = 0", "test").monitor, Some(Monitor::Index(0)));
    }
}
