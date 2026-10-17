//! `bb.hud` 1.0: a centred message for a few seconds, and hiding it (catalogue: docs/modding/api-world.md#hud).

use core::ffi::c_void;

use super::BbIface;
use crate::abi::{BbStatus, BbStr};

/// A plain message, in the game's text colour.
pub const BB_HUD_INFO: u32 = 0;
/// A warning: drawn in the warning colour.
pub const BB_HUD_WARNING: u32 = 1;
/// A large message, for an event the player must see.
pub const BB_HUD_BIG: u32 = 2;

/// `message`: show `text` for `secs` seconds in `style` (one of the `BB_HUD_*` values). A newer message replaces it.
pub type BbHudMessageFn = unsafe extern "C" fn(ctx: *mut c_void, text: BbStr, style: u32, secs: f32) -> BbStatus;
/// `hide`: take the message down now.
pub type BbHudHideFn = unsafe extern "C" fn(ctx: *mut c_void) -> BbStatus;

/// The `bb.hud` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbHud {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Show a message.
    pub message: Option<BbHudMessageFn>,
    /// Hide the message.
    pub hide: Option<BbHudHideFn>,
}
