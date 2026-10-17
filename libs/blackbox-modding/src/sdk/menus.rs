//! `Menus`: rows on the Mods page of the options, each with a Rust closure that runs when the player changes it
//! (catalogue: docs/modding/api.md#menus; the table is `abi::BbMenus`).

use super::ctx::Keep;
use super::ffi::HostTable;
use super::ffi::menus::{self as host_menus, MenusFns, RowHandler, RowSpec};
use super::{Ctx, MenuId, ModResult};
use crate::abi::{BB_MENU_BUTTON, BB_MENU_LIST, BB_MENU_SLIDER, BB_MENU_TOGGLE};

/// The range of a slider: from `min` to `max` in steps of `step`. A step of zero means a tenth of the range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderRange {
    /// The lowest value.
    pub min: f32,
    /// The highest value.
    pub max: f32,
    /// The step between values.
    pub step: f32,
}

/// The `bb.menus` interface of the host, as a mod uses it. Obtained from `Ctx::menus`.
///
/// Each `add` method makes one row and keeps its closure until the mod unloads. The closure gets the row's new value;
/// a button's closure gets nothing. The player's saved value comes back as the row's default on the next start.
pub struct Menus {
    fns: MenusFns,
    host: HostTable,
    keep: Keep,
}

impl Menus {
    pub(crate) fn new(fns: MenusFns, host: HostTable, keep: Keep) -> Self {
        Menus { fns, host, keep }
    }

    /// Add a switch. `handler` gets the new state: on or off.
    pub fn toggle<F>(&self, name: &str, label: &str, default: bool, mut handler: F) -> ModResult<MenuId>
    where
        F: FnMut(&mut Ctx, bool) + 'static,
    {
        let mut spec = row(BB_MENU_TOGGLE, name, label);
        spec.default = f32::from(u8::from(default));
        let on_change: RowHandler = Box::new(move |ctx: &mut Ctx, value: f32| handler(ctx, value >= 0.5));
        host_menus::register(&self.fns, self.host, &self.keep, &spec, on_change)
    }

    /// Add a slider over `range`. `handler` gets the new value.
    pub fn slider<F>(&self, name: &str, label: &str, range: SliderRange, default: f32, handler: F) -> ModResult<MenuId>
    where
        F: FnMut(&mut Ctx, f32) + 'static,
    {
        let mut spec = row(BB_MENU_SLIDER, name, label);
        spec.min = range.min;
        spec.max = range.max;
        spec.step = range.step;
        spec.default = default;
        let on_change: RowHandler = Box::new(handler);
        host_menus::register(&self.fns, self.host, &self.keep, &spec, on_change)
    }

    /// Add a list of `choices`. `handler` gets the index of the chosen text; `default` is an index into `choices`.
    pub fn list<F>(
        &self,
        name: &str,
        label: &str,
        choices: &[&str],
        default: usize,
        mut handler: F,
    ) -> ModResult<MenuId>
    where
        F: FnMut(&mut Ctx, usize) + 'static,
    {
        let mut spec = row(BB_MENU_LIST, name, label);
        spec.choices = choices;
        spec.default = default as f32;
        let on_change: RowHandler = Box::new(move |ctx: &mut Ctx, value: f32| handler(ctx, value.max(0.0) as usize));
        host_menus::register(&self.fns, self.host, &self.keep, &spec, on_change)
    }

    /// Add a button. `handler` runs each time the player presses it.
    pub fn button<F>(&self, name: &str, label: &str, mut handler: F) -> ModResult<MenuId>
    where
        F: FnMut(&mut Ctx) + 'static,
    {
        let spec = row(BB_MENU_BUTTON, name, label);
        let on_change: RowHandler = Box::new(move |ctx: &mut Ctx, _value: f32| handler(ctx));
        host_menus::register(&self.fns, self.host, &self.keep, &spec, on_change)
    }

    /// The value of a row this mod or another added: 0 or 1 for a switch, the number for a slider, the index for a list.
    pub fn get(&self, id: MenuId) -> ModResult<f32> {
        host_menus::get(&self.fns, id)
    }

    /// Set a row's value, as the player's change would: it is saved, and the row's closure does not run.
    pub fn set(&self, id: MenuId, value: f32) -> ModResult<()> {
        host_menus::set(&self.fns, id, value)
    }

    /// Remove a row this mod added.
    pub fn unregister(&self, id: MenuId) -> ModResult<()> {
        host_menus::unregister(&self.fns, id)
    }
}

/// A row with no range, no choices and a zero default, of `kind`. The `add` methods fill in what their kind uses.
fn row<'a>(kind: u32, name: &'a str, label: &'a str) -> RowSpec<'a> {
    RowSpec { kind, name, label, min: 0.0, max: 0.0, step: 0.0, default: 0.0, choices: &[] }
}
