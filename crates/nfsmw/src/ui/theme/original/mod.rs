//! The Original theme: the look the game shipped with. It is the floor of the registry, so it must eventually fill every
//! slot in [`ElementId`](super::ElementId).
//!
//! Status (T8): every slot is filled, so the floor is complete. The HUD slots are [`FengSubtree`] adapters that emit the
//! FEng subtree of their element from the HUD package (or the chyron and busted overlay packages) as the plugin last
//! stored it in the HUD cell. The menu and career slots are [`menu::MenuTree`] adapters that emit the whole tree of
//! their screen as the plugin last stored it in the menu cell. Nothing resolves a slot in the live path yet (T12), so
//! the game still draws exactly as before.
//!
//! The design is in `docs/specs/ui-themes.md` section 8 (HUD) and `docs/specs/ui-themes-menus.md` (menus):
//!
//! - [`frame`]: the shared cells, and the producer rule (the plugin fills them, adapters only read).
//! - [`subtree`]: the HUD adapter and the copy rules (membership, hidden roots, draw order, parent remap).
//! - [`hud_slots`]: which adapter each HUD slot gets, and how the nested and overlapping parts are split.
//! - [`menu`]: the menu adapter, one per menu and career slot.
//!
//! The tests are in [`tests`] (HUD) and [`menu_tests`] (menus). They build synthetic trees, so they run with no game data
//! and no GPU.

mod frame;
mod hud_slots;
mod menu;
#[cfg(test)]
mod menu_tests;
mod subtree;
#[cfg(test)]
mod tests;

pub use frame::FrameCell;
// The HUD plugin names the packages it fills with this in T12.
#[allow(unused_imports)]
pub use frame::FengSource;

use blackbox_ui::{BuildCx, Component, SlotId, Theme, ThemeId};

use super::ElementId;

/// The Original theme. It holds the frame cells its adapters read from, so the cells must be the ones the plugins fill
/// (see [`super::UiThemes::frame`]).
#[derive(Clone)]
pub struct OriginalTheme {
    frame: FrameCell,
}

impl OriginalTheme {
    /// The Original theme reading its trees from `frame`.
    pub fn new(frame: FrameCell) -> Self {
        Self { frame }
    }
}

impl Theme for OriginalTheme {
    fn id(&self) -> ThemeId {
        ThemeId::Original
    }

    fn element(&self, slot: SlotId, _cx: &BuildCx<'_>) -> Option<Box<dyn Component>> {
        // A slot id outside the catalogue (a mod's, or a retired one) has no Original component.
        let element = ElementId::from_slot(slot)?;
        if let Some(adapter) = hud_slots::adapter(&self.frame, element) {
            return Some(Box::new(adapter));
        }
        let adapter = menu::adapter(&self.frame, element)?;
        Some(Box::new(adapter))
    }
}
