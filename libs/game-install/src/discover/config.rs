//! `game_dir` in the per-user config file.

use std::path::PathBuf;

use super::{Discovered, Source};
use crate::GameSpec;

/// Per-user config file: `<config dir>/<app_dir>/config.toml`
/// (`%APPDATA%\<app_dir>\config\config.toml` on Windows, `~/.config/<app_dir>/config.toml` on Linux).
pub fn config_file_path(spec: &GameSpec) -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", spec.app_dir).map(|d| d.config_dir().join("config.toml"))
}

#[derive(serde::Deserialize, Default)]
struct ConfigFile {
    game_dir: Option<PathBuf>,
}

pub(super) fn lookup(spec: &GameSpec) -> Option<Discovered> {
    let file = config_file_path(spec)?;
    let text = std::fs::read_to_string(&file).ok()?;
    match toml::from_str::<ConfigFile>(&text) {
        Ok(cfg) => cfg.game_dir.map(|path| Discovered { path, source: Source::ConfigFile(file) }),
        Err(e) => {
            log::warn!("ignoring {}: {e}", file.display());
            None
        }
    }
}
