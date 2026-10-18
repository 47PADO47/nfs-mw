//! The Mods page of the options (docs/specs/mod-manager.md). It lists the mods with a switch each (the mod manager,
//! `modding/manager.rs`), then the rows the mods registered (`bb.menus`), shown the way the option rows are.
//!
//! The rows are the mods', not the settings': a switch's value is read from the mod manager each time it is drawn, and
//! a row's label, control and data are read from the bridge each time they are drawn, so a value a mod sets shows at
//! once. A player's change goes back to the manager (`toggle`) or to the bridge (`change`), which saves it and calls the mod.
//!
//! The first row is the UI theme choice (`bb.ui`, `docs/specs/ui-themes.md` section 12). It is offered only when a mod
//! registered a theme. It steps through the built-in style and then each mod theme; the bridge keeps the choice and saves
//! it, and the theme sync (`ui/theme`) makes it active.

use blackbox_modding::host::MenuKind;

use super::ids::{LABEL_OFF, LABEL_ON};
use super::options::{Control, Data, Row, Setting, Title};
use crate::modding::manager::Row as Switch;
use crate::modding::{
    Mods, RowInfo, change_menu_row, has_ui_themes, menu_rows, step_ui_theme_choice, ui_theme_display,
};

/// The text a button row shows where a toggle shows its value.
const PRESS: &str = "Press";
/// The label of the UI theme row, and what it shows while the built-in style is chosen.
const THEME_LABEL: &str = "UI theme";
const BUILT_IN: &str = "Built-in style";

/// What a Mods row shows: its label, its control and its data.
pub struct Shown {
    pub label: String,
    pub control: Control,
    pub data: Data,
}

/// The rows of the Mods page: the UI theme choice when a mod registered a theme, then one switch per listed mod, then the
/// mods' own rows in the order they registered them. None without mods.
pub fn rows(mods: Option<&Mods>) -> Vec<Row> {
    let Some(mods) = mods else {
        return Vec::new();
    };
    let theme = theme_row(has_ui_themes(mods.bridge()));
    let switches = (0..mods.manager().rows().len())
        .map(|index| Row { setting: Setting::ModManaged(index), title: Title::Text("") });
    let registered = menu_rows(mods.bridge())
        .into_iter()
        .map(|info| Row { setting: Setting::Mod(info.handle), title: Title::Text("") });
    theme.into_iter().chain(switches).chain(registered).collect()
}

/// The UI theme row, when `offered`: a mod registered a theme, so there is a choice to make.
fn theme_row(offered: bool) -> Option<Row> {
    match offered {
        true => Some(Row { setting: Setting::ModTheme, title: Title::Text("") }),
        false => None,
    }
}

/// What a Mods row shows, or `None` for any other setting, or for a row that is gone.
pub fn shown(mods: Option<&Mods>, setting: Setting) -> Option<Shown> {
    let mods = mods?;
    match setting {
        Setting::ModTheme => Some(describe_theme(ui_theme_display(mods.bridge()))),
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

/// A player's change to a Mods row: the theme choice steps, a switch flips, a mod's row steps one step forward or back.
/// Any other setting does nothing.
pub fn change(mods: Option<&Mods>, setting: Setting, forward: bool) {
    let Some(mods) = mods else {
        return;
    };
    match setting {
        Setting::ModTheme => step_ui_theme_choice(mods.bridge(), forward),
        Setting::ModManaged(index) => {
            if let Err(e) = mods.manager().toggle(index) {
                log::warn!("mod manager: {e}");
            }
        }
        Setting::Mod(handle) => change_menu_row(mods.bridge(), handle, forward),
        _ => {}
    }
}

/// The UI theme row shows the chosen mod theme's display text, or the built-in style when none is chosen.
fn describe_theme(chosen: Option<String>) -> Shown {
    let text = chosen.unwrap_or_else(|| BUILT_IN.to_owned());
    Shown { label: THEME_LABEL.to_owned(), control: Control::Toggle, data: Data::Text(text) }
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
