//! The Original theme: the look the game shipped with. It is the floor of the registry, so it must eventually fill every
//! slot in [`ElementId`](super::ElementId).
//!
//! Status (T7): the HUD slots are filled. Each one is a [`FengSubtree`] adapter that emits the FEng subtree of its
//! element from the HUD package (or the chyron and busted overlay packages) as the plugin last stored it in the
//! [`FrameCell`]. The menu slots are still `None` (T8). Nothing resolves a slot in the live path yet (T12), so the
//! game still draws exactly as before.
//!
//! The design, with the decisions made in T7, is in `docs/specs/ui-themes.md` section 8:
//!
//! - [`frame`]: the shared cell, and the producer rule (the plugin fills it, adapters only read).
//! - [`subtree`]: the adapter and the copy rules (membership, hidden roots, draw order, parent remap).
//! - [`hud_slots`]: which adapter each HUD slot gets, and how the nested and overlapping parts are split.
//!
//! The tests are in [`tests`]. They build a synthetic tree, so they run with no game data and no GPU.

mod frame;
mod hud_slots;
mod subtree;
#[cfg(test)]
mod tests;

pub use frame::FrameCell;
// The HUD plugin names the packages it fills with this in T12.
#[allow(unused_imports)]
pub use frame::FengSource;

use blackbox_ui::{BuildCx, Component, SlotId, Theme, ThemeId};

use super::ElementId;

/// The Original theme. It holds the frame cell its HUD adapters read from, so the cell must be the one the plugin
/// fills (see [`super::UiThemes::frame`]).
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
        // The menu slots return `None` here until T8 fills them.
        let adapter = hud_slots::adapter(&self.frame, element)?;
        Some(Box::new(adapter))
    }
}
