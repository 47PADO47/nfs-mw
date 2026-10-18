//! Cop sight as pure rules: how far a cop car sees the player by heat, the rear cone, and the test period. The
//! world ray that decides line of sight is the caller's; this module decides the range and cone.
//! Spec: `docs/specs/ai-pursuit-cop-cars.md` section 2 and `docs/specs/ai-pursuit.md` section 4.1 ([decomp] sources,
//! distances [verified] in the install's pursuit levels).

/// Seconds between two sight tests of a cop (`VisibiltyTestTimer`).
pub const TEST_PERIOD: f32 = 0.25;

/// A target this far behind a cop (its direction against the cop's forward, dot at or below this) is out of sight at
/// the rear range.
pub const REAR_CONE_DOT: f32 = -0.3;

/// (front, rear) sight distances (m) per heat level 1 to 10: 151/151 at heat 1 to 7, 201/101 at heat 8 to 10.
const RANGES: [(f32, f32); 10] = [
    (151.0, 151.0),
    (151.0, 151.0),
    (151.0, 151.0),
    (151.0, 151.0),
    (151.0, 151.0),
    (151.0, 151.0),
    (151.0, 151.0),
    (201.0, 101.0),
    (201.0, 101.0),
    (201.0, 101.0),
];

/// The (front, rear) sight distances of a heat level (clamped to 1..=10).
pub fn ranges(level: u32) -> (f32, f32) {
    RANGES[(level as usize).clamp(1, RANGES.len()) - 1]
}

/// Whether a cop can not see a target `distance` metres away, where `facing_dot` is the dot product of the cop's
/// forward with the unit direction from the cop to the target. The rear cone only matters when the rear range is
/// shorter than the front range.
pub fn out_of_range(level: u32, distance: f32, facing_dot: f32) -> bool {
    let (front, rear) = ranges(level);
    distance >= front || (distance >= rear && facing_dot <= REAR_CONE_DOT)
}

/// An unassigned patrol uses the constructor's default sight distances, rather than pursuit heat tuning.
pub fn patrol_out_of_range(distance: f32, facing_dot: f32) -> bool {
    distance >= 150.0 || (distance >= 50.0 && facing_dot <= REAR_CONE_DOT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heat_one_to_seven_sees_151_metres_all_round() {
        assert_eq!(ranges(1), (151.0, 151.0));
        assert_eq!(ranges(7), (151.0, 151.0));
        assert!(!out_of_range(3, 120.0, -1.0), "behind a cop at 120 m is still in sight");
    }

    #[test]
    fn heat_eight_to_ten_sees_201_ahead_and_101_behind() {
        assert_eq!(ranges(9), (201.0, 101.0));
        assert!(!out_of_range(9, 150.0, 1.0), "ahead at 150 m is in sight");
        assert!(out_of_range(9, 210.0, 1.0), "ahead beyond 201 m is out");
    }

    #[test]
    fn the_rear_cone_cuts_sight_at_the_rear_range() {
        assert!(out_of_range(9, 120.0, -0.5), "behind the cop past 101 m is out");
        assert!(!out_of_range(9, 120.0, 0.0), "to the side at 120 m is in sight");
        assert!(!out_of_range(9, 120.0, -0.2), "the cone starts at dot -0.3");
    }

    #[test]
    fn levels_clamp_to_one_to_ten() {
        assert_eq!(ranges(0), (151.0, 151.0));
        assert_eq!(ranges(99), (201.0, 101.0));
    }

    #[test]
    fn patrols_have_shorter_rear_sight_than_assigned_heat_one_units() {
        assert!(!patrol_out_of_range(149.0, 1.0));
        assert!(patrol_out_of_range(150.0, 1.0));
        assert!(!patrol_out_of_range(49.0, -1.0));
        assert!(patrol_out_of_range(50.0, -1.0));
        assert!(!out_of_range(1, 100.0, -1.0));
    }
}
