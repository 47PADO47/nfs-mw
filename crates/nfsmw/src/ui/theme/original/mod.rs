//! The Original theme: the look the game shipped with. It is the floor of the registry, so it must eventually fill every
//! slot in [`ElementId`](super::ElementId).
//!
//! Status (T12): every slot is filled, and the presenters resolve them every frame through the composer
//! (`ui/theme/compose.rs`). The HUD slots are [`FengSubtree`] adapters that emit the FEng subtree of their element from
//! the HUD package (or the chyron and busted overlay packages) as the plugin last stored it in the HUD cell. The menu and
//! career slots are [`menu::MenuTree`] adapters that emit the whole tree of their screen as the plugin last stored it in
//! the menu cell. The HUD nodes no slot takes are appended by [`emit_unclaimed`].
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
mod structure;
#[cfg(test)]
mod structure_tests;
mod subtree;
#[cfg(test)]
mod tests;

pub use frame::{FengSource, FrameCell};
#[cfg(test)]
pub(crate) use hud_slots::claim_counts;
pub(crate) use hud_slots::{HUD_SLOTS, emit_unclaimed};
pub(crate) use subtree::FengSubtree;

use blackbox_ui::{BuildCx, Component, Empty, SlotId, Theme, ThemeId};

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
        // The world map's chrome is an overlay only alternative themes draw. Original leaves it empty, so the FEng map
        // screen keeps drawing its own list, card, tab and zoom.
        if element == ElementId::WorldMapChrome {
            return Some(Box::new(Empty));
        }
        if let Some(adapter) = hud_slots::adapter(&self.frame, element) {
            return Some(Box::new(adapter));
        }
        let adapter = menu::adapter(&self.frame, element)?;
        Some(Box::new(adapter))
    }
}
