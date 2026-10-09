//! Settings, layered: command line > environment > per-user config file > defaults.
//!
//! Each source produces a [`Partial`]; [`Settings::load`] merges them. The in-game settings menu
//! (milestone 6) and the developer console write the config file layer.

mod controls;
#[cfg(test)]
mod controls_tests;
mod env;
mod file;
mod partial;
mod smoke_quality;
#[cfg(test)]
mod smoke_quality_tests;
#[cfg(test)]
mod tire_tests;
mod transmission;
mod window;
mod write;

use blackbox_render::Backend;

pub use controls::{Controls, Deadzone, DeadzoneMode, Sensitivity};
pub use partial::{Partial, Percent, parse_bool};
pub use smoke_quality::SmokeQuality;
pub use transmission::Transmission;
pub use window::{Monitor, Resolution, WindowMode};
pub use write::write as write_file;

use crate::app::pacing::MaxFps;
use crate::devtools::{ShowMetrics, ShowReadout};

/// Computer-driven cars kept on the road by default (the original keeps fewer than 10 vehicles active).
pub const DEFAULT_TRAFFIC: u32 = 10;
/// The default share of those cars that are patrol cops: 5 percent, one in twenty.
pub const DEFAULT_COP_SHARE: Percent = Percent(5);
/// Whether traffic stops at red lights by default. The original game has no traffic lights: this is a rewrite
/// extension, on because it was asked for.
pub const DEFAULT_TRAFFIC_LIGHTS: bool = true;

/// The settings the world's traffic follows, told to the scene together so that it can see any of them change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrafficSettings {
    /// Cars kept on the road (0: none).
    pub cars: u32,
    /// Share of them that are patrol cops, in percent.
    pub cop_share: Percent,
    /// Whether they obey traffic lights.
    pub lights: bool,
}

impl From<&Settings> for TrafficSettings {
    fn from(s: &Settings) -> Self {
        Self { cars: s.traffic, cop_share: s.cop_share, lights: s.traffic_lights }
    }
}

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
    /// How many computer-driven cars the world keeps on the road around you (0 turns traffic off).
    pub traffic: u32,
    /// Of those cars, the share that are patrol cops, in percent.
    pub cop_share: Percent,
    /// Whether the traffic obeys traffic lights at junctions (a rewrite extension: the original has none).
    pub traffic_lights: bool,
    /// Optional smoke presentation quality; standard retains the default cost and look.
    pub smoke_quality: SmokeQuality,
    /// Draw bounded, ground-following tire marks.
    pub skid_marks: bool,
    /// Who changes gear: the box (default) or the player.
    pub transmission: Transmission,
    /// Gamepad button codes of a steering wheel's shift paddles (`GamepadButton::Other`), if the player gave them.
    pub paddle_up: Option<u32>,
    pub paddle_down: Option<u32>,
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
            traffic: p.traffic.unwrap_or(DEFAULT_TRAFFIC),
            cop_share: p.cop_share.unwrap_or(DEFAULT_COP_SHARE),
            traffic_lights: p.traffic_lights.unwrap_or(DEFAULT_TRAFFIC_LIGHTS),
            smoke_quality: p.smoke_quality.unwrap_or_default(),
            skid_marks: p.skid_marks.unwrap_or(true),
            transmission: p.transmission.unwrap_or_default(),
            paddle_up: p.paddle_up,
            paddle_down: p.paddle_down,
        }
    }
}

impl Settings {
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

    #[test]
    fn traffic_is_on_by_default_with_one_cop_in_twenty() {
        let s = Settings::from(Partial::default());
        assert_eq!((s.traffic, s.cop_share), (DEFAULT_TRAFFIC, DEFAULT_COP_SHARE));
        assert!(s.traffic > 0);
        assert_eq!(DEFAULT_COP_SHARE, Percent(5), "5 percent is one in twenty");
    }

    #[test]
    fn traffic_lights_are_on_by_default_and_come_from_every_layer() {
        assert!(Settings::from(Partial::default()).traffic_lights);
        const { assert!(DEFAULT_TRAFFIC_LIGHTS) };
        let cli = Partial { traffic_lights: Some(false), ..Partial::default() };
        let env = env::read(|n| (n == env::TRAFFIC_LIGHTS).then(|| "on".to_owned()));
        let file = file::parse("traffic_lights = false\n", "test");
        assert!(!Settings::from(cli.or(env).or(file)).traffic_lights, "the command line wins");
        assert!(Settings::from(Partial::default().or(env).or(file)).traffic_lights, "the environment over the file");
        assert!(!Settings::from(Partial::default().or(file)).traffic_lights);
        assert_eq!(file::parse("traffic_lights = 3", "test").traffic_lights, None);
    }

    #[test]
    fn the_traffic_settings_come_from_every_layer() {
        let cli = Partial { traffic: Some(0), ..Partial::default() };
        let env = env::read(|n| match n {
            env::TRAFFIC => Some("20".to_owned()),
            env::COP_SHARE => Some("10".to_owned()),
            _ => None,
        });
        let file = file::parse("traffic = 6\ncop_share = 25\n", "test");
        let s: Settings = cli.clone().or(env.clone()).or(file.clone()).into();
        assert_eq!(
            (s.traffic, s.cop_share),
            (0, Percent(10)),
            "the command line turns traffic off, the environment sets the share"
        );
        let s: Settings = Partial::default().or(file).into();
        assert_eq!((s.traffic, s.cop_share), (6, Percent(25)));
        assert_eq!(env::read(|n| (n == env::TRAFFIC).then(|| "many".to_owned())).traffic, None);
        assert_eq!(file::parse("traffic = -3", "test").traffic, None, "a negative count is ignored");
    }
}
