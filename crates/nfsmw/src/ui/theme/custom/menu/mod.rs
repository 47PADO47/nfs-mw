//! The Custom menus (`docs/specs/custom-hud/menus.md`): the main menu, the option categories, the pause menu, the safe
//! house, the option pages and the dialogs (PR 11a), and the key list, the EA Trax playlist, the credits, the Blacklist
//! with a rival's bio, and the LAN pages (PR 11b), written from scratch in the Custom tokens and the OS fonts. They read
//! the entries the frontend publishes for the list looks ([`MenuListCell`], `frontend/logic/look.rs`; the LAN pages
//! from the LAN cell) and draw nothing for a slot with no list, as the FEng screens keep running underneath.
//!
//! One component, [`CustomMenu`], draws the menu slots; what differs comes from the list itself: a dialog
//! (`MenuStyle::Dialog`) is a centred plate, the credits roll, the Blacklist and a rival's bio have pages of their own,
//! the pause menu dims the whole frozen world, a settings page (entries with values) has wider rows with a value
//! column, and the other menus are a list down the left of the content box.
//!
//! - [`layout`]: the places, shared with the frontend's hit tests.
//! - [`look`]: the `[custom_hud]` options the menus take (text size, motion reduction), carried in the skin.
//! - [`glide`]: the selection plate's glide from row to row.
//! - `list`, `value`, `keys`, `footer`, `dialog`: the parts of a screen; `keys` the key list's caps.
//! - `jukebox`, `credits`, [`blacklist`], `rival`: the playlist, the roll, the Blacklist and its card and bio page.
//! - [`lan`]: the LAN page (`LanMenu`), a list with the LAN's own lines round it.

pub mod blacklist;
mod credits;
mod dialog;
mod footer;
pub mod glide;
mod jukebox;
mod keys;
pub mod lan;
pub mod layout;
mod list;
pub mod look;
mod rival;
#[cfg(test)]
mod screens_tests;
#[cfg(test)]
mod tests;
mod value;

pub use blacklist::row_at as blacklist_row_at;
pub use dialog::row_at as dialog_row_at;
pub use footer::hint_at;
pub use glide::GlideCell;
pub use lan::LanMenu;
pub use look::MenuLook;

use blackbox_ui::{Component, DrawList, LayoutCx, Panel, Rect, SlotId, justify};

use self::blacklist::BlacklistFrame;
use self::layout::MenuFrame;
use super::plate::{Plate, PlateStyle};
use super::text::Label;
use super::tokens::faded_skin;
use crate::ui::theme::{ElementId, MenuList, MenuListCell, MenuStyle};

/// The slots the Custom menus draw from the menu list cell. The LAN page is drawn from its own cell ([`LanMenu`]).
pub const SLOTS: [ElementId; 10] = [
    ElementId::MainMenu,
    ElementId::Categories,
    ElementId::PauseMenu,
    ElementId::SafeHouseMenu,
    ElementId::OptionTable,
    ElementId::Dialog,
    ElementId::ControlsList,
    ElementId::Jukebox,
    ElementId::Credits,
    ElementId::Blacklist,
];

/// Whether the Custom menus draw `slot`.
pub fn draws(slot: SlotId) -> bool {
    ElementId::from_slot(slot).is_some_and(|element| SLOTS.contains(&element))
}

/// The Custom menu component for one menu slot.
pub struct CustomMenu {
    slot: SlotId,
    cell: MenuListCell,
    glide: GlideCell,
    parts: Vec<Box<dyn Component>>,
}

impl CustomMenu {
    /// The menu of `slot`, drawn from `cell`, its selection gliding on `glide`.
    pub fn new(slot: SlotId, cell: MenuListCell, glide: GlideCell) -> Self {
        Self { slot, cell, glide, parts: Vec::new() }
    }
}

impl Component for CustomMenu {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let Some(list) = self.cell.get(self.slot) else {
            self.glide.forget(self.slot);
            self.parts = Vec::new();
            return;
        };
        self.parts = screen(self.slot, &list, cx, &self.glide).finish();
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// The parts of the screen `list` on `slot`, its selection plate gliding on `glide` (no glide with motion reduction).
fn screen<'a, 'b>(slot: SlotId, list: &MenuList, cx: &'a LayoutCx<'b>, glide: &GlideCell) -> Parts<'a, 'b> {
    let look = MenuLook::from_skin(cx.skin);
    let mut parts = Parts::new(cx, MenuFrame::new(list.aspect, look.text_size()));
    let duration = match look.reduce_motion() {
        true => 0.0,
        false => glide::GLIDE_SECS,
    };
    let plate = |target: Option<f32>| target.map(|y| glide.follow(slot, &list.title, y, cx.time_s, duration));
    match list.style {
        MenuStyle::Dialog => parts.dialog(list),
        MenuStyle::Credits => parts.credits(list),
        MenuStyle::Rival => parts.rival_bio(list),
        MenuStyle::Blacklist => {
            let rows = BlacklistFrame::of(&parts.frame());
            let shown = list.entries.len().min(blacklist::ROWS);
            parts.blacklist(list, plate((list.selected < shown).then(|| rows.row_y(list.selected))));
        }
        MenuStyle::Main | MenuStyle::Sub | MenuStyle::Pause => {
            let plate_y = plate(selected_row(list).map(|row| parts.frame().row_y(row)));
            match ElementId::from_slot(slot) == Some(ElementId::Jukebox) {
                true => parts.jukebox(list, plate_y),
                false => parts.list(list, plate_y),
            }
        }
    }
    parts
}

/// The selected entry's place among the shown ones, or `None` when it is scrolled out of view.
fn selected_row(list: &MenuList) -> Option<usize> {
    let row = list.selected.checked_sub(list.first)?;
    (row < layout::shown(list.entries.len().saturating_sub(list.first))).then_some(row)
}

/// How a string sits on the point it is drawn at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Align {
    /// Its left edge on the point.
    Left,
    /// Its right edge on the point.
    Right,
    /// Its middle on the point.
    Centre,
}

/// The width of the box a string is laid out in. Wide enough for any menu string; the justification places it.
const TEXT_BOX: f32 = 640.0;

/// The parts of one screen, back to front, laid out against one frame.
pub(super) struct Parts<'a, 'b> {
    cx: &'a LayoutCx<'b>,
    frame: MenuFrame,
    parts: Vec<Box<dyn Component>>,
}

impl<'a, 'b> Parts<'a, 'b> {
    fn new(cx: &'a LayoutCx<'b>, frame: MenuFrame) -> Self {
        Self { cx, frame, parts: Vec::new() }
    }

    /// The frame the parts are placed in.
    pub(super) fn frame(&self) -> MenuFrame {
        self.frame
    }

    /// The layout context the parts are made against.
    pub(super) fn cx(&self) -> &'a LayoutCx<'b> {
        self.cx
    }

    fn finish(self) -> Vec<Box<dyn Component>> {
        self.parts
    }

    /// A string whose `align` side is at `(x, y)`, centred on `y` vertically.
    pub(super) fn text(&mut self, label: Label, (x, y): (f32, f32), align: Align) {
        let height = self.frame.sizing().units(40.0);
        let (centre, flags) = match align {
            Align::Left => (x + TEXT_BOX / 2.0, justify::V_CENTRE),
            Align::Right => (x - TEXT_BOX / 2.0, justify::H_RIGHT | justify::V_CENTRE),
            Align::Centre => (x, justify::H_CENTRE | justify::V_CENTRE),
        };
        let text = label.justify(flags).laid_out(Rect::new(centre, y, TEXT_BOX, height), self.frame.sizing(), self.cx);
        self.parts.push(Box::new(text));
    }

    /// A rectangle in the skin colour `key`, its alpha multiplied by `opacity`.
    pub(super) fn panel(&mut self, key: &'static str, rect: Rect, opacity: f32) {
        let skin = faded_skin(self.cx.skin, &[key], &[], opacity);
        let mut panel = Panel::new(key);
        panel.layout(&LayoutCx::new(&skin, self.cx.time_s), rect);
        self.parts.push(Box::new(panel));
    }

    /// A Custom plate over `rect` in `style`, at the menus' scale.
    pub(super) fn plate(&mut self, rect: Rect, style: PlateStyle) {
        let plate = Plate::new(rect, style.scaled(self.frame.scale()), self.cx);
        self.parts.push(Box::new(plate));
    }

    /// Adds a part that was laid out when it was made.
    pub(super) fn add(&mut self, part: impl Component + 'static) {
        self.parts.push(Box::new(part));
    }
}
