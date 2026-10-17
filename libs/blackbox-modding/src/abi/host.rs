//! The host table the loader passes to a mod (ADR 0005 section 1).

use core::ffi::c_void;

use super::{BbIface, BbStr};

/// Log levels for [`BbLogFn`].
pub const BB_LOG_ERROR: u32 = 1;
/// Log level: warning.
pub const BB_LOG_WARN: u32 = 2;
/// Log level: information.
pub const BB_LOG_INFO: u32 = 3;
/// Log level: debug.
pub const BB_LOG_DEBUG: u32 = 4;
/// Log level: trace.
pub const BB_LOG_TRACE: u32 = 5;

/// Host function: write one line to the game log. `ctx` is the host's own pointer, passed back unchanged.
pub type BbLogFn = unsafe extern "C" fn(ctx: *mut c_void, level: u32, msg: BbStr);

/// Host function: find an interface by id. It returns null when the interface is absent, when its major differs
/// from `major`, or when its minor is below `min_minor`. Otherwise it returns a table that starts with a
/// [`BbIface`] header. The host keeps the table valid while the mod is loaded.
pub type BbQueryInterfaceFn =
    unsafe extern "C" fn(ctx: *mut c_void, id: BbStr, major: u16, min_minor: u16) -> *const BbIface;

/// The host table. Built by the host, read by the mod in `bb_mod_create`.
///
/// The header (`size`, `abi_major`, `abi_minor`) is present in every version. The fields after it may be missing
/// when the host is older than the mod; check `size` before reading one.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbHost {
    /// Bytes the host filled in.
    pub size: u32,
    /// The host's ABI major version.
    pub abi_major: u16,
    /// The host's ABI minor version.
    pub abi_minor: u16,
    /// Opaque host context. The first argument of every host function.
    pub ctx: *mut c_void,
    /// Game id, for example the short name of the game.
    pub game_id: BbStr,
    /// Game version string.
    pub game_version: BbStr,
    /// Writes one log line.
    pub log: Option<BbLogFn>,
    /// Finds an interface by id.
    pub query_interface: Option<BbQueryInterfaceFn>,
}
