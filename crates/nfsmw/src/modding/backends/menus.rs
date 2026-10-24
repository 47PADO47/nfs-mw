//! `bb.menus`: rows a mod adds to the Mods page of the options.
//!
//! A row keeps its value here. A player's change (through [`change`]) or the mod's own `set` makes the value, saves it
//! under the mod in the config (`menu_store`), and a change calls the mod's callback. A value saved by an earlier run is
//! the one a row registers with, if it still fits the row's kind.

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_EXISTS, BB_ERR_NOT_FOUND};
use blackbox_modding::host::{BackendResult, MenuHandler, MenuKind, MenuRowSpec, MenusBackend};
use toml::Value;

use super::bridge::Bridge;
use super::is_plain_name;
use super::menu_store::ValueStore;

/// The `bb.menus` port of the game: one per mod table, sharing the bridge.
pub struct MenusPort(pub(super) Rc<Bridge>);

/// A row a mod registered.
pub struct MenuRow {
    pub handle: u64,
    pub owner: String,
    pub name: String,
    pub label: String,
    pub kind: MenuKind,
    pub value: f32,
    /// The mod's callback. Shared, so a change can run it while the rows are free to change.
    pub on_change: Rc<MenuHandler>,
}

/// A row as the Mods page shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct RowInfo {
    pub handle: u64,
    pub label: String,
    pub kind: MenuKind,
    pub value: f32,
}

/// The value a row of `kind` takes for `value`: a toggle is 0 or 1, a slider is within its range (a step snaps it to
/// the grid), a list is one of its indices, and a button has none (0).
pub fn fit(kind: &MenuKind, value: f32) -> f32 {
    match kind {
        MenuKind::Toggle => f32::from(u8::from(value >= 0.5)),
        MenuKind::Slider { min, max, step } => {
            let clamped = value.clamp(*min, *max);
            match *step > 0.0 {
                true => (min + ((clamped - min) / step).round() * step).clamp(*min, *max),
                false => clamped,
            }
        }
        MenuKind::List { choices } => value.round().clamp(0.0, choices.len().saturating_sub(1) as f32),
        MenuKind::Button => 0.0,
    }
}

/// The value after one step forward or back, or `None` when it cannot move (a slider at its end). A button's step is
/// its press, so it always yields 0.
fn step(kind: &MenuKind, value: f32, forward: bool) -> Option<f32> {
    match kind {
        MenuKind::Button => Some(0.0),
        MenuKind::Toggle => Some(fit(kind, 1.0 - value)),
        MenuKind::Slider { min, max, step } => {
            let by = match *step > 0.0 {
                true => *step,
                false => (max - min) / 10.0,
            };
            let next = fit(
                kind,
                match forward {
                    true => value + by,
                    false => value - by,
                },
            );
            (next != value).then_some(next)
        }
        MenuKind::List { choices } => {
            let count = choices.len().max(1);
            let index = value.round() as usize;
            let next = match forward {
                true => (index + 1) % count,
                false => (index + count - 1) % count,
            };
            Some(next as f32)
        }
    }
}

/// The value as the config file keeps it: a boolean for a toggle, an integer for a list, a float for a slider.
fn saved_value(kind: &MenuKind, value: f32) -> Value {
    match kind {
        MenuKind::Toggle => Value::Boolean(value >= 0.5),
        MenuKind::List { .. } => Value::Integer(value as i64),
        MenuKind::Slider { .. } | MenuKind::Button => Value::Float(f64::from(value)),
    }
}

/// Save a row's value in the store. A button has none to save. A failure is logged: the value stays for this run.
fn persist(store: &dyn ValueStore, owner: &str, name: &str, kind: &MenuKind, value: f32) {
    if *kind == MenuKind::Button {
        return;
    }
    if let Err(error) = store.save(owner, name, saved_value(kind, value)) {
        log::warn!("mod {owner}: the value of {name:?} is not saved: {error}");
    }
}

impl MenusBackend for MenusPort {
    fn register(&self, owner: &str, row: &MenuRowSpec<'_>, on_change: MenuHandler) -> BackendResult<u64> {
        if !is_plain_name(row.name) {
            return Err(BB_ERR_ARG);
        }
        let mut rows = self.0.menu_rows.borrow_mut();
        if rows.iter().any(|known| known.owner == owner && known.name == row.name) {
            return Err(BB_ERR_EXISTS);
        }
        let saved = match row.kind {
            MenuKind::Button => None,
            _ => self.0.store.saved(owner, row.name),
        };
        let value = fit(&row.kind, saved.unwrap_or(row.default));
        let handle = self.0.next_handle();
        rows.push(MenuRow {
            handle,
            owner: owner.to_owned(),
            name: row.name.to_owned(),
            label: row.label.to_owned(),
            kind: row.kind.clone(),
            value,
            on_change: Rc::new(on_change),
        });
        Ok(handle)
    }

    fn unregister(&self, owner: &str, handle: u64) -> BackendResult<()> {
        let mut rows = self.0.menu_rows.borrow_mut();
        let before = rows.len();
        rows.retain(|row| !(row.handle == handle && row.owner == owner));
        match rows.len() == before {
            true => Err(BB_ERR_NOT_FOUND),
            false => Ok(()),
        }
    }

    fn get(&self, owner: &str, handle: u64) -> BackendResult<f32> {
        let rows = self.0.menu_rows.borrow();
        rows.iter().find(|row| row.handle == handle && row.owner == owner).map(|row| row.value).ok_or(BB_ERR_NOT_FOUND)
    }

    fn set(&self, owner: &str, handle: u64, value: f32) -> BackendResult<()> {
        let (name, kind, fitted) = {
            let mut rows = self.0.menu_rows.borrow_mut();
            let row = rows.iter_mut().find(|row| row.handle == handle && row.owner == owner).ok_or(BB_ERR_NOT_FOUND)?;
            row.value = fit(&row.kind, value);
            (row.name.clone(), row.kind.clone(), row.value)
        };
        persist(self.0.store.as_ref(), owner, &name, &kind, fitted);
        Ok(())
    }
}

/// The rows the Mods page shows, in registration order.
pub fn rows(bridge: &Bridge) -> Vec<RowInfo> {
    bridge
        .menu_rows
        .borrow()
        .iter()
        .map(|row| RowInfo {
            handle: row.handle,
            label: crate::i18n::mod_label(&row.owner, &row.label),
            kind: shown_kind(&row.owner, &row.kind),
            value: row.value,
        })
        .collect()
}

/// A row's kind as the page shows it: a list's choices that name messages of the mod's catalogue are those messages.
fn shown_kind(owner: &str, kind: &MenuKind) -> MenuKind {
    let MenuKind::List { choices } = kind else { return kind.clone() };
    MenuKind::List { choices: choices.iter().map(|choice| crate::i18n::mod_label(owner, choice)).collect() }
}

/// A player's change to row `handle`: one step forward or back. The value is saved, and the mod's callback runs with the
/// new value (a button runs it with 0). A row that cannot move (a slider at its end) does nothing.
pub fn change(bridge: &Bridge, handle: u64, forward: bool) {
    let (owner, name, kind, next, on_change) = {
        let mut rows = bridge.menu_rows.borrow_mut();
        let Some(row) = rows.iter_mut().find(|row| row.handle == handle) else {
            return;
        };
        let Some(next) = step(&row.kind, row.value, forward) else {
            return;
        };
        if row.kind != MenuKind::Button {
            row.value = next;
        }
        (row.owner.clone(), row.name.clone(), row.kind.clone(), next, Rc::clone(&row.on_change))
    };
    persist(bridge.store.as_ref(), &owner, &name, &kind, next);
    (*on_change)(next);
}

/// Whether any mod registered a row. The Options page shows the Mods category only then.
pub fn any_row(bridge: &Bridge) -> bool {
    !bridge.menu_rows.borrow().is_empty()
}
