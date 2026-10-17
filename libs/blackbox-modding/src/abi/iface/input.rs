//! `bb.input` 1.0: register a hotkey with a callback.

use core::ffi::c_void;

use super::BbIface;
use crate::abi::{BbStatus, BbStr};

/// The callback of a hotkey. It runs when the hotkey is pressed, on the main thread, inside the frame.
pub type BbHotkeyFn = unsafe extern "C" fn(user: *mut c_void);

/// `register_hotkey`: the action's name and label, its default binding text (`""` for none, `"F5"`, `"Gamepad X"`),
/// the mod's `user` pointer and callback. The new action id is written to `out_id`.
pub type BbInputRegisterFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    name: BbStr,
    label: BbStr,
    default_binding: BbStr,
    user: *mut c_void,
    callback: Option<BbHotkeyFn>,
    out_id: *mut u64,
) -> BbStatus;

/// `unregister`: remove a hotkey by its action id.
pub type BbInputUnregisterFn = unsafe extern "C" fn(ctx: *mut c_void, id: u64) -> BbStatus;

/// The `bb.input` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbInput {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Register a hotkey.
    pub register_hotkey: Option<BbInputRegisterFn>,
    /// Unregister a hotkey.
    pub unregister: Option<BbInputUnregisterFn>,
}
