//! The rider's gear in the garage: one page for each part (body, helmet, suit, gloves, boots), reached from the Visual category
//! of a bike when a mod offers gear (`choices.rs`, `docs/specs/ride6-rider-gear.md`).
//!
//! The page is the game's notice dialog, as the plate design's is (`plate_design.rs`). Left and Right step through the list (L and
//! R ten at a time) and the rider on the bike in the garage wears each item as it is reached; Accept keeps the item beside the
//! profile's save (`garage/rider_gear`), Back puts back the one he wore. In a theme that lays menus out as vertical lists the dialog
//! shows a window of the names around the one picked. Gear costs nothing and is not part of the cart.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK, PAD_DOWN, PAD_LEFT, PAD_LTRIGGER, PAD_RIGHT, PAD_RTRIGGER, PAD_UP};

use super::plate_design::{JUMP, WINDOW, stepped, window_start};
use super::{GarageUi, Request};
use crate::frontend::logic::{Cx, Footer, Pick, ScreenLogic};
use crate::frontend::{confirm, ids};
use crate::garage::rider_gear::{self, Entry, Part};
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuList, MenuStyle};
use crate::ui::{Text, UiAssets};

/// The list of a part: the rider's own gear first, then what the mod offers; just the first when nothing is offered.
fn entries(part: Part) -> Vec<Entry> {
    let stock = || vec![Entry { id: String::new(), label: rider_gear::catalogue::STOCK.to_owned() }];
    rider_gear::loaded().map_or_else(stock, |catalogue| catalogue.entries(part))
}

/// The position of the item `id` in `entries`; the rider's own gear (the first) for an id the list lacks.
fn position(entries: &[Entry], id: &str) -> usize {
    entries.iter().position(|entry| entry.id == id).unwrap_or(0)
}

pub(super) struct GearPage {
    handle: GarageUi,
    part: Part,
    entries: Vec<Entry>,
    /// The entry the cursor is on.
    at: usize,
    vertical: bool,
    done: bool,
}

impl GearPage {
    /// The cursor starts on the item the profile kept for the part.
    pub fn new(handle: GarageUi, part: Part) -> Self {
        rider_gear::reload();
        let entries = entries(part);
        let at = position(&entries, rider_gear::committed().get(part));
        Self { handle, part, entries, at, vertical: false, done: false }
    }

    /// The text the notice shows: which item, of how many.
    fn shown(&self) -> String {
        let entry = &self.entries[self.at];
        format!("{} {} of {}: {}", self.part.label(), self.at + 1, self.entries.len(), entry.label)
    }

    fn refresh(&self, cx: &mut Cx) {
        confirm::set_text(cx, confirm::MESSAGE, &Text::Literal(self.shown()));
    }

    /// Moves the cursor and shows the rider in the item under it.
    fn go(&mut self, to: usize) {
        self.at = to.min(self.entries.len().saturating_sub(1));
        let mut choice = rider_gear::committed();
        choice.set(self.part, &self.entries[self.at].id);
        rider_gear::set_preview(Some(choice));
    }

    fn step(&mut self, by: isize) {
        let to = stepped(self.at, by, self.entries.len());
        self.go(to);
    }

    /// Keeps the item and leaves; nothing was bought, so there is nothing for leaving the garage to discard.
    fn accept(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
        if let Err(error) = rider_gear::commit(self.part, &self.entries[self.at].id) {
            log::warn!("the rider's {} is not kept: {error:#}", self.part.key());
            rider_gear::set_preview(None);
        }
        self.handle.request(Request::Pop);
    }

    /// Puts back what he wore when the page opened and leaves.
    fn cancel(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
        rider_gear::set_preview(None);
        self.handle.request(Request::Pop);
    }
}

impl Drop for GearPage {
    /// However the page goes away, what is on show goes with it.
    fn drop(&mut self) {
        if !self.done {
            rider_gear::set_preview(None);
        }
    }
}

impl ScreenLogic for GearPage {
    fn start(&mut self, cx: &mut Cx) {
        self.vertical = cx.list_look();
        confirm::prepare_notice(cx);
        confirm::set_text(cx, ids::HEADER_TEXT, &Text::Literal(self.part.label().to_owned()));
        self.refresh(cx);
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.vertical = cx.list_look();
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    /// In the vertical list the names are one above the other, so Up and Down are the package's Left and Right.
    fn rotate_pad(&self) -> bool {
        self.vertical
    }

    fn click(&mut self, _cx: &mut Cx, _point: glam::Vec2) -> bool {
        self.vertical
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        self.step(-(steps as isize));
        self.refresh(cx);
    }

    fn pick_row(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        if !matches!(pick, Pick::Press(..)) {
            return;
        }
        let target = window_start(self.at, self.entries.len()) + row;
        if target == self.at {
            self.accept();
            return;
        }
        self.go(target);
        self.refresh(cx);
    }

    fn list(&self, _assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let first = window_start(self.at, self.entries.len());
        let shown = &self.entries[first..(first + WINDOW).min(self.entries.len())];
        Some(MenuList {
            title: self.part.label().to_owned(),
            message: self.shown(),
            entries: shown.iter().map(|entry| MenuEntry::plain(entry.label.clone(), true)).collect(),
            selected: self.at - first,
            first: 0,
            hints: Vec::new(),
            style: MenuStyle::Dialog,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_LEFT | PAD_UP => self.step(-1),
            PAD_RIGHT | PAD_DOWN => self.step(1),
            PAD_LTRIGGER => self.step(-JUMP),
            PAD_RTRIGGER => self.step(JUMP),
            confirm::ANSWERED | PAD_ACCEPT => self.accept(),
            PAD_BACK => self.cancel(),
            _ => {}
        }
        self.refresh(cx);
    }

    fn footer(&self) -> Footer {
        Footer::BackAccept
    }
}

#[cfg(test)]
#[path = "gear_page_tests.rs"]
mod tests;
