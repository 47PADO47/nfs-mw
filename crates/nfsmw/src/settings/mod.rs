//! Settings, layered: command line > environment > per-user config file > defaults.
//!
//! Each source produces a [`Partial`]; [`Settings::load`] merges them. The in-game settings menu
//! (milestone 6) and the developer console write the config file layer.

mod env;
mod file;
mod partial;
mod smoke_quality;
#[cfg(test)]
mod smoke_quality_tests;
#[cfg(test)]
mod tire_tests;
mod transmission;
#[cfg(test)]
mod vehicle_effects_tests;
mod window;
mod write;

use blackbox_render::Backend;

pub use partial::{Partial, Percent, parse_bool};
pub use smoke_quality::SmokeQuality;
pub use transmission::Transmission;
pub use window::{Monitor, Resolution, WindowMode};
pub use write::write as write_file;

use crate::app::pacing::MaxFps;
use crate::devtools::{ShowMetrics, ShowReadout};

/// The resolved settings.
#[derive(bevy_ecs::resource::Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
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
    /// Optional Xenon-style impact and scrape sparks.
    pub collision_sparks: bool,
    /// Optional wind trails behind the car at high forward speed.
    pub speed_trails: bool,
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
            speed_trails: p.speed_trails.unwrap_or(false),
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
}
