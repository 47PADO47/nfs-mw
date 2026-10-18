//! Mod themes: the themes a mod registered through `bb.ui`, as `blackbox_ui::Theme`s.
//!
//! The port (`modding/backends/ui.rs`) records each mod's slot specs as plain data. A [`ModTheme`] holds them, and it has
//! to be `Send + Sync` because the registry is a Bevy resource. Its `element` builds a [`ModSlot`] for a slot the mod
//! named, out of the base components of `blackbox-ui` ([`compose`]). A slot the mod did not name returns `None`, so the
//! registry falls back to Original for it.
//!
//! Design: `docs/specs/ui-themes.md` section 12; the mod-facing catalogue: `docs/modding/api-ui.md`.

mod compose;
#[cfg(test)]
mod tests;

pub use compose::ModSlot;

use blackbox_modding::host::UiPrim;
use blackbox_ui::{BuildCx, Component, SlotId, Theme, ThemeId};

/// A theme a mod registered: its id in the registry, its name and display text, and the draw spec of each slot it
/// overrides. A later [`ModTheme::set_slot`] for a slot replaces its spec.
#[derive(Debug, Clone, PartialEq)]
pub struct ModTheme {
    id: ThemeId,
    name: String,
    display: String,
    slots: Vec<(SlotId, Vec<UiPrim>)>,
}

impl ModTheme {
    /// A theme with no slots yet. `id` is the registry id the port gave it.
    pub fn new(id: ThemeId, name: &str, display: &str) -> Self {
        Self { id, name: name.to_owned(), display: display.to_owned(), slots: Vec::new() }
    }

    /// The theme's name, unique among the theme's mod.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The text the Mods page shows for the theme.
    pub fn display(&self) -> &str {
        &self.display
    }

    /// Give `slot` the draw spec `prims`, replacing the spec it had.
    pub fn set_slot(&mut self, slot: SlotId, prims: Vec<UiPrim>) {
        match self.slots.iter_mut().find(|(known, _)| *known == slot) {
            Some((_, spec)) => *spec = prims,
            None => self.slots.push((slot, prims)),
        }
    }
}

impl Theme for ModTheme {
    fn id(&self) -> ThemeId {
        self.id
    }

    fn element(&self, slot: SlotId, _cx: &BuildCx<'_>) -> Option<Box<dyn Component>> {
        let (_, prims) = self.slots.iter().find(|(known, _)| *known == slot)?;
        Some(Box::new(ModSlot::new(prims)))
    }
}
