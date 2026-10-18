//! The Custom theme: the rewrite's own look. It overrides only the slots it has re-expressed as components and
//! returns `None` for the rest, so every other slot falls back to Original.
//!
//! Status (T6): scaffolding. [`CustomTheme::element`] returns `None` for every slot. The radio card
//! (`ElementId::RadioCard`) moves here in T9, drawn from `Panel`, `Text` and `Stack` as neutral nodes through the
//! presenter. Nothing is drawn in egui (`docs/specs/ui-themes.md` section 9).

use blackbox_ui::{BuildCx, Component, SlotId, Theme, ThemeId};

/// The Custom theme. A unit struct until T9 gives it the radio card's component.
#[derive(Clone, Copy, Debug, Default)]
pub struct CustomTheme;

impl Theme for CustomTheme {
    fn id(&self) -> ThemeId {
        ThemeId::Custom
    }

    fn element(&self, _slot: SlotId, _cx: &BuildCx<'_>) -> Option<Box<dyn Component>> {
        // T9: `ElementId::RadioCard` (the card from `hud/radio.rs`) returns here. Every other slot stays `None`.
        None
    }
}
