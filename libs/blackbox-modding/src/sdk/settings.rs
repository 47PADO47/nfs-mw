//! `Settings`: options a mod adds to the game's gameplay settings, or values it adds to options the game has
//! (catalogue: docs/modding/api-settings.md; the table is `abi::BbSettings`).

use super::ctx::Keep;
use super::ffi::HostTable;
use super::ffi::settings::{self as host_settings, ChangeHandler, OptionSpec, SettingsFns};
use super::{Ctx, ModResult, SettingId};

/// A value of a list option: its id (what the game saves and what `get` answers) and the text the menu shows.
///
/// The text may be a message id of the mod's own catalogue (`bb.i18n`), shown in the player's language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingValue<'a> {
    /// The value's id: `a-z`, `0-9`, `_`, `.` and `-`.
    pub id: &'a str,
    /// The text the menu shows.
    pub label: &'a str,
}

/// The `bb.settings` interface of the host, as a mod uses it. Obtained from `Ctx::settings`.
///
/// `add_option` has one rule for every option, in the host and in the docs:
///
/// - the game has no option with that id: it is appended to the gameplay settings, with the values given;
/// - the game has it with the same kind: the values are appended after the game's own (a value the option already has is
///   skipped);
/// - the game has it with another kind: nothing changes, the host logs a warning, and the call is `Error::Exists`.
///
/// A list option with one value is not shown in the menu. One with more is.
pub struct Settings {
    fns: SettingsFns,
    host: HostTable,
    keep: Keep,
}

impl Settings {
    pub(crate) fn new(fns: SettingsFns, host: HostTable, keep: Keep) -> Self {
        Settings { fns, host, keep }
    }

    /// Add the list option `id` with `values`, or merge `values` into the game's option of that id. `label` is the row's
    /// text when the game does not have the option yet. The chosen value is read with [`Settings::get`].
    pub fn add_option(&self, id: &str, label: &str, values: &[SettingValue<'_>]) -> ModResult<SettingId> {
        let spec = OptionSpec { toggle: false, id, label, values, default_on: false };
        host_settings::add_option(&self.fns, self.host, &self.keep, &spec, None)
    }

    /// As [`Settings::add_option`], and `handler` runs with the value's id each time the player picks another value.
    pub fn add_option_with<F>(
        &self,
        id: &str,
        label: &str,
        values: &[SettingValue<'_>],
        mut handler: F,
    ) -> ModResult<SettingId>
    where
        F: FnMut(&mut Ctx, &str) + 'static,
    {
        let spec = OptionSpec { toggle: false, id, label, values, default_on: false };
        let on_change: ChangeHandler = Box::new(move |ctx: &mut Ctx, value: &str| handler(ctx, value));
        host_settings::add_option(&self.fns, self.host, &self.keep, &spec, Some(on_change))
    }

    /// Add the on/off option `id`, or find the game's. `handler` runs with the new state.
    pub fn add_toggle<F>(&self, id: &str, label: &str, default: bool, mut handler: F) -> ModResult<SettingId>
    where
        F: FnMut(&mut Ctx, bool) + 'static,
    {
        let spec = OptionSpec { toggle: true, id, label, values: &[], default_on: default };
        let on_change: ChangeHandler = Box::new(move |ctx: &mut Ctx, value: &str| handler(ctx, value == "on"));
        host_settings::add_option(&self.fns, self.host, &self.keep, &spec, Some(on_change))
    }

    /// The chosen value of option `id`: a value's id, or `"on"` or `"off"` for a toggle.
    pub fn get(&self, id: &str) -> ModResult<String> {
        host_settings::get(&self.fns, id)
    }

    /// Take back one `add_option` of this mod: the values it added go from the menu, and so does the option if this mod
    /// made it and nothing is left in it.
    pub fn remove_option(&self, id: SettingId) -> ModResult<()> {
        host_settings::remove_option(&self.fns, id)
    }
}
