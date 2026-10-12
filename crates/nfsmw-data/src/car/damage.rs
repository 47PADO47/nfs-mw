//! Which impact zone drives which visible damage of a car: the corner overlays (`DAMAGE0_*`
//! slots) and whether a zone has glazing. Zone indices follow [`super::physics::DAMAGE_ZONES`].
//! Spec: `docs/specs/vehicle-damage.md`.

use std::collections::BTreeSet;

use blackbox_damage::Damage;

use super::slot;

/// The corner overlay slot each damage zone shows once its panel is dented, or `None` for zones
/// without a corner overlay (the sides, the roof and the underbody).
pub const ZONE_OVERLAY_SLOT: [Option<usize>; 10] = [
    Some(slot::DAMAGE0_FRONT),       // DZ_FRONT
    Some(slot::DAMAGE0_REAR),        // DZ_REAR
    None,                            // DZ_LEFT
    None,                            // DZ_RIGHT
    Some(slot::DAMAGE0_FRONT_LEFT),  // DZ_LFRONT
    Some(slot::DAMAGE0_FRONT_RIGHT), // DZ_RFRONT
    Some(slot::DAMAGE0_REAR_LEFT),   // DZ_LREAR
    Some(slot::DAMAGE0_REAR_RIGHT),  // DZ_RREAR
    None,                            // DZ_TOP
    None,                            // DZ_BOTTOM
];

/// Zones with glazing that can crack: every side and corner zone, not the roof or underbody.
pub const ZONE_HAS_GLASS: [bool; 10] = [true, true, true, true, true, true, true, true, false, false];

/// The wheel each corner zone's tire is (0 front left, 1 front right, 2 rear left, 3 rear right).
pub const ZONE_WHEEL: [Option<usize>; 10] = [None, None, None, None, Some(0), Some(1), Some(2), Some(3), None, None];

/// The impact severity from which a corner zone punctures its tire: the top level. A port choice; the
/// original punctures tires from spike strips, which this port does not have yet.
pub const PUNCTURE_SEVERITY: usize = 6;

/// The window placeholders a glazed zone covers: the windscreen, the rear window or the side windows at
/// that corner. Empty for zones without glass.
pub fn zone_windows(zone: usize) -> &'static [&'static str] {
    match zone {
        0 => &["WINDOW_FRONT"],
        1 => &["WINDOW_REAR"],
        2 => &["WINDOW_LEFT_FRONT", "WINDOW_LEFT_REAR"],
        3 => &["WINDOW_RIGHT_FRONT", "WINDOW_RIGHT_REAR"],
        4 => &["WINDOW_LEFT_FRONT"],
        5 => &["WINDOW_RIGHT_FRONT"],
        6 => &["WINDOW_LEFT_REAR"],
        7 => &["WINDOW_RIGHT_REAR"],
        _ => &[],
    }
}

/// The window solid of a glazed corner or the rear zone: its name ends with this (the body's own glass is
/// a placeholder solid, see `zone_windows`). Zones without one return `None`.
pub fn zone_window_solid(zone: usize) -> Option<&'static str> {
    match zone {
        1 => Some("KIT00_REAR_WINDOW_A"),
        0 => Some("KIT00_FRONT_WINDOW_A"),
        4 => Some("KIT00_FRONT_LEFT_WINDOW_A"),
        5 => Some("KIT00_FRONT_RIGHT_WINDOW_A"),
        6 => Some("KIT00_REAR_LEFT_WINDOW_A"),
        7 => Some("KIT00_REAR_RIGHT_WINDOW_A"),
        _ => None,
    }
}

/// Whether `slot` holds a corner overlay. These are placed on every car but drawn only once their
/// zone is damaged, so views of an undamaged car skip them.
pub fn is_corner_overlay(slot: usize) -> bool {
    slot::DAMAGE0.contains(&slot)
}

/// The corner overlay slots to draw: those of zones whose panel has dented.
pub fn revealed_overlays(damage: &Damage) -> BTreeSet<usize> {
    (0..damage.zone_count())
        .filter(|&zone| damage.zone(zone).panel > 0)
        .filter_map(|zone| ZONE_OVERLAY_SLOT.get(zone).copied().flatten())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::car::physics::DAMAGE_ZONES;

    #[test]
    fn zone_tables_match_the_zone_order() {
        assert_eq!(DAMAGE_ZONES[0], "DZ_FRONT");
        assert_eq!(ZONE_OVERLAY_SLOT[0], Some(slot::DAMAGE0_FRONT));
        assert_eq!(ZONE_OVERLAY_SLOT[7], Some(slot::DAMAGE0_REAR_RIGHT));
        assert!(ZONE_OVERLAY_SLOT.iter().flatten().all(|s| slot::DAMAGE0.contains(s)));
        assert!(!ZONE_HAS_GLASS[8] && !ZONE_HAS_GLASS[9]);
    }

    #[test]
    fn only_dented_zones_with_an_overlay_reveal_it() {
        let mut damage = Damage::new(DAMAGE_ZONES.len());
        assert!(revealed_overlays(&damage).is_empty());
        // Light contact scrapes paint but does not dent: no overlay yet.
        damage.impact(0, 1, true);
        assert!(revealed_overlays(&damage).is_empty());
        damage.impact(0, 3, true);
        assert_eq!(revealed_overlays(&damage), BTreeSet::from([slot::DAMAGE0_FRONT]));
        // The roof has no corner overlay even when it dents.
        damage.impact(8, 6, false);
        assert_eq!(revealed_overlays(&damage), BTreeSet::from([slot::DAMAGE0_FRONT]));
        damage.impact(6, 2, false);
        assert_eq!(revealed_overlays(&damage), BTreeSet::from([slot::DAMAGE0_FRONT, slot::DAMAGE0_REAR_LEFT]));
    }

    #[test]
    fn only_the_four_corner_zones_have_a_tire() {
        let corners: Vec<usize> = ZONE_WHEEL.iter().flatten().copied().collect();
        assert_eq!(corners, vec![0, 1, 2, 3]);
        assert!(ZONE_WHEEL[..4].iter().all(Option::is_none));
    }

    #[test]
    fn a_cracked_zone_swaps_its_windows_to_the_damaged_texture() {
        use crate::car::swaps::{TextureSwaps, WINDOW_DAMAGE};
        use blackbox_hash::bstring_hash;
        assert_eq!(zone_windows(0), &["WINDOW_FRONT"]);
        assert!(zone_windows(8).is_empty() && zone_windows(9).is_empty(), "roof and underbody have no glass");
        let swaps = TextureSwaps::for_car("BMWM3GTR").with_window_hashes(&[bstring_hash("WINDOW_REAR")]);
        assert_eq!(swaps.resolve(bstring_hash("WINDOW_REAR")), bstring_hash(WINDOW_DAMAGE));
        assert_eq!(swaps.resolve(bstring_hash("WINDOW_FRONT")), bstring_hash("WINDOW_FRONT"), "the others are intact");
    }
}
