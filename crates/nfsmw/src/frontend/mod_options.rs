//! The Mods page of the options: the rows the mods registered (`bb.menus`), shown the way the option rows are.
//!
//! The rows are the bridge's, not the settings': a row's label, control and data are read from the bridge each time they
//! are drawn, so a value a mod sets shows at once, and a player's change goes back to the bridge (`change`), which saves it
//! and calls the mod.

use blackbox_modding::host::MenuKind;

use super::ids::{LABEL_OFF, LABEL_ON};
use super::options::{Control, Data, Row, Setting, Title};
use crate::modding::{Bridge, RowInfo, change_menu_row, menu_rows};

/// The text a button row shows where a toggle shows its value.
const PRESS: &str = "Press";

/// What a Mods row shows: its label, its control and its data.
pub struct Shown {
    pub label: String,
    pub control: Control,
    pub data: Data,
}

/// The rows of the Mods page, in the order the mods registered them. None without a bridge (no mods).
pub fn rows(bridge: Option<&Bridge>) -> Vec<Row> {
    let Some(bridge) = bridge else {
        return Vec::new();
    };
    menu_rows(bridge).iter().map(|info| Row { setting: Setting::Mod(info.handle), title: Title::Text("") }).collect()
}

/// What a Mods row shows, or `None` for any other setting, or for a row that is gone.
pub fn shown(bridge: Option<&Bridge>, setting: Setting) -> Option<Shown> {
    let Setting::Mod(handle) = setting else {
        return None;
    };
    let info = menu_rows(bridge?).into_iter().find(|info| info.handle == handle)?;
    Some(describe(info))
}

/// A player's change to a Mods row, one step forward or back. A setting that is not a Mods row does nothing.
pub fn change(bridge: Option<&Bridge>, setting: Setting, forward: bool) {
    let (Some(bridge), Setting::Mod(handle)) = (bridge, setting) else {
        return;
    };
    change_menu_row(bridge, handle, forward);
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
