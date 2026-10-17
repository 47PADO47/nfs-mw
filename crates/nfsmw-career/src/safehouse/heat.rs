//! Heat between worlds (`docs/specs/career-safehouse-2.md` section 8, `docs/specs/career-economy-flows.md`
//! section 4).
//!
//! Heat lives in two places: the live heat of the driven car, which exists only while a world does, and the heat
//! of the career car record, which is saved. They meet at two moments:
//!
//! - **world unload** ([`unload_world`]): the live heat is written to the record of the current car as
//!   `min(heat, 5.0)`;
//! - **car spawn** ([`spawn_heat`]): the new car starts with `max(1.0, record heat)`.
//!
//! The evade, race-win and milestone factors ([`apply_heat_factor`]) act on the **records** only. They never touch
//! the live heat.
//!
//! # The overwrite quirk
//!
//! Because the unload writes the live heat over the record of the car that is being driven, a factor applied to
//! that record before the unload is lost: it survives only on the other cars of the stable. Entering a safe house
//! after an evade therefore does not cool the driven car (the cool-down entry multiplies every record by 0.95, and
//! then the unload overwrites the driven car's record with the unchanged live heat). This is how the original
//! reads (`career-safehouse-2.md` 8.4, **[exe]**, not tested in game) and it is implemented as it reads; see the
//! test `the_driven_car_does_not_keep_the_evade_factor`.

use crate::settings::CareerSettings;
use crate::stable::Stable;

/// The most heat a record keeps when the world is torn down.
pub const UNLOAD_HEAT_CAP: f32 = 5.0;
/// The least heat a new car starts with.
pub const SPAWN_HEAT_FLOOR: f32 = 1.0;
/// The `pursuitlevels/default` evade factor given by the spec (`career-economy-flows.md` 4,
/// `career-safehouse.md` 3.4). The values come from the install data; callers should pass those.
pub const DEFAULT_EVADE_HEAT_FACTOR: f32 = 0.95;
/// The race-win factor (`pursuitlevels/default`, 0.94 in `ai-pursuit-heat.md` 3.3).
pub const DEFAULT_RACE_WIN_HEAT_FACTOR: f32 = 0.94;
/// The milestone factor (0.93). The pairing of this value with the milestone moment is **[unconfirmed]** in the
/// spec and no caller was found.
pub const DEFAULT_MILESTONE_HEAT_FACTOR: f32 = 0.93;

/// Multiplies the heat of every live car record of the stable by `factor` (pursuit evaded, career race won,
/// milestone completed). The live heat of a running world is not part of this.
pub fn apply_heat_factor(stable: &mut Stable, factor: f32) {
    for index in stable.live_career_indices() {
        stable.careers[index].heat *= factor;
    }
}

/// The player car object is destroyed while the world is torn down: in a career context, a car that has a
/// record writes `min(live heat, 5.0)` into it. Returns false (nothing written) outside the career context or when
/// the current car has no record. A heat that is not a number is written as the cap.
pub fn unload_world(settings: &CareerSettings, stable: &mut Stable, live_heat: f32, career_context: bool) -> bool {
    if !career_context {
        return false;
    }
    let Some(record) = stable.career_for_handle_mut(settings.car_handle) else {
        return false;
    };
    record.heat = live_heat.min(UNLOAD_HEAT_CAP);
    true
}

/// The heat the new world gives the player car: `max(1.0, record heat)` in a career context, 1.0 (the default)
/// otherwise or when the car has no record.
pub fn spawn_heat(settings: &CareerSettings, stable: &Stable, career_context: bool) -> f32 {
    if !career_context {
        return SPAWN_HEAT_FLOOR;
    }
    let Some(record) = stable.career_for_handle(settings.car_handle) else {
        return SPAWN_HEAT_FLOOR;
    };
    record.heat.max(SPAWN_HEAT_FLOOR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stable::{CarRecord, CareerRecord};

    /// A stable with the current car at record 0 and a second car at record 1, both with the given heat.
    fn two_cars(settings: &CareerSettings, heat_a: f32, heat_b: f32) -> Stable {
        let mut stable = Stable::empty();
        for (slot, handle, heat) in [(0usize, settings.car_handle, heat_a), (1, 99, heat_b)] {
            stable.cars[slot] = CarRecord { handle, flags: 0xF_0002, career_index: slot as u8, ..CarRecord::free() };
            stable.careers[slot] = CareerRecord { heat, ..CareerRecord::new(slot as u8) };
        }
        stable
    }

    #[test]
    fn the_unload_caps_the_live_heat_at_five() {
        let settings = CareerSettings::new_career();
        let mut stable = two_cars(&settings, 1.0, 1.0);
        assert!(unload_world(&settings, &mut stable, 7.5, true));
        assert_eq!(stable.careers[0].heat, 5.0);
        assert!(unload_world(&settings, &mut stable, 3.25, true));
        assert_eq!(stable.careers[0].heat, 3.25);
        assert!(unload_world(&settings, &mut stable, f32::NAN, true));
        assert_eq!(stable.careers[0].heat, 5.0);
        assert_eq!(stable.careers[1].heat, 1.0, "other cars are untouched");
    }

    #[test]
    fn the_unload_writes_nothing_outside_a_career_or_without_a_record() {
        let mut settings = CareerSettings::new_career();
        let mut stable = two_cars(&settings, 2.0, 2.0);
        assert!(!unload_world(&settings, &mut stable, 4.0, false));
        assert_eq!(stable.careers[0].heat, 2.0);
        settings.car_handle = 12345;
        assert!(!unload_world(&settings, &mut stable, 4.0, true));
    }

    #[test]
    fn the_spawn_never_starts_below_one() {
        let settings = CareerSettings::new_career();
        let stable = two_cars(&settings, 0.0, 3.0);
        assert_eq!(spawn_heat(&settings, &stable, true), 1.0);
        let hot = two_cars(&settings, 4.5, 0.0);
        assert_eq!(spawn_heat(&settings, &hot, true), 4.5);
        assert_eq!(spawn_heat(&settings, &hot, false), 1.0, "the default outside a career");
        let nan = two_cars(&settings, f32::NAN, 0.0);
        assert_eq!(spawn_heat(&settings, &nan, true), 1.0);
    }

    #[test]
    fn a_car_with_no_record_spawns_at_the_floor() {
        let mut settings = CareerSettings::new_career();
        let stable = two_cars(&settings, 4.0, 4.0);
        settings.car_handle = 4242;
        assert_eq!(spawn_heat(&settings, &stable, true), 1.0);
    }

    #[test]
    fn the_factor_multiplies_every_live_record_and_no_other() {
        let settings = CareerSettings::new_career();
        let mut stable = two_cars(&settings, 4.0, 2.0);
        stable.careers[2] = CareerRecord { heat: 9.0, ..CareerRecord::new(2) };
        apply_heat_factor(&mut stable, 0.5);
        assert_eq!((stable.careers[0].heat, stable.careers[1].heat), (2.0, 1.0));
        assert_eq!(stable.careers[2].heat, 9.0, "a record with no car is not live");
    }

    #[test]
    fn the_documented_factors_are_the_pursuitlevels_defaults() {
        assert_eq!(DEFAULT_EVADE_HEAT_FACTOR, 0.95);
        assert_eq!(DEFAULT_RACE_WIN_HEAT_FACTOR, 0.94);
        assert_eq!(DEFAULT_MILESTONE_HEAT_FACTOR, 0.93);
    }

    /// The round trip of `career-safehouse-2.md` 8.4: drive with live heat h, evade into the safe house, unload,
    /// resume.
    #[test]
    fn the_driven_car_does_not_keep_the_evade_factor() {
        let settings = CareerSettings::new_career();
        let live_heat = 3.0;
        // Record 0 is the driven car; its record held 3.0 when the drive started (spawn gave 3.0 to the car).
        let mut stable = two_cars(&settings, 3.0, 4.0);
        assert_eq!(spawn_heat(&settings, &stable, true), live_heat);

        // Cool-down entry: the evade handling multiplies every record by 0.95. The live heat is not changed.
        apply_heat_factor(&mut stable, DEFAULT_EVADE_HEAT_FACTOR);
        assert_eq!(stable.careers[0].heat, 3.0 * 0.95);
        assert_eq!(stable.careers[1].heat, 4.0 * 0.95);

        // Unload: the record of the driven car is overwritten by the live value, so its factor is gone.
        unload_world(&settings, &mut stable, live_heat, true);
        assert_eq!(stable.careers[0].heat, 3.0, "the quirk: 0.95 does not survive on the driven car");
        assert_eq!(stable.careers[1].heat, 4.0 * 0.95, "the factor survives on the other cars");

        // Resume: the new car starts at max(1.0, record).
        assert_eq!(spawn_heat(&settings, &stable, true), 3.0);
    }

    #[test]
    fn heat_above_five_is_cut_on_the_round_trip() {
        let settings = CareerSettings::new_career();
        let mut stable = two_cars(&settings, 1.0, 1.0);
        unload_world(&settings, &mut stable, 8.0, true);
        assert_eq!(spawn_heat(&settings, &stable, true), 5.0);
    }

    #[test]
    fn entering_without_a_pursuit_differs_only_by_the_factor_on_the_other_cars() {
        let settings = CareerSettings::new_career();
        let mut plain = two_cars(&settings, 2.0, 4.0);
        unload_world(&settings, &mut plain, 2.0, true);
        let mut cooled = two_cars(&settings, 2.0, 4.0);
        apply_heat_factor(&mut cooled, DEFAULT_EVADE_HEAT_FACTOR);
        unload_world(&settings, &mut cooled, 2.0, true);
        assert_eq!(plain.careers[0].heat, cooled.careers[0].heat);
        assert_ne!(plain.careers[1].heat, cooled.careers[1].heat);
    }
}
