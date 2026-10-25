//! The Custom theme: the rewrite's own look, rebuilt from scratch as the Custom HUD 2026 (ADR 0013,
//! `docs/decisions/0013-custom-hud.md`; the design is `docs/specs/custom-hud/`). It overrides only the slots it has
//! re-expressed as components and returns `None` for the rest, so every other slot falls back to Original.
//!
//! Status (PR 5): the foundation. The old bottom-right drive card is gone; the horizontal telemetry cluster, the
//! centred map and the world markers come in the next PRs of the stack, and until then the drive slots fall back to
//! Original. What is built here is what those components share:
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
//! The contextual layouts (PR 8, `docs/specs/custom-hud/modes.md`, `race.md`, `pursuit.md`):
//!
//! - [`mode`]: the resolver from the HUD state to free roam, the race modes, a pursuit phase or hidden, and the
//!   crossfade clock.
//! - [`memory`]: what the layouts remember across frames (the mode clock, the discovery chips, the condition chip's
//!   last warning, the moments a place or a heat level changed), carried by the feed cell.
//! - [`free_roam`]: the discovery chips, the entry card and the optional extras line (the `BountyRow` slot).
//! - [`race`]: the status strip (`RaceStatus`), the compact leaderboard (`Leaderboard`), the intro card and countdown.
//! - [`pursuit`]: the pursuit panel (`PursuitMeter`) and the end-of-chase cards.
//! - [`banner`]: the `Banner` slot's transient cards; [`condition`]: the vehicle condition chip (`VehicleCondition`).
//! - [`parts`]: the plates, labels, bars, icons and key caps they draw with; [`words`]: their shared words.
//! - [`map`]: the centred translucent map (the `Minimap` slot; `Blips` is left empty) and its turn-cue chip.
//! - [`menu`]: the menus (PR 11a, 11b): the main menu, the option categories, the pause menu, the safe house, the option
//!   pages, the dialogs, the key list, the jukebox, the credits and the Blacklist, drawn from the lists the frontend
//!   publishes ([`MenuListCell`]), and the LAN page, drawn from the LAN cell ([`LanCell`]).
//! - [`lan_hud`]: the LAN session's HUD pieces (`LanHud`), from the LAN cell.
//! - [`radio_card`], [`radio_placement`]: the radio card in the Custom style (PR 11b) and its place; [`radio_feed`] the
//!   song it reads, which the Remastered card shares.
//!
//! Kept from before the rewrite until their own PRs replace them:
//!
//! - [`map_legend`]: the world map's legend, drawn over the recoloured FEng map (the `WorldMapChrome` slot).
//! - [`drive_feed`]: the drive state the Remastered theme reads ([`DriveFeedCell`]).
//! - [`skin`]: the legend's font and the map's icons; [`palette`]: the two palettes `ui_theme` picks.
//! - [`hud_component`]: the slot-to-component map, also used by a mod theme that declares tokens.
//!
//! The HUD components:
//!
//! - [`markers`]: the world-space markers (the `WorldMarkers` slot).

mod cluster;
mod banner;
mod condition;
mod drive_feed;
// The Custom components that read the feed, the layout, the plate and the text helpers land in PRs 6 to 9.
#[allow(dead_code)]
pub mod feed;
mod free_roam;
#[cfg(test)]
mod helpers_tests;
mod lan_hud;
#[cfg(test)]
mod lan_tests;
#[allow(dead_code)]
pub mod layout;
#[cfg(test)]
mod layout_tests;
pub mod map;
mod map_legend;
mod markers;
mod notify;
mod memory;
pub mod mode;
pub mod menu;
pub mod palette;
mod parts;
#[allow(dead_code)]
pub mod plate;
mod pursuit;
mod race;
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
mod words;

pub(in crate::ui::theme) use drive_feed::DriveFeed;
pub use drive_feed::DriveFeedCell;
pub use feed::{CustomFeed, CustomFeedCell};
pub use map_legend::{MapLegend, WorldMapLegendCell};
pub use radio_card::RadioCard;
pub use radio_feed::RadioFeedCell;
pub use skin::card_skin;

use blackbox_ui::{BuildCx, Component, Empty, SlotId, Theme, ThemeId, Tokens};

use crate::ui::theme::{ElementId, LanCell, MenuListCell};
use cluster::Cluster;
use lan_hud::LanHudParts;
use menu::{CustomMenu, GlideCell, LanMenu};

/// The token-driven theme: its id, the tokens it declares, and the feeds its components read.
#[derive(Clone, Debug)]
pub struct CustomTheme {
    id: ThemeId,
    tokens: Tokens,
    radio: RadioFeedCell,
    feed: CustomFeedCell,
    legend: WorldMapLegendCell,
    /// The lists the menus draw (filled by the frontend for the list looks), and where each menu's selection plate is.
    menus: MenuListCell,
    glide: GlideCell,
    /// The LAN page and session HUD (filled by the LAN front end).
    lan: LanCell,
}

impl CustomTheme {
    /// The token-driven theme (`ThemeId::Custom`), whose radio card reads `radio`, whose HUD components read `feed` and
    /// whose map legend reads `legend`.
    pub fn new(radio: RadioFeedCell, feed: CustomFeedCell, legend: WorldMapLegendCell) -> Self {
        Self {
            id: ThemeId::Custom,
            tokens: palette::bevy(),
            radio,
            feed,
            legend,
            menus: MenuListCell::new(),
            glide: GlideCell::default(),
            lan: LanCell::default(),
        }
    }

    /// This theme with its menus drawn from `menus`, the cell the frontend fills each frame.
    pub fn with_menus(self, menus: MenuListCell) -> Self {
        Self { menus, ..self }
    }

    /// This theme with its LAN page and session HUD drawn from `lan`, the cell the LAN front end fills each frame.
    pub fn with_lan(self, lan: LanCell) -> Self {
        Self { lan, ..self }
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
        // The LAN screens read the LAN cell: the page as a Custom list, the session's HUD pieces on Custom plates.
        if ElementId::from_slot(slot) == Some(ElementId::LanMenu) {
            return Some(Box::new(LanMenu::new(self.lan.clone(), self.glide.clone())));
        }
        if ElementId::from_slot(slot) == Some(ElementId::LanHud) {
            return Some(Box::new(LanHudParts::new(self.lan.clone(), self.feed.clone())));
        }
        // The menus are the Custom theme's own; the HUD components below are shared with the mod themes.
        if menu::draws(slot) {
            return Some(Box::new(CustomMenu::new(slot, self.menus.clone(), self.glide.clone())));
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
        ElementId::RadioCard => Some(Box::new(RadioCard::new(radio.clone(), feed.clone()))),
        // The telemetry cluster draws the speed, the revs, the gear and the bars from one slot (PR 6).
        ElementId::Speedometer => Some(Box::new(Cluster::new(feed.clone()))),
        ElementId::Tachometer => Some(Box::new(Empty)),
        ElementId::GearIndicator => Some(Box::new(Empty)),
        ElementId::Nitrous => Some(Box::new(Empty)),
        ElementId::Turbo => Some(Box::new(Empty)),
        ElementId::Speedbreaker => Some(Box::new(Empty)),
        ElementId::WorldMarkers => Some(Box::new(markers::WorldMarkers::new(feed.clone()))),
        ElementId::Notifications => Some(Box::new(notify::Notifications::new(feed.clone()))),
        ElementId::RaceStatus => Some(Box::new(race::RaceStrip::new(feed.clone()))),
        ElementId::Leaderboard => Some(Box::new(race::Leaderboard::new(feed.clone()))),
        ElementId::Banner => Some(Box::new(banner::Banner::new(feed.clone()))),
        ElementId::PursuitMeter => Some(Box::new(pursuit::PursuitPanel::new(feed.clone()))),
        // The bounty is folded into the pursuit panel; the slot draws the free-roam extras.
        ElementId::BountyRow => Some(Box::new(free_roam::Extras::new(feed.clone()))),
        // The map and the edge arrows show what the player can know of the police.
        ElementId::PoliceRadar => Some(Box::new(Empty)),
        ElementId::VehicleCondition => Some(Box::new(condition::ConditionChip::new(feed.clone()))),
        ElementId::Minimap => Some(Box::new(map::CustomMap::new(feed.clone()))),
        ElementId::Blips => Some(Box::new(Empty)),
        _ => None,
    }
}
