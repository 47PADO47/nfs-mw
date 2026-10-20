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

/// One entry of the list.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuEntry {
    pub label: String,
    /// A disabled entry can be selected and is drawn dimmed; pressing it does nothing.
    pub enabled: bool,
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
}

/// A menu as a theme draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuList {
    pub title: String,
    pub entries: Vec<MenuEntry>,
    /// The index of the selected entry.
    pub selected: usize,
    /// The index of the entry shown first: a list longer than a screenful scrolls, and the window follows the selection.
    pub first: usize,
    pub hints: Vec<MenuHint>,
    pub style: MenuStyle,
    /// The screen's width over its height, which sets how far left the screen's edge lies in FEng units. The frontend
    /// sets it from the window; a menu that does not know it leaves 16:9.
    pub aspect: f32,
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
            entries: vec![MenuEntry { label: "Career".into(), enabled: true }],
            selected: 0,
            first: 0,
            hints: Vec::new(),
            style: MenuStyle::Main,
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
