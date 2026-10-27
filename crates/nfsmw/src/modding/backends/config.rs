//! `bb.config`: the game's half of a mod's configuration, the table `[mods.<id>]` of the main config file
//! (docs/modding/api-config.md). The host reads the mod's own file and merges the two; this port only says what the main
//! config holds for the mod.
//!
//! The file is read each time a mod asks, which a mod does when it loads. A change the player made to the file, or a save of
//! the settings, is therefore seen by the next mod load or developer reload, and never by a mod that is already running.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use blackbox_modding::abi::BB_ERR_HOST;
use blackbox_modding::host::{BackendResult, ConfigBackend};
use toml::Table;

use crate::settings::Settings;
use crate::settings::mods::section_in;

/// The `bb.config` port of the game. It holds the path of the config file, and reads it on every call.
pub struct ConfigPort {
    pub(super) path: Option<PathBuf>,
}

impl ConfigPort {
    /// The port over the per-user config file.
    pub fn new() -> Self {
        Self { path: Settings::config_path() }
    }

    /// The port over the config file at `path`.
    pub fn at(path: &Path) -> Self {
        Self { path: Some(path.to_owned()) }
    }
}

impl Default for ConfigPort {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigBackend for ConfigPort {
    fn main_section(&self, owner: &str) -> BackendResult<Option<String>> {
        let Some(path) = &self.path else { return Ok(None) };
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                log::warn!("mod {owner}: {} cannot be read, so the mod has no table there: {error}", path.display());
                return Ok(None);
            }
        };
        let table: Table = match text.parse() {
            Ok(table) => table,
            Err(error) => {
                log::warn!("mod {owner}: {} is not valid TOML, so the mod has no table there: {error}", path.display());
                return Ok(None);
            }
        };
        let Some(section) = section_in(&table, owner) else { return Ok(None) };
        toml::to_string(section).map(Some).map_err(|error| {
            log::warn!("mod {owner}: its table in the main config cannot be written out: {error}");
            BB_ERR_HOST
        })
    }
}
