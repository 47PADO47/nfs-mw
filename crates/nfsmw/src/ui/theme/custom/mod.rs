//! The Custom theme: the rewrite's own look, rebuilt from scratch as the Custom HUD 2026 (ADR 0013,
//! `docs/decisions/0013-custom-hud.md`; the design is `docs/specs/custom-hud/`). It overrides only the slots it has
//! re-expressed as components and returns `None` for the rest, so every other slot falls back to Original.
//!
//! Status (PR 5): the foundation. The old bottom-right drive card is gone; the horizontal telemetry cluster, the
//! centred map, the free-roam, race and pursuit layouts and the world markers come in the next PRs of the stack, and
//! until then the drive slots fall back to Original. What is built here is what those components share:
//!
//! - [`tokens`]: the colours (`custom.*` skin keys), high contrast, the colour-vision palettes, opacity and the motion
//!   table. `ui_theme` still chooses the accent family ([`palette`]).
//! - [`layout`]: the 16:9 content box, the safe area, the anchors and the HUD scale.
//! - [`feed`]: [`CustomFeedCell`], the one cell every Custom component reads, filled by the HUD plugin each frame.
//! - [`plate`]: the translucent charcoal plate with its optional accent line.
//! - [`text`]: text in the three font roles, sized by the HUD scale and `text_size`.
//! - `notify`: the `Notifications` slot, the notification stack and the feedback lane's chip.
//!
//! Built on it (PR 6): [`cluster`], the horizontal telemetry cluster, which fills the drive slots.
//!
//! Kept from before the rewrite until their own PRs replace them:
//!
//! - [`radio_card`], [`radio_feed`], [`radio_placement`]: the radio card, its feed and its place.
//! - [`map_legend`]: the world map's legend, drawn over the recoloured FEng map (the `WorldMapChrome` slot).
//! - [`drive_feed`]: the drive state the Remastered theme reads ([`DriveFeedCell`]).
//! - [`skin`]: the radio card's and the legend's fonts; [`palette`]: the two palettes `ui_theme` picks.
//! - [`hud_component`]: the slot-to-component map, also used by a mod theme that declares tokens.
//!
//! The HUD components:
//!
//! - [`markers`]: the world-space markers (the `WorldMarkers` slot).

mod cluster;
mod drive_feed;
// The Custom components that read the feed, the layout, the plate and the text helpers land in PRs 6 to 9.
#[allow(dead_code)]
pub mod feed;
#[cfg(test)]
mod helpers_tests;
#[allow(dead_code)]
pub mod layout;
#[cfg(test)]
mod layout_tests;
mod map_legend;
mod markers;
mod notify;
pub mod mode;
pub mod palette;
#[allow(dead_code)]
pub mod plate;
mod radio_card;
mod radio_feed;
mod radio_placement;
mod skin;
#[cfg(test)]
mod tests;
#[allow(dead_code)]
pub mod text;
#[allow(dead_code)]
pub mod tokens;

pub(in crate::ui::theme) use drive_feed::DriveFeed;
pub use drive_feed::DriveFeedCell;
pub use feed::{CustomFeed, CustomFeedCell};
pub use map_legend::{MapLegend, WorldMapLegendCell};
pub use radio_card::RadioCard;
pub use radio_feed::RadioFeedCell;
pub use skin::card_skin;

use blackbox_ui::{BuildCx, Component, Empty, SlotId, Theme, ThemeId, Tokens};

use crate::ui::theme::ElementId;
use cluster::Cluster;

/// The token-driven theme: its id, the tokens it declares, and the feeds its components read.
#[derive(Clone, Debug)]
pub struct CustomTheme {
    id: ThemeId,
    tokens: Tokens,
    radio: RadioFeedCell,
    feed: CustomFeedCell,
    legend: WorldMapLegendCell,
}

impl CustomTheme {
    /// The token-driven theme (`ThemeId::Custom`), whose radio card reads `radio`, whose HUD components read `feed` and
    /// whose map legend reads `legend`.
    pub fn new(radio: RadioFeedCell, feed: CustomFeedCell, legend: WorldMapLegendCell) -> Self {
        Self { id: ThemeId::Custom, tokens: palette::bevy(), radio, feed, legend }
    }
}

impl Theme for CustomTheme {
    fn id(&self) -> ThemeId {
        self.id
    }

    fn element(&self, slot: SlotId, _cx: &BuildCx<'_>) -> Option<Box<dyn Component>> {
        // The map: the FEng screen is recoloured in place by the frontend, and this legend is drawn over it. Providing
        // the slot is also what tells the frontend to recolour.
        if ElementId::from_slot(slot) == Some(ElementId::WorldMapChrome) {
            return Some(Box::new(MapLegend::new(self.legend.clone())));
        }
        hud_component(slot, &self.radio, &self.feed)
    }

    fn tokens(&self) -> Tokens {
        self.tokens
    }
}

/// The component the token-driven look draws for `slot`, or `None` for a slot it does not re-express. Shared by the
/// built-in token theme and by a mod theme that declares tokens (`ui/theme/modded`).
///
/// The HUD components read `feed`. A slot the look does not re-express returns `None`, so it falls back to Original.
pub fn hud_component(slot: SlotId, radio: &RadioFeedCell, feed: &CustomFeedCell) -> Option<Box<dyn Component>> {
    // A slot id outside the catalogue has no component, and neither does any slot the look does not re-express.
    match ElementId::from_slot(slot)? {
        ElementId::RadioCard => Some(Box::new(RadioCard::new(radio.clone()))),
        // The telemetry cluster draws the speed, the revs, the gear and the bars from one slot (PR 6).
        ElementId::Speedometer => Some(Box::new(Cluster::new(feed.clone()))),
        ElementId::Tachometer => Some(Box::new(Empty)),
        ElementId::GearIndicator => Some(Box::new(Empty)),
        ElementId::Nitrous => Some(Box::new(Empty)),
        ElementId::Turbo => Some(Box::new(Empty)),
        ElementId::Speedbreaker => Some(Box::new(Empty)),
        ElementId::WorldMarkers => Some(Box::new(markers::WorldMarkers::new(feed.clone()))),
        ElementId::Notifications => Some(Box::new(notify::Notifications::new(feed.clone()))),
        _ => None,
    }
}
