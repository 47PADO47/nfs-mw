//! Settings, layered: command line > environment > per-user config file > defaults.
//!
//! Each source produces a [`Partial`]; [`Settings::load`] merges them. The in-game settings menu
//! (milestone 6) and the developer console write the config file layer.

mod env;
mod file;
mod partial;

use blackbox_render::Backend;

pub use partial::{Partial, Percent, parse_bool};

use crate::app::pacing::MaxFps;
use crate::devtools::ShowMetrics;

/// The resolved settings.
#[derive(bevy_ecs::resource::Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub backend: Backend,
    pub vsync: bool,
    pub max_fps: MaxFps,
    pub show_metrics: ShowMetrics,
    pub master_volume: Percent,
    pub music_volume: Percent,
    pub sfx_volume: Percent,
    pub engine_volume: Percent,
    /// Draw the in-game HUD while driving (the free camera never shows it).
    pub hud: bool,
}

impl From<Partial> for Settings {
    /// Fill what no layer set with the defaults.
    fn from(p: Partial) -> Self {
        Self {
            backend: p.backend.unwrap_or_default(),
            vsync: p.vsync.unwrap_or(true),
            max_fps: p.max_fps.unwrap_or_default(),
            show_metrics: p.show_metrics.unwrap_or_default(),
            master_volume: p.master_volume.unwrap_or(Percent(80)),
            music_volume: p.music_volume.unwrap_or(Percent(60)),
            sfx_volume: p.sfx_volume.unwrap_or(Percent(90)),
            engine_volume: p.engine_volume.unwrap_or(Percent(90)),
            hud: p.hud.unwrap_or(true),
        }
    }
}

impl Settings {
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
}
