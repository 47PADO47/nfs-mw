//! The Original menu adapter: the whole tree of one front-end or career screen, as the frontend plugin last stored it in
//! the menu cell (`FrameCell::set_menu`).
//!
//! A menu slot is one screen's FEng package, and the slot takes the whole package. So the adapter emits every node of
//! the slot's tree (`Scope::Package`), in the tree's order, with parents remapped exactly as the chyron and the busted
//! overlay do (`subtree.rs`). It runs no script and changes no FEng state: the screen's logic still advances the package,
//! as it does today. An empty slot emits nothing.
//!
//! Interaction stays native. The adapter reads no input and sends no message, so `ScreenLogic`, `Cx`, `Command` and the
//! screen stack are unchanged. The design and the producer's mapping are in `docs/specs/ui-themes-menus.md`.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect, SlotId};

use super::frame::FrameCell;
use super::subtree::{Scope, emit};
use crate::ui::theme::ElementId;

/// The adapter for one menu slot. It holds the slot and the cell, and emits the slot's tree when it is built.
pub struct MenuTree {
    frame: FrameCell,
    slot: SlotId,
}

impl MenuTree {
    /// The adapter for `slot`, reading the menu cell of `frame`.
    pub fn new(frame: &FrameCell, slot: SlotId) -> Self {
        Self { frame: frame.clone(), slot }
    }
}

impl Component for MenuTree {
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        let Some(tree) = self.frame.get_menu(self.slot) else { return };
        emit(&tree, &Scope::Package, out);
    }
}

/// The adapter for a menu or career slot, or `None` for a slot this file does not cover (the HUD slots).
pub(super) fn adapter(frame: &FrameCell, slot: ElementId) -> Option<MenuTree> {
    match slot {
        ElementId::MainMenu
        | ElementId::Categories
        | ElementId::PauseMenu
        | ElementId::OptionTable
        | ElementId::TrackSelect
        | ElementId::SafeHouseMenu
        | ElementId::Dialog
        | ElementId::ControlsList
        | ElementId::Jukebox
        | ElementId::Credits
        | ElementId::Blacklist
        | ElementId::CarLot
        | ElementId::RapSheet
        | ElementId::RegionUnlock
        | ElementId::MemoryCard
        | ElementId::Sms => Some(MenuTree::new(frame, slot.slot())),
        _ => None,
    }
}
