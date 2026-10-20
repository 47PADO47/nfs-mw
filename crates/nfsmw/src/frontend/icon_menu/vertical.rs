//! The icon menu as a list: what a theme that draws it vertically (the Remastered menus) reads and does. The package
//! still supplies the order of the entries and the messages; this is the part that is not its icons.

use super::{Category, Cx, Icon, IconMenu, Id, Kind, Pick};
use crate::ui::theme::remastered::menu::layout::window_start;
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuIcon, MenuList, MenuStyle};
use crate::ui::{Text, UiAssets};

impl IconMenu {
    /// The label of the menu's title, as the package's title object shows it.
    pub(super) fn title_label(&self) -> u32 {
        match self.kind {
            Kind::Main => 0xB24A_AE58,
            Kind::Categories { pause: false, .. } => 0x4ECA_678C,
            Kind::Categories { pause: true, .. } => 0x1D7B_B6C9,
            Kind::Pause => 0x6C83_9FBE,
            Kind::SafeHouse => 0xE596_C4A3,
        }
    }

    /// The text of an icon that is not a language label: the entries the original has no name for.
    pub(super) fn special_text(id: Id) -> Option<String> {
        match id {
            Id::ControlOptions => Some("Control Options".into()),
            Id::Category(Category::Discord) => Some("Discord".into()),
            Id::Category(Category::Mods) => Some("Mods".into()),
            Id::SafeHouseBlacklist(stage) => Some(format!("Blacklist {stage}")),
            _ => None,
        }
    }

    /// The picture an icon gives its entry: the package's own texture, or for the control options the controller's glyph of
    /// the input atlas. An icon with no texture has none.
    fn picture(icon: &Icon) -> Option<MenuIcon> {
        if icon.texture == 0 {
            return None;
        }
        let uv = match icon.id {
            Id::ControlOptions => crate::ui::input_icons::Glyph::Controller.uv(),
            _ => [0.0, 0.0, 1.0, 1.0],
        };
        Some(MenuIcon { texture: icon.texture, uv })
    }

    /// The pointer is over (`activate` false) or pressed (`activate` true) the entry `row` of the vertical list.
    pub(super) fn pick(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        if !self.reacts || row >= self.icons.len() {
            return;
        }
        let strip = self.first_slot() + row;
        if strip != self.selected {
            self.select(cx, strip);
        }
        if matches!(pick, Pick::Press(..)) {
            self.press(cx);
        }
    }

    /// Moves the list's window so the selected entry is in view (`layout::window_start`): it slides by one entry when the
    /// selection passes either end of the entries shown.
    pub(super) fn follow(&mut self) {
        let selected = self.current_icon().unwrap_or(0);
        self.top = window_start(self.top, selected, self.icons.len());
    }

    /// The menu's entries as the list a theme draws (the footer hints and the aspect are filled in by `Screens`).
    pub(super) fn menu_list(&self, assets: &UiAssets) -> Option<MenuList> {
        if self.icons.is_empty() {
            return None;
        }
        let entries = self
            .icons
            .iter()
            .map(|icon| {
                let label =
                    Self::special_text(icon.id).or_else(|| Text::Game(icon.name).resolve(assets)).unwrap_or_default();
                MenuEntry { icon: Self::picture(icon), ..MenuEntry::plain(label, icon.enabled) }
            })
            .collect();
        let style = match self.kind {
            Kind::Main => MenuStyle::Main,
            Kind::Categories { pause: false, .. } | Kind::SafeHouse => MenuStyle::Sub,
            Kind::Categories { pause: true, .. } | Kind::Pause => MenuStyle::Pause,
        };
        Some(MenuList {
            title: Text::Game(self.title_label()).resolve(assets).unwrap_or_default(),
            message: String::new(),
            entries,
            selected: self.current_icon().unwrap_or(0),
            first: self.top,
            hints: Vec::new(),
            style,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
    }
}
