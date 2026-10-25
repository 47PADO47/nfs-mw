//! The Custom theme: the rewrite's own look, rebuilt from scratch as the Custom HUD 2026 (ADR 0013,
//! `docs/decisions/0013-custom-hud.md`; the design is `docs/specs/custom-hud/`). It draws the whole HUD but the busted
//! overlay, the menus, the LAN screens and the world map with its own components; any other slot falls back to Original.
//!
//! What every component shares:
//!
//! - [`tokens`]: the colours (`custom.*` skin keys), high contrast, the colour-vision palettes, opacity and the motion
//!   table. `ui_theme` still chooses the accent family ([`palette`]).
//! - [`layout`]: the 16:9 content box, the safe area, the anchors and the HUD scale.
//! - [`feed`]: [`CustomFeedCell`], the one cell the HUD components read, filled by the HUD plugin each frame, with the
//!   memories the components keep between frames.
//! - [`plate`], [`text`], [`parts`], [`words`]: the translucent plate, text in the three font roles, the shapes and
//!   key caps the layouts draw with, and their shared words.
//!
//! The HUD ([`hud_component`], shared with a mod theme that declares tokens):
//!
//! - [`cluster`]: the horizontal telemetry cluster (the drive slots).
//! - [`map`]: the centred translucent map, drawn from the game's own map picture and icons (`Minimap`; `Blips` is left
//!   empty), and its turn-cue chip.
//! - [`mode`] and [`memory`]: the contextual mode (free roam, race, pursuit) with its crossfade, and what the layouts
//!   remember across frames.
//! - [`free_roam`], [`race`], [`pursuit`], [`banner`], [`condition`]: the discovery chips and extras, the race strip and
//!   leaderboard, the pursuit panel and end cards, the transient banner and the vehicle condition chip.
//! - [`markers`]: the world-space markers and the police threat indicators (`WorldMarkers`).
//! - [`notify`]: the notification stack and the feedback lane (`Notifications`).
//! - [`radio_card`], [`radio_placement`]: the radio card and its place; [`radio_feed`] the song it reads, which the
//!   Remastered card shares.
//!
//! The screens:
//!
//! - [`menu`]: the menus, drawn from the lists the frontend publishes ([`MenuListCell`]), and the LAN page ([`LanCell`]).
//! - [`lan_hud`]: the LAN session's HUD pieces.
//! - [`full_map`]: the full-screen world map (`WorldMapChrome`), drawn with the centred map's roads, routes and marks.
//!
//! Kept for the other looks: [`drive_feed`], the drive state the Remastered theme reads ([`DriveFeedCell`]); [`skin`],
//! the map's icons.

mod banner;
mod cluster;
mod condition;
mod drive_feed;
pub mod feed;
mod free_roam;
pub mod full_map;
#[cfg(test)]
mod helpers_tests;
mod lan_hud;
#[cfg(test)]
mod lan_tests;
pub mod layout;
#[cfg(test)]
mod layout_tests;
pub mod map;
mod markers;
mod memory;
pub mod menu;
pub mod mode;
mod notify;
pub mod palette;
mod parts;
pub mod plate;
mod pursuit;
mod race;
mod radio_card;
mod radio_feed;
mod radio_placement;
mod skin;
#[cfg(test)]
mod tests;
pub mod text;
pub mod tokens;
mod words;

pub(in crate::ui::theme) use drive_feed::DriveFeed;
pub use drive_feed::DriveFeedCell;
pub use feed::{CustomFeed, CustomFeedCell};
pub use full_map::{FullMap, FullMapCell, FullMapSnapshot};
pub use radio_card::RadioCard;
pub use radio_feed::RadioFeedCell;
pub use skin::card_skin;

use blackbox_ui::{BuildCx, Component, Empty, SlotId, Theme, ThemeId, Tokens};

use crate::ui::theme::original::FrameCell;
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
    /// The open world map, as the frontend publishes it.
    map: FullMapCell,
    /// The lists the menus draw (filled by the frontend for the list looks), and where each menu's selection plate is.
    menus: MenuListCell,
    glide: GlideCell,
    /// The LAN page and session HUD (filled by the LAN front end).
    lan: LanCell,
    /// The HUD package's frame, where the centred map finds the game's icons. `None` draws shapes.
    frame: Option<FrameCell>,
}

impl CustomTheme {
    /// The token-driven theme (`ThemeId::Custom`), whose radio card reads `radio`, whose HUD components read `feed` and
    /// whose full-screen world map reads `map`.
    pub fn new(radio: RadioFeedCell, feed: CustomFeedCell, map: FullMapCell) -> Self {
        Self {
            id: ThemeId::Custom,
            tokens: palette::bevy(),
            radio,
            feed,
            map,
            menus: MenuListCell::new(),
            glide: GlideCell::default(),
            lan: LanCell::default(),
            frame: None,
        }
    }

    /// This theme with the centred map drawing the game's own icons, found in `frame`'s HUD tree.
    pub fn with_frame(self, frame: FrameCell) -> Self {
        Self { frame: Some(frame), ..self }
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
        // The world map: the theme draws all of it, full screen, and the frontend does not present the FEng screen.
        // Providing the slot is what tells the frontend so (`UiThemes::world_map_chrome`).
        if ElementId::from_slot(slot) == Some(ElementId::WorldMapChrome) {
            return Some(Box::new(FullMap::new(self.map.clone())));
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
        hud_component(slot, &self.radio, &self.feed, self.frame.as_ref())
    }

    fn tokens(&self) -> Tokens {
        self.tokens
    }
}

/// The component the token-driven look draws for `slot`, or `None` for a slot it does not re-express. Shared by the
/// built-in token theme and by a mod theme that declares tokens (`ui/theme/modded`).
///
/// The HUD components read `feed`; the map takes the game's icons from `frame` when it has one. A slot the look does not
/// re-express returns `None`, so it falls back to Original.
pub fn hud_component(
    slot: SlotId,
    radio: &RadioFeedCell,
    feed: &CustomFeedCell,
    frame: Option<&FrameCell>,
) -> Option<Box<dyn Component>> {
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
        ElementId::Minimap => {
            let map = map::CustomMap::new(feed.clone());
            Some(Box::new(match frame {
                Some(frame) => map.with_frame(frame.clone()),
                None => map,
            }))
        }
        ElementId::Blips => Some(Box::new(Empty)),
        _ => None,
    }
}
