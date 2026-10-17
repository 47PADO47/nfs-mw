//! The respawn marker: which collection the next world starts at (`docs/specs/career-safehouse-2.md` section 9,
//! `docs/specs/career-safehouse.md` sections 3.5, 3.6 and 4).
//!
//! The keys are 32-bit keys of collections of the gameplay data. This module only stores and selects keys; it does
//! not look at the data. Whether a collection exists is asked of the caller through a closure, and the string hash
//! behind the fallback key is the caller's too.

use super::zone::ZoneKind;
use crate::settings::{CareerSettings, PROLOGUE_STAGE};

/// The attribute of a zone collection that references its `respawn` child marker.
pub const ZONE_RESPAWN_ATTRIBUTE: u32 = 0x2241_F4CD;
/// The marker attribute that holds the position, three floats in game coordinates.
pub const MARKER_POSITION_ATTRIBUTE: u32 = 0x9F74_3A0E;
/// The marker attribute that holds the heading in degrees.
pub const MARKER_HEADING_ATTRIBUTE: u32 = 0x5A6A_57C6;
/// The factor the heading is multiplied by before the direction vector (0, 0, 1) is rotated by it.
pub const HEADING_FACTOR: f32 = -1.0 / 360.0;
/// The front-end platform id of the safe house.
pub const SAFEHOUSE_PLATFORM: u8 = 2;

/// The label of the start collection the world falls back to: `career_start_<region>`, with the region (the
/// track name of the loaded region) inserted as it is. The install has `career_start_l2ra`; the case of the
/// region string is the caller's concern.
pub fn career_start_label(region: &str) -> String {
    format!("career_start_{region}")
}

/// Game coordinates `(x, y, z)` to engine axes: `(-y, z, x)`.
pub const fn engine_position(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    (-y, z, x)
}

/// The rotation amount for a marker heading in degrees: `heading * (-1/360)`. The spec gives the factor and says
/// the direction vector (0, 0, 1) is rotated by this amount; it does not give the rotation axis or the unit
/// (presumably turns), so no direction vector is built here.
pub fn heading_rotation(heading_degrees: f32) -> f32 {
    heading_degrees * HEADING_FACTOR
}

/// Where the world loader takes the start point from: a debug start position wins,
/// then the current or pending event node, then the respawn marker of the gameplay manager. The built-in table
/// of positions is overwritten by all of them in career, so it is not a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartSource {
    /// The `SkipFEStartPosition` debug flag with a start position given.
    DebugOverride,
    /// The start position and heading of the current or pending event node.
    EventNode,
    /// The respawn marker selected by [`Respawn::select_key`].
    RespawnMarker,
}

/// Picks the start source (section 7.4).
pub const fn start_source(debug_override: bool, has_event_node: bool) -> StartSource {
    if debug_override {
        return StartSource::DebugOverride;
    }
    match has_event_node {
        true => StartSource::EventNode,
        false => StartSource::RespawnMarker,
    }
}

/// The respawn fields of the gameplay manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Respawn {
    /// `+0x108`: the marker of the last visited zone of any kind.
    pub home: u32,
    /// `+0x10C`: the marker of the last visited safe house.
    pub safe_house: u32,
    /// `+0x110`: use [`Respawn::safe_house`] instead of [`Respawn::home`] for the next world load.
    pub use_safe_house: bool,
    /// `+0x11C`: explicit override key. Nothing in the original writes it (unconfirmed); it is used first when its
    /// collection exists.
    pub override_key: u32,
}

impl Respawn {
    /// `DoZoneMenuAction`: stores the zone respawn key (the zone attribute, 0 when absent) as the last visited zone,
    /// and for a safe house also as the last visited safe house. `kind` is the type remembered by `EnterGateZone`,
    /// not the one of the collection. Does not touch the flag.
    pub fn visit_zone(&mut self, kind: ZoneKind, zone_respawn_key: u32) {
        self.home = zone_respawn_key;
        if kind == ZoneKind::SafeHouse {
            self.safe_house = zone_respawn_key;
        }
    }

    /// `DoSafeHouseIntro(marker)`: both keys become the key of the marker collection, the flag is set and the
    /// stage goes to 16. (The movie and the request are the caller's.)
    pub fn safe_house_intro(&mut self, settings: &mut CareerSettings, marker_key: u32) {
        self.home = marker_key;
        self.safe_house = marker_key;
        self.use_safe_house = true;
        settings.stage = PROLOGUE_STAGE;
    }

    /// The front-end request (section 4, career mode): with the platform 2 and no current event node the flag is
    /// set so the next load starts at the safe house marker. This covers the pause menu jump and `JumpToSafeHouse`.
    pub fn front_end_request(&mut self, platform: u8, has_event_node: bool) {
        if platform == SAFEHOUSE_PLATFORM && !has_event_node {
            self.use_safe_house = true;
        }
    }

    /// The key the next world uses (`select_respawn_key`, 9.3):
    ///
    /// 1. the override key when its collection exists;
    /// 2. else `safe_house` if the flag is set, otherwise `home`, when that collection exists;
    /// 3. else the hash of [`career_start_label`] for `region`, which also becomes `home`.
    ///
    /// `exists` answers whether a collection of the gameplay data has the key; `hash` is the game's name hash for
    /// collection keys.
    pub fn select_key(&mut self, region: &str, exists: impl Fn(u32) -> bool, hash: impl Fn(&str) -> u32) -> u32 {
        if exists(self.override_key) {
            return self.override_key;
        }
        let key = match self.use_safe_house {
            true => self.safe_house,
            false => self.home,
        };
        if exists(key) {
            return key;
        }
        let fallback = hash(&career_start_label(region));
        self.home = fallback;
        fallback
    }

    /// The load-complete callback of the world: the flag is cleared, so a later unload that does not go through
    /// the request uses [`Respawn::home`].
    pub fn world_loaded(&mut self) {
        self.use_safe_house = false;
    }

    /// The two keys the gameplay blob saves (tail offsets 0 and 16). The flag and the override are not saved.
    pub const fn saved_keys(&self) -> (u32, u32) {
        (self.home, self.safe_house)
    }

    /// Restores the saved keys; each is accepted only if its collection exists in the loaded data, otherwise the
    /// field keeps its value. The flag and the override are not changed.
    pub fn restore_saved(&mut self, home: u32, safe_house: u32, exists: impl Fn(u32) -> bool) {
        if exists(home) {
            self.home = home;
        }
        if exists(safe_house) {
            self.safe_house = safe_house;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAFE_A: u32 = 0xA1;
    const SAFE_B: u32 = 0xA2;
    const SHOP: u32 = 0xB1;
    const FALLBACK: u32 = 0xF0;

    fn exists(key: u32) -> bool {
        matches!(key, SAFE_A | SAFE_B | SHOP | FALLBACK)
    }

    fn hash(label: &str) -> u32 {
        assert_eq!(label, "career_start_l2ra");
        FALLBACK
    }

    #[test]
    fn the_attribute_keys_are_the_documented_ones() {
        assert_eq!(ZONE_RESPAWN_ATTRIBUTE, 0x2241F4CD);
        assert_eq!(MARKER_POSITION_ATTRIBUTE, 0x9F743A0E);
        assert_eq!(MARKER_HEADING_ATTRIBUTE, 0x5A6A57C6);
    }

    #[test]
    fn the_label_and_the_axis_mapping_follow_the_spec() {
        assert_eq!(career_start_label("l2ra"), "career_start_l2ra");
        // career_start_l2ra: (981.90, 2148.45, 152.29) in game coordinates.
        assert_eq!(engine_position(981.90, 2148.45, 152.29), (-2148.45, 152.29, 981.90));
        assert_eq!(heading_rotation(0.0), 0.0);
        assert_eq!(heading_rotation(360.0), -1.0);
        assert!((heading_rotation(53.08) - (-53.08 / 360.0)).abs() < 1e-6);
    }

    #[test]
    fn a_safe_house_visit_stores_both_keys_and_other_zones_only_the_home() {
        let mut r = Respawn::default();
        r.visit_zone(ZoneKind::SafeHouse, SAFE_A);
        assert_eq!((r.home, r.safe_house), (SAFE_A, SAFE_A));
        r.visit_zone(ZoneKind::CustomShop, SHOP);
        assert_eq!((r.home, r.safe_house), (SHOP, SAFE_A));
        r.visit_zone(ZoneKind::CarLot, 0);
        assert_eq!((r.home, r.safe_house), (0, SAFE_A), "a zone without the attribute stores 0");
    }

    #[test]
    fn coming_out_of_a_safe_house_uses_its_own_marker() {
        let mut r = Respawn::default();
        r.visit_zone(ZoneKind::SafeHouse, SAFE_B);
        r.front_end_request(2, false);
        assert!(r.use_safe_house);
        assert_eq!(r.select_key("l2ra", exists, hash), SAFE_B);
    }

    #[test]
    fn coming_out_of_a_car_lot_or_shop_uses_the_last_zone() {
        let mut r = Respawn::default();
        r.visit_zone(ZoneKind::SafeHouse, SAFE_A);
        r.visit_zone(ZoneKind::CustomShop, SHOP);
        r.front_end_request(3, false);
        assert!(!r.use_safe_house, "the request flag is only set for platform 2");
        assert_eq!(r.select_key("l2ra", exists, hash), SHOP);
    }

    #[test]
    fn a_request_with_an_event_node_does_not_set_the_flag() {
        let mut r = Respawn::default();
        r.front_end_request(2, true);
        assert!(!r.use_safe_house);
    }

    #[test]
    fn the_flag_is_cleared_when_the_world_has_loaded() {
        let mut r = Respawn::default();
        r.visit_zone(ZoneKind::SafeHouse, SAFE_A);
        r.visit_zone(ZoneKind::CustomShop, SHOP);
        r.front_end_request(2, false);
        assert_eq!(r.select_key("l2ra", exists, hash), SAFE_A);
        r.world_loaded();
        assert!(!r.use_safe_house);
        assert_eq!(r.select_key("l2ra", exists, hash), SHOP, "a later unload without the request uses home");
    }

    #[test]
    fn a_missing_collection_falls_back_to_the_career_start_and_remembers_it() {
        let mut r = Respawn::default();
        r.front_end_request(2, false);
        assert_eq!(r.select_key("l2ra", exists, hash), FALLBACK);
        assert_eq!(r.home, FALLBACK);
        let mut stale = Respawn { home: 0xDEAD, ..Respawn::default() };
        assert_eq!(stale.select_key("l2ra", exists, hash), FALLBACK);
    }

    #[test]
    fn the_override_wins_when_its_collection_exists() {
        let mut r = Respawn { override_key: SHOP, safe_house: SAFE_A, use_safe_house: true, ..Respawn::default() };
        assert_eq!(r.select_key("l2ra", exists, hash), SHOP);
        r.override_key = 0x9999;
        assert_eq!(r.select_key("l2ra", exists, hash), SAFE_A);
    }

    #[test]
    fn the_intro_sets_both_keys_the_flag_and_stage_16() {
        let mut settings = CareerSettings::new_career();
        settings.stage = 15;
        let mut r = Respawn::default();
        r.safe_house_intro(&mut settings, SAFE_A);
        assert_eq!((r.home, r.safe_house, r.use_safe_house), (SAFE_A, SAFE_A, true));
        assert_eq!(settings.stage, 16);
    }

    #[test]
    fn only_the_two_keys_are_saved_and_only_existing_ones_restored() {
        let mut r = Respawn { home: SHOP, safe_house: SAFE_A, use_safe_house: true, override_key: 5 };
        assert_eq!(r.saved_keys(), (SHOP, SAFE_A));
        let mut loaded = Respawn::default();
        loaded.restore_saved(SHOP, 0xBAD, exists);
        assert_eq!((loaded.home, loaded.safe_house), (SHOP, 0));
        assert!(!loaded.use_safe_house);
        r.restore_saved(SAFE_B, SAFE_B, exists);
        assert_eq!((r.home, r.safe_house, r.use_safe_house, r.override_key), (SAFE_B, SAFE_B, true, 5));
    }

    #[test]
    fn the_start_source_order_is_debug_node_marker() {
        assert_eq!(start_source(true, true), StartSource::DebugOverride);
        assert_eq!(start_source(false, true), StartSource::EventNode);
        assert_eq!(start_source(false, false), StartSource::RespawnMarker);
    }
}
