//! Mod themes: the themes a mod registered through `bb.ui`, as `blackbox_ui::Theme`s.
//!
//! The port (`modding/backends/ui.rs`) records each mod's slot specs and tokens as plain data. A [`ModTheme`] holds them,
//! and it has to be `Send + Sync` because the registry is a Bevy resource. Its `element` builds a [`ModSlot`] for a slot
//! the mod named, out of the base components of `blackbox-ui` ([`compose`]). A slot the mod did not name returns `None`.
//!
//! A mod that declared tokens also gets the built-in token look for the slots it did not name: [`ModHud`] draws them with
//! the mod's colours, radius and padding. Any other unnamed slot falls back to Original.
//!
//! Design: `docs/specs/ui-themes.md` section 12; the mod-facing catalogue: `docs/modding/api-ui.md`.

mod compose;
#[cfg(test)]
mod tests;

pub use compose::ModSlot;

use blackbox_modding::host::UiPrim;
use blackbox_ui::{BuildCx, Component, SlotId, Theme, ThemeId, Tokens};

use crate::ui::theme::custom::{DriveFeedCell, RadioFeedCell, hud_component};

/// A theme a mod registered: its id in the registry, its name and display text, and the draw spec of each slot it
/// overrides. A later [`ModTheme::set_slot`] for a slot replaces its spec.
#[derive(Debug, Clone, PartialEq)]
pub struct ModTheme {
    id: ThemeId,
    name: String,
    display: String,
    slots: Vec<(SlotId, Vec<UiPrim>)>,
    tokens: Option<Tokens>,
}

impl ModTheme {
    /// A theme with no slots yet. `id` is the registry id the port gave it.
    pub fn new(id: ThemeId, name: &str, display: &str) -> Self {
        Self { id, name: name.to_owned(), display: display.to_owned(), slots: Vec::new(), tokens: None }
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

impl ModTheme {
    /// Declare the tokens the mod chose: its colours, corner radius and padding. A second call replaces the first.
    pub fn set_tokens(&mut self, tokens: Tokens) {
        self.tokens = Some(tokens);
    }

    /// Whether the mod declared tokens, which makes the built-in token look available to its unnamed slots.
    pub fn declares_tokens(&self) -> bool {
        self.tokens.is_some()
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

    fn tokens(&self) -> Tokens {
        self.tokens.unwrap_or_default()
    }
}

/// A mod theme as the registry holds it: the [`ModTheme`] and the feeds the built-in token look reads. A slot the mod
/// named draws its spec. Any other slot draws the token look when the mod declared tokens, and Original otherwise.
pub struct ModHud {
    theme: ModTheme,
    radio: RadioFeedCell,
    drive: DriveFeedCell,
}

impl ModHud {
    /// The registry's entry for `theme`, reading the feeds the game fills each frame.
    pub fn new(theme: ModTheme, radio: RadioFeedCell, drive: DriveFeedCell) -> Self {
        Self { theme, radio, drive }
    }
}

impl Theme for ModHud {
    fn id(&self) -> ThemeId {
        self.theme.id()
    }

    fn element(&self, slot: SlotId, cx: &BuildCx<'_>) -> Option<Box<dyn Component>> {
        if let Some(component) = self.theme.element(slot, cx) {
            return Some(component);
        }
        match self.theme.declares_tokens() {
            true => hud_component(slot, &self.radio, &self.drive),
            false => None,
        }
    }

    fn tokens(&self) -> Tokens {
        self.theme.tokens()
    }
}
