//! The Custom theme: the rewrite's own look. It overrides only the slots it has re-expressed as components and
//! returns `None` for the rest, so every other slot falls back to Original.
//!
//! Status (T9): [`ElementId::RadioCard`] is the one slot Custom fills. The card is [`RadioCard`]: a `Panel` and three
//! `Text` rows from `blackbox-ui`, emitted as neutral nodes through the presenter. The egui card is retired
//! (`docs/specs/ui-themes-custom.md`). Its state is fed by the plugin through [`RadioFeedCell`], and the card draws
//! nothing until then.
//!
//! - [`radio_card`]: the component and its fade.
//! - [`radio_feed`]: the shared cell the plugin fills each frame.
//! - [`radio_placement`]: where the card sits on the window.
//! - [`skin`]: the card's colours and font, which [`card_skin`] hands to the registry's skin.

mod radio_card;
mod radio_feed;
mod radio_placement;
mod skin;
#[cfg(test)]
mod tests;

pub use radio_card::RadioCard;
pub use radio_feed::RadioFeedCell;
pub use skin::card_skin;

use blackbox_ui::{BuildCx, Component, SlotId, Theme, ThemeId};

use crate::ui::theme::ElementId;

/// The Custom theme. It holds the radio feed the card reads, so the cell must be the one the plugin fills (see
/// [`crate::ui::theme::UiThemes::radio`]).
#[derive(Clone, Debug, Default)]
pub struct CustomTheme {
    radio: RadioFeedCell,
}

impl CustomTheme {
    /// The Custom theme whose radio card reads `radio`.
    pub fn new(radio: RadioFeedCell) -> Self {
        Self { radio }
    }
}

impl Theme for CustomTheme {
    fn id(&self) -> ThemeId {
        ThemeId::Custom
    }

    fn element(&self, slot: SlotId, _cx: &BuildCx<'_>) -> Option<Box<dyn Component>> {
        // A slot id outside the catalogue has no Custom component, and neither does any slot but the radio card.
        match ElementId::from_slot(slot)? {
            ElementId::RadioCard => Some(Box::new(RadioCard::new(self.radio.clone()))),
            _ => None,
        }
    }
}
