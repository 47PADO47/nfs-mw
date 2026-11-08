//! The rider's gear in a bike's shop: one page for each part (body, helmet, suit, gloves, boots), reached from the Visual category
//! of a bike in a career shop when a mod offers gear (`choices.rs`, `docs/specs/ride6-rider-gear.md`).
//!
//! The page is the game's notice dialog, as the plate design's is (`plate_design.rs`). Left and Right step through the list (L and
//! R ten at a time) and the rider on the bike in the shop wears each item as it is reached, with its price in the text (nothing for
//! the rider's own gear and for an item the profile owns). Accept puts the item in the cart like any part (`Row::Gear`): the price
//! is part of the total, and the checkout charges it and keeps the choice beside the profile's save (`garage/rider_gear`); Back
//! leaves the cart as it was. In a theme that lays menus out as vertical lists the dialog shows a window of the names around the one picked.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK, PAD_DOWN, PAD_LEFT, PAD_LTRIGGER, PAD_RIGHT, PAD_RTRIGGER, PAD_UP};

use super::plate_design::{JUMP, WINDOW, stepped, window_start};
use super::{GarageUi, Request};
use crate::frontend::logic::{Cx, Footer, Pick, ScreenLogic};
use crate::frontend::{confirm, ids};
use crate::garage::menu_rows::Row;
use crate::garage::rider_gear::{self, Entry, Part, position};
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuList, MenuStyle};
use crate::ui::{Text, UiAssets};

/// The list of a part: the rider's own gear first, then what the mod offers; just the first when nothing is offered.
fn entries(part: Part) -> Vec<Entry> {
    let stock = || vec![Entry { id: String::new(), label: rider_gear::catalogue::STOCK.to_owned() }];
    rider_gear::loaded().map_or_else(stock, |catalogue| catalogue.entries(part))
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
        let at = position(&entries, handle.model().draft.gear.get(part));
        Self { handle, part, entries, at, vertical: false, done: false }
    }

    /// The text the notice shows: which item, of how many.
    fn shown(&self) -> String {
        let entry = &self.entries[self.at];
        let cost = rider_gear::price::cost(self.part, &entry.id, rider_gear::owns);
        let price = match (entry.id.is_empty(), cost) {
            (true, _) => String::new(),
            (false, 0) => " (owned)".to_owned(),
            (false, cost) => format!(" (${})", crate::i18n::fmt::grouped(i64::from(cost))),
        };
        format!("{} {} of {}: {}{price}", self.part.label(), self.at + 1, self.entries.len(), entry.label)
    }

    fn refresh(&self, cx: &mut Cx) {
        confirm::set_text(cx, confirm::MESSAGE, &Text::Literal(self.shown()));
    }

    /// Moves the cursor and shows the rider in the item under it.
    fn go(&mut self, to: usize) {
        self.at = to.min(self.entries.len().saturating_sub(1));
        let mut choice = self.handle.model().draft.gear;
        choice.set(self.part, &self.entries[self.at].id);
        rider_gear::set_preview(Some(choice));
    }

    fn step(&mut self, by: isize) {
        let to = stepped(self.at, by, self.entries.len());
        self.go(to);
    }

    /// Puts the item in the cart and leaves: the checkout charges it and keeps it.
    fn accept(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
        self.handle.preview(Row::Gear(self.part), self.at);
        self.handle.add_to_cart();
        self.handle.request(Request::Pop);
    }

    /// Puts back what the cart showed when the page opened and leaves.
    fn cancel(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
        rider_gear::set_preview(Some(self.handle.model().draft.gear));
        self.handle.request(Request::Pop);
    }
}

impl Drop for GearPage {
    /// However the page goes away, what is on show goes back to the cart's.
    fn drop(&mut self) {
        if !self.done {
            rider_gear::set_preview(Some(self.handle.model().draft.gear));
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
        confirm::keep_notice(cx);
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
