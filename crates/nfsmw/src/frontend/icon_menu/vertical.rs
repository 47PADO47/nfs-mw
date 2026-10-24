//! The icon menu as a list: what a theme that draws it vertically (the Remastered and Custom menus) reads and does. The
//! package still supplies the order of the entries and the messages; this is the part that is not its icons.

use super::{Category, Cx, Icon, IconMenu, Id, Kind, Pick};
use crate::frontend::custom_race::state as custom_state;
use crate::i18n::t;
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
            Kind::Career => 0x8FFF_61F2,
            Kind::QuickRace => 0xCDF0_CC03,
            // `split screen mode select` in split screen **[decomp]** (`UIQRModeSelect::RefreshHeader`).
            Kind::CustomRace if custom_state::with(|state| state.split.on) => 0x43C8_25ED,
            Kind::CustomRace => 0x1F20_3817,
            Kind::AliasManager => 0x1B76_F2CE,
        }
    }

    /// The text of an icon that is not a language label: the entries the original has no name for. Discord is a name, the
    /// same in every language.
    pub(super) fn special_text(id: Id) -> Option<String> {
        match id {
            Id::ControlOptions => Some(t!("menus-control-options")),
            Id::EndCoop => Some(t!("menus-end-coop")),
            Id::Category(Category::Discord) => Some("Discord".into()),
            Id::Category(Category::Mods) => Some(t!("menus-mods")),
            Id::SafeHouseBlacklist(stage) => Some(t!("menus-blacklist-stage", stage = stage)),
            _ => None,
        }
    }

    /// The picture an icon gives its entry: the package's own texture, or for the control options the controller's glyph of
    /// the input atlas. An icon with no texture has none.
    fn picture(icon: &Icon) -> Option<MenuIcon> {
        if icon.texture == 0 {
            return None;
        }
        Some(MenuIcon { texture: icon.texture, uv: Self::icon_uv(icon).unwrap_or(super::WHOLE_IMAGE) })
    }

    /// The pointer is over (`activate` false) or pressed (`activate` true) the entry `row` of the vertical list.
    pub(super) fn pick(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        if !self.reacts || row >= self.icons.len() {
            return;
        }
        let entry = self.strip.first() + row;
        if entry != self.strip.selected() {
            self.select(cx, entry);
        }
        if matches!(pick, Pick::Press(..)) {
            let before = cx.commands.len();
            self.press(cx);
            let external = self.icons[row].enabled && self.icons[row].id == Id::LanPlay;
            if cx.commands.len() != before || self.pending.is_some() || external {
                crate::audio::menu::request(blackbox_feng::ids::BUTTON_PRESSED);
            }
        }
    }

    /// Moves the list's window so the selected entry is in view (`layout::window_start`): it slides by one entry when the
    /// selection passes either end of the entries shown.
    pub(super) fn follow(&mut self) {
        let selected = self.current_icon().unwrap_or(0);
        self.top = window_start(self.top, selected, self.icons.len());
    }

    /// The menu's entries as the list a theme draws (the footer hints and the aspect are filled in by `Screens`).
    pub(super) fn menu_list(&self, _assets: &UiAssets) -> Option<MenuList> {
        if self.icons.is_empty() {
            return None;
        }
        let entries = self
            .icons
            .iter()
            .map(|icon| {
                let label = Self::special_text(icon.id).or_else(|| Text::Game(icon.name).resolve()).unwrap_or_default();
                MenuEntry { icon: Self::picture(icon), ..MenuEntry::plain(label, icon.enabled) }
            })
            .collect();
        let style = match self.kind {
            Kind::Main => MenuStyle::Main,
            Kind::Categories { pause: false, .. }
            | Kind::SafeHouse
            | Kind::Career
            | Kind::QuickRace
            | Kind::CustomRace
            | Kind::AliasManager => MenuStyle::Sub,
            Kind::Categories { pause: true, .. } | Kind::Pause => MenuStyle::Pause,
        };
        Some(MenuList {
            title: Text::Game(self.title_label()).resolve().unwrap_or_default(),
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
