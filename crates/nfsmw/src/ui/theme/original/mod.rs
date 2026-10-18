//! The Original theme: the look the game shipped with. It is the floor of the registry, so it must eventually
//! fill every slot in [`ElementId`](super::ElementId).
//!
//! Status (T6): scaffolding. [`OriginalTheme::element`] returns `None` for every slot, so a slot drawn through
//! this theme draws nothing until the adapters below land. Nothing reads the registry yet (T12 wires it in),
//! so the game still draws exactly as before.
//!
//! Plan for the adapters (T7 for the HUD slots, T8 for the menu slots). Each adapter wraps code that already
//! exists and changes no FEng object, script or viewport maths. The design, with its open point, is in
//! `docs/specs/ui-themes.md` section 8 ("Adapter design: emitting the FEng subtree").

use blackbox_ui::{BuildCx, Component, SlotId, Theme, ThemeId};

/// The Original theme. A unit struct for now: the adapters will hold the state they read from the existing code.
#[derive(Clone, Copy, Debug, Default)]
pub struct OriginalTheme;

impl Theme for OriginalTheme {
    fn id(&self) -> ThemeId {
        ThemeId::Original
    }

    fn element(&self, _slot: SlotId, _cx: &BuildCx<'_>) -> Option<Box<dyn Component>> {
        // T7 (HUD slots): one adapter per `HudBinding` group, each emitting its FEng subtree.
        // T8 (menu slots): adapters over `IconMenu`, `WidgetMenu` and the career screen logic.
        None
    }
}
