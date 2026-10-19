//! The token-driven theme: the rewrite's own look. It overrides only the slots it has re-expressed as components and
//! returns `None` for the rest, so every other slot falls back to Original.
//!
//! The palette it draws in is the settings' choice (`ui_theme`, see [`palette`]). The look reads its colours, corner
//! radius and padding from the skin ([`Tokens`](blackbox_ui::Tokens)), so a component does not name a palette.
//!
//! Status (T9): [`ElementId::RadioCard`] was the first slot. The card is [`RadioCard`]: a `Panel` and three `Text` rows
//! from `blackbox-ui`, emitted as neutral nodes through the presenter. Its state is fed by the plugin through
//! [`RadioFeedCell`], and the card draws nothing until then (`docs/specs/ui-themes-custom.md`).
//!
//! - [`radio_card`]: the radio card and its fade.
//! - [`radio_feed`]: the shared cell the plugin fills each frame.
//! - [`radio_placement`]: where the card sits on the window.
//! - [`skin`]: the fonts and status colours the components read, and the palettes' keys.
//! - [`palette`]: the two palettes.
//! - [`hud_component`]: the slot-to-component map, also used by a mod theme that declares tokens.

mod drive_feed;
mod drive_layout;
#[cfg(test)]
mod drive_tests;
mod gear;
mod meters;
pub mod palette;
mod radio_card;
mod radio_feed;
mod radio_placement;
mod skin;
mod speedometer;
mod tachometer;
#[cfg(test)]
mod tests;

pub use drive_feed::DriveFeedCell;
pub use gear::GearCard;
pub use radio_card::RadioCard;
pub use radio_feed::RadioFeedCell;
pub use skin::card_skin;
pub use speedometer::SpeedCard;
pub use tachometer::TachBar;

use blackbox_ui::{BuildCx, Component, SlotId, Theme, ThemeId, Tokens};

use crate::ui::theme::ElementId;
use meters::{Kind, Meter};

/// The token-driven theme: its id, the tokens it declares, and the feeds its components read.
#[derive(Clone, Debug)]
pub struct CustomTheme {
    id: ThemeId,
    tokens: Tokens,
    radio: RadioFeedCell,
    drive: DriveFeedCell,
}

impl CustomTheme {
    /// The Bevy theme (`ThemeId::Custom`), whose radio card reads `radio` and whose drive cluster reads `drive`.
    pub fn new(radio: RadioFeedCell, drive: DriveFeedCell) -> Self {
        Self { id: ThemeId::Custom, tokens: palette::bevy(), radio, drive }
    }
}

impl Theme for CustomTheme {
    fn id(&self) -> ThemeId {
        self.id
    }

    fn element(&self, slot: SlotId, _cx: &BuildCx<'_>) -> Option<Box<dyn Component>> {
        hud_component(slot, &self.radio, &self.drive)
    }

    fn tokens(&self) -> Tokens {
        self.tokens
    }
}

/// The component the token-driven look draws for `slot`, or `None` for a slot it does not re-express. Shared by the
/// built-in token themes and by a mod theme that declares tokens (`ui/theme/modded`).
pub fn hud_component(slot: SlotId, radio: &RadioFeedCell, drive: &DriveFeedCell) -> Option<Box<dyn Component>> {
    // A slot id outside the catalogue has no component, and neither does any slot the look does not re-express.
    match ElementId::from_slot(slot)? {
        ElementId::RadioCard => Some(Box::new(RadioCard::new(radio.clone()))),
        ElementId::Speedometer => Some(Box::new(SpeedCard::new(drive.clone()))),
        ElementId::Tachometer => Some(Box::new(TachBar::new(drive.clone()))),
        ElementId::GearIndicator => Some(Box::new(GearCard::new(drive.clone()))),
        ElementId::Nitrous => Some(Box::new(Meter::new(Kind::Nitrous, drive.clone()))),
        ElementId::Turbo => Some(Box::new(Meter::new(Kind::Boost, drive.clone()))),
        ElementId::Speedbreaker => Some(Box::new(Meter::new(Kind::Speedbreaker, drive.clone()))),
        _ => None,
    }
}
