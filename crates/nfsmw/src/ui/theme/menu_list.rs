//! The list a themed menu draws. An icon menu (the main menu, the option categories, the pause menu, the safe house) is a
//! row of FEng icons under the Original theme. A theme that lays the same entries out another way (the Remastered
//! vertical list) needs the entries, the selection and the footer hints, not the FEng package. The frontend fills this
//! each frame from the screen's logic ([`Screens::presented`](crate::frontend::Presented)); a theme reads it by slot.
//!
//! The list is data about the menu, never about a theme: it names no layout and no colour.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use blackbox_ui::SlotId;

use crate::input::Action;
use crate::ui::prompts::Prompt;

/// What an entry of a settings page shows to the right of its label.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum MenuValue {
    /// Nothing: the entries of an icon menu.
    #[default]
    None,
    /// A value that left and right cycle through, between two chevrons. A row that only does something when pressed has
    /// no `arrows`.
    Choice { text: String, arrows: bool },
    /// A level from 0 to 100.
    Slider(u8),
    /// Two keys, one per column, as the key list shows a control: what is bound, and which of the two has the focus (only
    /// the selected row has one). A column waiting for a key reads as the caller's prompt.
    Keys { primary: String, secondary: String, focus: Option<usize> },
    /// A rival of the Blacklist: the entry's label is the rival's name.
    Rival(RivalInfo),
}

/// Where a rival of the Blacklist stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RivalStatus {
    /// Every boss race is won.
    Complete,
    /// The rival the player is working towards now.
    InProgress,
    /// A rival further down the list than the player has come.
    #[default]
    Locked,
}

/// What a theme shows of one rival of the Blacklist.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RivalInfo {
    /// The rank, 15 down to 1.
    pub rank: u8,
    /// The texture key of the rival's picture in the game's files, 0 when the rank has none.
    pub portrait: u32,
    /// The rival's alias, car and strength, as the bio gives them; empty when the text is missing.
    pub alias: String,
    pub car: String,
    pub strength: String,
    /// The bounty the rival's gate needs, as written for the screen.
    pub bounty: String,
    pub status: RivalStatus,
}

/// The picture of an entry: a texture key, and the part of it to show.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuIcon {
    pub texture: u32,
    pub uv: [f32; 4],
}

/// One entry of the list.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuEntry {
    pub label: String,
    /// A disabled entry can be selected and is drawn dimmed; pressing it does nothing.
    pub enabled: bool,
    pub value: MenuValue,
    /// The picture the entry has beside its name, for a theme that shows pictures (`Tokens::icons`).
    pub icon: Option<MenuIcon>,
}

impl MenuEntry {
    /// An entry with no value, as an icon menu has.
    pub fn plain(label: impl Into<String>, enabled: bool) -> Self {
        Self { label: label.into(), enabled, value: MenuValue::None, icon: None }
    }
}

/// The last press on a row of a settings page: which row, which way, and how many seconds ago. The chevron it pressed is
/// nudged for a moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuNudge {
    pub row: usize,
    pub forward: bool,
    pub age: f32,
}

/// One footer hint: the keys that do something, what they do, and the action a click on it presses.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuHint {
    pub keys: Vec<Prompt>,
    pub label: &'static str,
    pub action: Option<Action>,
}

/// Which menu the list is, for the parts of the screen that differ: the title screen's menu carries the game's logo and
/// a tagline, a sub-menu the logo only, and the in-game menu neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuStyle {
    Main,
    Sub,
    Pause,
    /// The credits roll: the lines of `message`, one to a line of text, scrolled by `scroll`.
    Credits,
    /// A question or a notice in a panel over the screen: `title`, `message`, and the answers as the entries.
    Dialog,
    /// The Blacklist: one `MenuValue::Rival` entry per rival, with `message` the count of rivals beaten ("3/15").
    Blacklist,
    /// One rival's bio: a single `MenuValue::Rival` entry, with `message` the bio's text.
    Rival,
}

/// A menu as a theme draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuList {
    pub title: String,
    /// The body of a dialog; empty for every other list.
    pub message: String,
    pub entries: Vec<MenuEntry>,
    /// The index of the selected entry.
    pub selected: usize,
    /// The index of the entry shown first: a list longer than a screenful scrolls, and the window follows the selection.
    pub first: usize,
    pub hints: Vec<MenuHint>,
    pub style: MenuStyle,
    /// The last press on a row, for the chevron's nudge.
    pub nudge: Option<MenuNudge>,
    /// How far the roll of a credits list has scrolled past its first line, as a fraction of a line.
    pub scroll: f32,
    /// The screen's width over its height, which sets how far left the screen's edge lies in FEng units. The frontend
    /// sets it from the window; a menu that does not know it leaves 16:9.
    pub aspect: f32,
}

impl MenuList {
    /// Whether the entries carry values, as a settings page's do: the rows are wider and the values sit in a column.
    pub fn has_values(&self) -> bool {
        self.entries.iter().any(|entry| entry.value != MenuValue::None)
    }
}

/// The aspect a list has until the frontend knows the window's.
pub const DEFAULT_ASPECT: f32 = 16.0 / 9.0;

/// The lists of this frame by menu slot. Clones share one map: the frontend fills it, and the theme's component for the
/// slot reads it.
#[derive(Clone, Debug, Default)]
pub struct MenuListCell {
    lists: Arc<Mutex<HashMap<SlotId, MenuList>>>,
}

impl MenuListCell {
    pub fn new() -> Self {
        Self::default()
    }

    /// Empties every slot, so a screen that closed leaves no list behind.
    pub fn clear(&self) {
        self.lock().clear();
    }

    /// Stores the list of the screen that took `slot` this frame.
    pub fn set(&self, slot: SlotId, list: MenuList) {
        self.lock().insert(slot, list);
    }

    /// This frame's list for `slot`.
    pub fn get(&self, slot: SlotId) -> Option<MenuList> {
        self.lock().get(&slot).cloned()
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<SlotId, MenuList>> {
        self.lists.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(title: &str) -> MenuList {
        MenuList {
            title: title.into(),
            message: String::new(),
            entries: vec![MenuEntry::plain("Career", true)],
            selected: 0,
            first: 0,
            hints: Vec::new(),
            style: MenuStyle::Main,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        }
    }

    #[test]
    fn a_list_is_read_back_by_its_slot_and_replaced_by_the_next_set() {
        let cell = MenuListCell::new();
        cell.set(SlotId::new(20), list("main"));
        assert_eq!(cell.get(SlotId::new(20)).map(|l| l.title), Some("main".into()));
        assert!(cell.get(SlotId::new(21)).is_none());
        cell.set(SlotId::new(20), list("again"));
        assert_eq!(cell.get(SlotId::new(20)).map(|l| l.title), Some("again".into()));
    }

    #[test]
    fn clear_empties_every_slot_and_clones_share_one_map() {
        let cell = MenuListCell::new();
        let other = cell.clone();
        cell.set(SlotId::new(20), list("main"));
        assert!(other.get(SlotId::new(20)).is_some(), "a clone sees the list");
        other.clear();
        assert!(cell.get(SlotId::new(20)).is_none(), "and its clear empties the original");
    }
}
