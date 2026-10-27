//! `bb.config` 1.0: the mod's own configuration as one TOML text (catalogue: docs/modding/api-config.md).
//!
//! A mod's configuration can live in a file of its own folder (`config.toml`, or the file the manifest's `config` key
//! names) and in a table of the game's main config file (`[mods.<id>]`). The host merges the two, the main config winning
//! key by key, and hands the mod the result as TOML text. The mod parses it with its own TOML library, so the SDK has no
//! TOML dependency. `sources` says which of the two exist, so a mod can tell the player what it used.

use core::ffi::c_void;

use super::BbIface;
use crate::abi::{BbSinkFn, BbStatus};

/// `sources` bit: the mod's own config file exists and was read.
pub const BB_CONFIG_FILE: u32 = 1;
/// `sources` bit: the game's main config file has a table for the mod.
pub const BB_CONFIG_MAIN: u32 = 2;

/// `read`: write the merged configuration of the calling mod, as TOML text, to `sink` during the call. The text is empty
/// when neither source exists. A source that is not valid TOML makes the call fail with `BB_ERR_HOST`; the host logs why.
pub type BbConfigReadFn =
    unsafe extern "C" fn(ctx: *mut c_void, sink: Option<BbSinkFn>, sink_user: *mut c_void) -> BbStatus;
/// `sources`: write the `BB_CONFIG_*` bits of the sources that exist for the calling mod to `out`.
pub type BbConfigSourcesFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut u32) -> BbStatus;

/// The `bb.config` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbConfig {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Read the merged configuration text.
    pub read: Option<BbConfigReadFn>,
    /// Which sources exist.
    pub sources: Option<BbConfigSourcesFn>,
}
