//! The pit goal's two records. Spec: `docs/specs/ai-pursuit-goals.md` sections 2.3 and 4.
//!
//! A pit cop sits 4 m beside the target's rear quarter and drives to a point 10 m out on the opposite side, 2.7 m behind.
//! The spec's labels are `left` (A.x = +4) and `right` (A.x = -4). In the target's frame `x` is to the right, so the
//! label and the sign of A.x disagree: the signs are kept as the spec writes them [unconfirmed which label is the cop's
//! side]. Both records have priority 1.

use super::record::Record;
use glam::Vec2;

/// The pit's finisher run in seconds (goals spec section 4, item 5; formation spec: `FinisherTime` 2.0 s).
pub const RUN_TIME: f32 = 2.0;

/// The two pit records: the pursuit slots beside the rear quarter and the in-position points across it.
pub fn records() -> [Record; 2] {
    [
        Record { name: "left", pursuit: Vec2::new(4.0, -2.7), in_position: Vec2::new(-10.0, -2.7), priority: 1 },
        Record { name: "right", pursuit: Vec2::new(-4.0, -2.7), in_position: Vec2::new(10.0, -2.7), priority: 1 },
    ]
}

#[cfg(test)]
mod tests {
    use super::super::approach::slot_point;
    use super::*;

    #[test]
    fn the_pursuit_slots_sit_beside_the_rear_quarter_of_a_target_facing_plus_z() {
        // Target at the origin facing +Z (Vec2 y): the slot is in the target's frame, so it is the offset itself.
        let [left, right] = records();
        let forward = Vec2::Y;
        assert_eq!(slot_point(Vec2::ZERO, forward, left.pursuit), Vec2::new(4.0, -2.7));
        assert_eq!(slot_point(Vec2::ZERO, forward, right.pursuit), Vec2::new(-4.0, -2.7));
    }

    #[test]
    fn the_in_position_point_is_across_the_rear_on_the_opposite_side() {
        let [left, right] = records();
        assert_eq!(left.in_position, Vec2::new(-10.0, -2.7));
        assert_eq!(right.in_position, Vec2::new(10.0, -2.7));
        assert!(left.in_position.x * left.pursuit.x < 0.0, "opposite side to the cop's slot");
    }

    #[test]
    fn both_pit_records_share_priority_one() {
        assert!(records().iter().all(|r| r.priority == 1));
    }
}
