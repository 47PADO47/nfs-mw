//! `ModConfig`: the mod's own configuration, merged from its config file and its table in the game's main config
//! (catalogue: docs/modding/api-config.md).
//!
//! The host hands the mod the merged result as TOML text and the SDK does not parse it: the mod uses the TOML library it
//! prefers. Read it once when the mod loads; the text does not change while the mod runs.

use super::ModResult;
use super::ffi::config::{self as host_config, ConfigFns};
use crate::abi::{BB_CONFIG_FILE, BB_CONFIG_MAIN};

/// The `bb.config` interface of the host, as a mod uses it. Obtained from `Ctx::config`.
#[derive(Clone, Copy)]
pub struct ModConfig {
    fns: ConfigFns,
}

impl ModConfig {
    pub(crate) fn new(fns: ConfigFns) -> Self {
        ModConfig { fns }
    }

    /// The merged configuration as TOML text: the mod's own file with the game's table for the mod merged over it, key by
    /// key. Empty when neither exists. `Host` when a source is not valid TOML (the host logs which).
    pub fn text(&self) -> ModResult<String> {
        host_config::text(&self.fns)
    }

    /// Whether the mod's own config file exists (`config.toml`, or the file its manifest names).
    pub fn has_file(&self) -> bool {
        self.bits() & BB_CONFIG_FILE != 0
    }

    /// Whether the game's main config has a table for the mod (NFS:MW: `[mods.<id>]`).
    pub fn has_main_section(&self) -> bool {
        self.bits() & BB_CONFIG_MAIN != 0
    }

    /// The `BB_CONFIG_*` bits of the sources, or 0 when the host cannot say.
    fn bits(&self) -> u32 {
        host_config::sources(&self.fns).unwrap_or(0)
    }
}
