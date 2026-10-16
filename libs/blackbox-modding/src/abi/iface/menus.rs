//! `bb.menus` 1.0: rows a mod adds to the Options page under "Mods" (catalogue: docs/modding/api-world.md#menus).
//!
//! A row is one control: a toggle, a slider, a list of choices or a button. The host keeps each row's value, saves it per
//! mod in the config file, and calls the mod's callback when the player changes it (a button calls it with 0).

use core::ffi::c_void;

use super::BbIface;
use crate::abi::{BbStatus, BbStr};

/// A switch: the value is 0 (off) or 1 (on).
pub const BB_MENU_TOGGLE: u32 = 0;
/// A slider from `min` to `max` in steps of `step` (0 steps in tenths of the range).
pub const BB_MENU_SLIDER: u32 = 1;
/// A list: the value is the index of the chosen text in `choices`.
pub const BB_MENU_LIST: u32 = 2;
/// A button: pressing it calls the callback with 0. It has no value.
pub const BB_MENU_BUTTON: u32 = 3;

/// The description of one row, read by `register`. Its strings and the `choices` array are valid for that call.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbMenuRow {
    /// Bytes the mod filled in.
    pub size: u32,
    /// `BB_MENU_TOGGLE`, `BB_MENU_SLIDER`, `BB_MENU_LIST` or `BB_MENU_BUTTON`.
    pub kind: u32,
    /// The row's name, unique among the mod's rows. The saved value is kept under it.
    pub name: BbStr,
    /// The text the row shows on the page.
    pub label: BbStr,
    /// Slider: the lowest value.
    pub min: f32,
    /// Slider: the highest value.
    pub max: f32,
    /// Slider: the step. Zero means a tenth of the range.
    pub step: f32,
    /// The value before the player or the config file changes it.
    pub default: f32,
    /// List: the texts to choose from, `choice_count` of them.
    pub choices: *const BbStr,
    /// List: how many texts `choices` holds.
    pub choice_count: u64,
}

/// The callback a row calls with its new value (a button calls it with 0). `user` is the mod's pointer.
pub type BbMenuCallbackFn = unsafe extern "C" fn(user: *mut c_void, value: f32);

/// `register`: a row described by `row`, with the mod's `user` pointer and callback. The new row's id goes to `out_id`.
pub type BbMenusRegisterFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    row: *const BbMenuRow,
    user: *mut c_void,
    callback: Option<BbMenuCallbackFn>,
    out_id: *mut u64,
) -> BbStatus;
/// `unregister`: remove one row.
pub type BbMenusUnregisterFn = unsafe extern "C" fn(ctx: *mut c_void, id: u64) -> BbStatus;
/// `get`: write a row's value to `out`.
pub type BbMenusGetFn = unsafe extern "C" fn(ctx: *mut c_void, id: u64, out: *mut f32) -> BbStatus;
/// `set`: set a row's value, saved as the player's change would be. The callback is not called.
pub type BbMenusSetFn = unsafe extern "C" fn(ctx: *mut c_void, id: u64, value: f32) -> BbStatus;

/// The `bb.menus` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbMenus {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Add a row to the Mods page.
    pub register: Option<BbMenusRegisterFn>,
    /// Remove a row.
    pub unregister: Option<BbMenusUnregisterFn>,
    /// Read a row's value.
    pub get: Option<BbMenusGetFn>,
    /// Set a row's value.
    pub set: Option<BbMenusSetFn>,
}
