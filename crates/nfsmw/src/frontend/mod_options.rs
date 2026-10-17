//! The Mods page of the options (docs/specs/mod-manager.md). It lists the mods with a switch each (the mod manager,
//! `modding/manager.rs`), then the rows the mods registered (`bb.menus`), shown the way the option rows are.
//!
//! The rows are the mods', not the settings': a switch's value is read from the mod manager each time it is drawn, and
//! a row's label, control and data are read from the bridge each time they are drawn, so a value a mod sets shows at
//! once. A player's change goes back to the manager (`toggle`) or to the bridge (`change`), which saves it and calls the mod.

use blackbox_modding::host::MenuKind;

use super::ids::{LABEL_OFF, LABEL_ON};
use super::options::{Control, Data, Row, Setting, Title};
use crate::modding::manager::Row as Switch;
use crate::modding::{Mods, RowInfo, change_menu_row, menu_rows};

/// The text a button row shows where a toggle shows its value.
const PRESS: &str = "Press";

/// What a Mods row shows: its label, its control and its data.
pub struct Shown {
    pub label: String,
    pub control: Control,
    pub data: Data,
}

/// The rows of the Mods page: one switch per listed mod, then the mods' own rows in the order they registered them.
/// None without mods.
pub fn rows(mods: Option<&Mods>) -> Vec<Row> {
    let Some(mods) = mods else {
        return Vec::new();
    };
    let switches = (0..mods.manager().rows().len())
        .map(|index| Row { setting: Setting::ModManaged(index), title: Title::Text("") });
    let registered = menu_rows(mods.bridge())
        .into_iter()
        .map(|info| Row { setting: Setting::Mod(info.handle), title: Title::Text("") });
    switches.chain(registered).collect()
}

/// What a Mods row shows, or `None` for any other setting, or for a row that is gone.
pub fn shown(mods: Option<&Mods>, setting: Setting) -> Option<Shown> {
    let mods = mods?;
    match setting {
        Setting::ModManaged(index) => {
            let switch = mods.manager().rows().into_iter().nth(index)?;
            Some(describe_switch(&switch))
        }
        Setting::Mod(handle) => {
            let info = menu_rows(mods.bridge()).into_iter().find(|info| info.handle == handle)?;
            Some(describe(info))
        }
        _ => None,
    }
}

/// A player's change to a Mods row: a switch flips, a mod's row steps one step forward or back. Any other setting
/// does nothing.
pub fn change(mods: Option<&Mods>, setting: Setting, forward: bool) {
    let Some(mods) = mods else {
        return;
    };
    match setting {
        Setting::ModManaged(index) => {
            if let Err(e) = mods.manager().toggle(index) {
                log::warn!("mod manager: {e}");
            }
        }
        Setting::Mod(handle) => change_menu_row(mods.bridge(), handle, forward),
        _ => {}
    }
}

/// A switch shows its mod's id, the switch's value, and a note when the next start will change what this run did.
fn describe_switch(switch: &Switch) -> Shown {
    let note = match (switch.loaded, switch.enabled) {
        (true, false) => " (restart to unload)",
        (false, true) => " (restart to load)",
        _ => "",
    };
    let label = format!("{}{note}", switch.id);
    let data = match switch.enabled {
        true => Data::Label(LABEL_ON),
        false => Data::Label(LABEL_OFF),
    };
    Shown { label, control: Control::Toggle, data }
}

fn describe(info: RowInfo) -> Shown {
    let (control, data) = match &info.kind {
        MenuKind::Toggle => {
            let label = match info.value >= 0.5 {
                true => LABEL_ON,
                false => LABEL_OFF,
            };
            (Control::Toggle, Data::Label(label))
        }
        MenuKind::Slider { min, max, .. } => {
            (Control::Slider(percent(info.value, *min, *max)), Data::Text(format!("{}", info.value)))
        }
        MenuKind::List { choices } => {
            let chosen = choices.get(info.value as usize).cloned().unwrap_or_default();
            (Control::Toggle, Data::Text(chosen))
        }
        MenuKind::Button => (Control::Toggle, Data::Text(PRESS.to_owned())),
    };
    Shown { label: info.label, control, data }
}

/// Where `value` lies in `min..max`, as the slider's percentage.
fn percent(value: f32, min: f32, max: f32) -> u8 {
    let span = max - min;
    match span > 0.0 {
        true => (((value - min) / span) * 100.0).round().clamp(0.0, 100.0) as u8,
        false => 0,
    }
}

#[cfg(test)]
#[path = "mod_options_tests.rs"]
mod tests;
