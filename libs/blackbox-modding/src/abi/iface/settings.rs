//! `bb.settings` 1.0: options a mod adds to the game's settings screens, or values it adds to options the game has
//! (catalogue: docs/modding/api-settings.md).
//!
//! `add_option` names an option of the gameplay settings by id and gives its kind and, for a list, its values. If the
//! game has no such option, it is added. If the game has it with the same kind, the values are merged in after the
//! game's own. If the kind differs, the call is refused and nothing changes. The host keeps the chosen value, saves it
//! with the other settings, and calls the mod back when the player changes it.

use core::ffi::c_void;

use super::BbIface;
use crate::abi::{BbSinkFn, BbStatus, BbStr};

/// An on/off option. It has no values.
pub const BB_SETTING_TOGGLE: u32 = 0;
/// A list option: the player steps through its values, one is chosen.
pub const BB_SETTING_LIST: u32 = 1;

/// The settings screen an option belongs to: the gameplay options.
pub const BB_SETTING_MENU_GAMEPLAY: u32 = 0;

/// One value of a list option. Its strings are valid for the call that carries it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbSettingValue {
    /// The value's id: plain (`[a-z0-9_.-]`), unique in the option, and what is saved.
    pub id: BbStr,
    /// The text the menu shows for the value, or a message id of the mod's catalogue (`bb.i18n`).
    pub label: BbStr,
}

/// The description of one option, read by `add_option`. Its strings and the `values` array are valid for that call.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbSettingOption {
    /// Bytes the mod filled in.
    pub size: u32,
    /// `BB_SETTING_TOGGLE` or `BB_SETTING_LIST`.
    pub kind: u32,
    /// The screen: `BB_SETTING_MENU_GAMEPLAY`.
    pub menu: u32,
    /// Reserved, zero.
    pub reserved: u32,
    /// The option's id: plain (`[a-z0-9_.-]`). The game's own ids are in the catalogue (`nitrous_variant`).
    pub id: BbStr,
    /// The row's text, used only when the game does not have the option yet.
    pub label: BbStr,
    /// List: the values to add, `value_count` of them.
    pub values: *const BbSettingValue,
    /// List: how many values `values` holds.
    pub value_count: u64,
    /// Toggle: 1 if it starts on. List: unused.
    pub default_on: u32,
    /// Padding, zero.
    pub pad: u32,
}

/// The callback an option calls with its new value: the value's id for a list, `"on"` or `"off"` for a toggle. `user` is
/// the mod's pointer.
pub type BbSettingCallbackFn = unsafe extern "C" fn(user: *mut c_void, value: BbStr);

/// `add_option`: add the option, or merge into the game's. `callback` may be `None`. The id of the mod's registration
/// goes to `out_id`.
pub type BbSettingsAddOptionFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    option: *const BbSettingOption,
    user: *mut c_void,
    callback: Option<BbSettingCallbackFn>,
    out_id: *mut u64,
) -> BbStatus;
/// `remove_option`: take back what one `add_option` added (its values, and the option if the mod made it).
pub type BbSettingsRemoveOptionFn = unsafe extern "C" fn(ctx: *mut c_void, id: u64) -> BbStatus;
/// `get`: write the chosen value of the option `id` to `sink`: the value's id, or `"on"` or `"off"`.
pub type BbSettingsGetFn =
    unsafe extern "C" fn(ctx: *mut c_void, id: BbStr, sink: Option<BbSinkFn>, user: *mut c_void) -> BbStatus;

/// The `bb.settings` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbSettings {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Add an option, or merge values into the game's.
    pub add_option: Option<BbSettingsAddOptionFn>,
    /// Take back one `add_option`.
    pub remove_option: Option<BbSettingsRemoveOptionFn>,
    /// Read an option's chosen value.
    pub get: Option<BbSettingsGetFn>,
}
