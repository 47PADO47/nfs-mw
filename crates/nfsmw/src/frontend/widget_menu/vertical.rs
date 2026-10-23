//! The option screens as a vertical list: what a theme that draws them its own way (the Remastered settings pages) reads
//! and does. The package still supplies the messages; the rows, their values and the window onto them are published as a
//! [`MenuList`], and a click or the pointer picks a row.

use super::options::{self, Control, Data, Row, Title};
use super::{Cx, Pick, Way, WidgetMenu, mod_options};
use crate::ui::Text;
use crate::ui::theme::remastered::menu::layout::{MAX_ROWS, window_start};
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuList, MenuNudge, MenuStyle, MenuValue};

impl WidgetMenu {
    /// Pointer edits have no FEng pad announcement. Play feedback only when the edit succeeds.
    pub(super) fn change_from_pointer(&mut self, cx: &mut Cx, forward: bool) {
        let Some(row) = self.rows.get(self.selected).copied() else { return };
        if self.leaving || !row.setting.enabled_in(cx.settings, &cx.caps) {
            return;
        }
        let before = *cx.settings;
        let data = data_of(cx, row.setting);
        let commands = cx.commands.len();
        self.change(cx, forward);
        if before != *cx.settings || data != data_of(cx, row.setting) || commands != cx.commands.len() {
            let message = match row.setting {
                options::Setting::AdvancedGraphics => blackbox_feng::ids::BUTTON_PRESSED,
                _ => blackbox_feng::ids::PAD_RIGHT,
            };
            crate::audio::menu::request(message);
        }
    }

    /// How many rows show at once: the package's own count, or the vertical list's window when that is smaller.
    pub(super) fn window(&self) -> usize {
        match self.vertical {
            true => self.on_screen.min(MAX_ROWS),
            false => self.on_screen,
        }
    }

    /// Whether the Remastered look draws this page, and if it does the rows as the list shows them. The window follows the
    /// selection, so a theme change on this very page leaves the selected row in view.
    pub(super) fn refresh(&mut self, cx: &Cx) {
        self.vertical = cx.vertical_look();
        if !self.vertical {
            self.entries.clear();
            return;
        }
        self.top = window_start(self.top, self.selected, self.rows.len());
        self.entries = self.rows.iter().map(|row| entry_of(cx, row)).collect();
    }

    /// The page as a list, or `None` when it is not drawn as one.
    pub(super) fn menu_list(&self) -> Option<MenuList> {
        if !self.vertical || self.entries.is_empty() {
            return None;
        }
        let nudge = self.nudge.map(|(row, forward, since)| MenuNudge { row, forward, age: self.clock - since });
        let style = match self.args.pause {
            true => MenuStyle::Pause,
            false => MenuStyle::Sub,
        };
        Some(MenuList {
            title: self.heading.clone(),
            message: String::new(),
            entries: self.entries.clone(),
            selected: self.selected,
            first: self.top,
            hints: Vec::new(),
            style,
            nudge,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
    }

    /// The pointer is over a row (`Pick::Hover`), or a click landed on it: on a chevron it changes the value that way, and
    /// anywhere else it steps the value on.
    pub(super) fn pick(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        if self.leaving || row >= self.rows.len() {
            return;
        }
        self.select(cx, row);
        let Pick::Press(way, _) = pick else { return };
        self.change_from_pointer(cx, way != Way::Back);
    }
}

/// The control of a row: a Mods row's from the bridge, any other row's from the settings.
pub(super) fn control_of(cx: &Cx<'_>, setting: options::Setting) -> Control {
    mod_options::shown(cx.mods, setting).map_or_else(|| setting.control(&*cx.settings), |shown| shown.control)
}

/// The data of a row: a Mods row's from the bridge, any other row's from the settings.
pub(super) fn data_of(cx: &Cx<'_>, setting: options::Setting) -> Data {
    mod_options::shown(cx.mods, setting).map_or_else(|| setting.data_in(&*cx.settings, &cx.caps), |shown| shown.data)
}

/// The page's heading: a language label, or the page's own name.
pub(super) fn heading(cx: &Cx, category: super::super::logic::Category, label: u32) -> String {
    use super::super::logic::Category;
    match category {
        Category::Controls => "Control Options".into(),
        Category::Discord => "Discord".into(),
        Category::Mods => "Mods".into(),
        Category::CustomRace => "Track Options".into(),
        _ => Text::Game(label).resolve(cx.assets).unwrap_or_default(),
    }
}

/// One row as the list shows it: its title, whether it can be changed, and its value.
fn entry_of(cx: &Cx, row: &Row) -> MenuEntry {
    let shown = mod_options::shown(cx.mods, row.setting);
    let label = match (&shown, row.title) {
        (Some(shown), _) => shown.label.clone(),
        (None, Title::Label(label)) => Text::Game(label).resolve(cx.assets).unwrap_or_default(),
        (None, Title::Text(text)) => text.to_owned(),
    };
    let value = match control_of(cx, row.setting) {
        Control::Slider(percent) => MenuValue::Slider(percent),
        Control::Toggle => {
            let text = match data_of(cx, row.setting) {
                Data::Label(label) => Text::Game(label).resolve(cx.assets).unwrap_or_default(),
                Data::Text(text) => text,
            };
            // A row that opens something or runs a command has no values to cycle, so no chevrons.
            let acts = matches!(row.setting, options::Setting::AdvancedGraphics);
            MenuValue::Choice { text, arrows: !acts }
        }
    };
    MenuEntry { label, enabled: row.setting.enabled_in(cx.settings, &cx.caps), value, icon: None }
}
