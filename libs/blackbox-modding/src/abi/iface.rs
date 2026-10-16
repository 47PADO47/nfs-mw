//! The header every interface table starts with, and the ids and versions of the generic interfaces (ADR 0005
//! sections 1 and 8).
//!
//! An interface table is a `repr(C)` struct: `head: BbIface` first, then `ctx`, then the function pointers. `ctx` is
//! the host's own pointer for that table, and every function takes it as its first argument. A function pointer is
//! `Option<fn>`; `None`, or a field the table's `head.size` does not cover, means "not supported".

mod commands;
mod input;
mod map;
mod menus;
mod time;

#[cfg(test)]
mod tests;

pub use commands::{BbCommandCallbackFn, BbCommands, BbCommandsRegisterFn, BbCommandsRunFn, BbCommandsUnregisterFn};
pub use input::{BbHotkeyFn, BbInput, BbInputRegisterFn, BbInputUnregisterFn};
pub use map::{BbBlip, BbMap, BbMapAddFn, BbMapRemoveFn, BbMapUpdateFn, BbRgba};
pub use menus::{
    BB_MENU_BUTTON, BB_MENU_LIST, BB_MENU_SLIDER, BB_MENU_TOGGLE, BbMenuCallbackFn, BbMenuRow, BbMenus, BbMenusGetFn,
    BbMenusRegisterFn, BbMenusSetFn, BbMenusUnregisterFn,
};
pub use time::{BbTime, BbTimeFreezeFn, BbTimeGetFn, BbTimeOfDay, BbTimeSetDayLengthFn, BbTimeSetHourFn};

/// The header of an interface table. Its `size` says how many bytes the host filled in, and `major` and `minor` are
/// the interface's own version (for example `bb.map 1.2`), separate from the ABI version.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbIface {
    /// Bytes the table holds.
    pub size: u32,
    /// Interface major version.
    pub major: u16,
    /// Interface minor version.
    pub minor: u16,
}

/// Id of the command interface: run console lines and register mod commands.
pub const IFACE_COMMANDS: &str = "bb.commands";
/// The version the host implements for [`IFACE_COMMANDS`], as `(major, minor)`.
pub const IFACE_COMMANDS_VERSION: (u16, u16) = (1, 0);

/// Id of the input interface: register hotkeys.
pub const IFACE_INPUT: &str = "bb.input";
/// The version the host implements for [`IFACE_INPUT`], as `(major, minor)`.
pub const IFACE_INPUT_VERSION: (u16, u16) = (1, 0);

/// Id of the time-of-day interface: read and set the world clock.
pub const IFACE_TIME: &str = "bb.time";
/// The version the host implements for [`IFACE_TIME`], as `(major, minor)`.
pub const IFACE_TIME_VERSION: (u16, u16) = (1, 0);

/// Id of the map interface: add, move and remove blips.
pub const IFACE_MAP: &str = "bb.map";
/// The version the host implements for [`IFACE_MAP`], as `(major, minor)`.
pub const IFACE_MAP_VERSION: (u16, u16) = (1, 0);

/// Id of the menus interface: rows on the Mods page of the options.
pub const IFACE_MENUS: &str = "bb.menus";
/// The version the host implements for [`IFACE_MENUS`], as `(major, minor)`.
pub const IFACE_MENUS_VERSION: (u16, u16) = (1, 0);
