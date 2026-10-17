//! The box-in goal's four records. Spec: `docs/specs/ai-pursuit-goals.md` sections 2.1 and 9.
//!
//! The tightness `t` (`BoxinTightness`) sets the wing scale `s = 0.7 - 0.5 t` and the front depth `foff = 2 - 5 t`. The
//! pursuit slots do not depend on `t`; only the in-position offsets do.

use super::record::{Record, front_depth};
use glam::Vec2;

/// The wing and rear scale of the in-position offsets: `s = 0.7 - 0.5 t`.
pub fn wing_scale(tightness: f32) -> f32 {
    0.7 - 0.5 * tightness
}

/// The four box-in records for a tightness: the front car, the two wings and the rear car.
pub fn records(tightness: f32) -> [Record; 4] {
    let depth = front_depth(tightness);
    let s = wing_scale(tightness);
    [
        Record { name: "front", pursuit: Vec2::new(0.0, 14.0), in_position: Vec2::new(0.0, depth), priority: 1 },
        Record { name: "left wing", pursuit: Vec2::new(-3.5, 0.0), in_position: Vec2::new(-3.5 * s, 0.0), priority: 2 },
        Record { name: "right wing", pursuit: Vec2::new(3.5, 0.0), in_position: Vec2::new(3.5 * s, 0.0), priority: 2 },
        Record { name: "rear", pursuit: Vec2::new(0.0, -7.5), in_position: Vec2::new(0.0, -7.5 * s), priority: 4 },
    ]
}

#[cfg(test)]
mod tests {
    use super::super::approach::slot_point;
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn the_pursuit_slots_of_a_target_facing_plus_z() {
        let recs = records(0.5);
        let world: Vec<Vec2> = recs.iter().map(|r| slot_point(Vec2::ZERO, Vec2::Y, r.pursuit)).collect();
        assert_eq!(world[0], Vec2::new(0.0, 14.0), "front, 14 m ahead");
        assert_eq!(world[1], Vec2::new(-3.5, 0.0), "left wing");
        assert_eq!(world[2], Vec2::new(3.5, 0.0), "right wing");
        assert_eq!(world[3], Vec2::new(0.0, -7.5), "rear, 7.5 m behind");
    }

    #[test]
    fn heat_one_tightness_half_gives_the_loose_in_position_offsets() {
        // t = 0.5: s = 0.45, foff = -0.5.
        let recs = records(0.5);
        assert!(close(recs[0].in_position.y, -0.5), "front depth {}", recs[0].in_position.y);
        assert!(close(recs[1].in_position.x, -1.575), "left wing {}", recs[1].in_position.x);
        assert!(close(recs[2].in_position.x, 1.575), "right wing {}", recs[2].in_position.x);
        assert!(close(recs[3].in_position.y, -3.375), "rear {}", recs[3].in_position.y);
    }

    #[test]
    fn full_tightness_closes_the_box_in() {
        // t = 1: s = 0.2, foff = -3. The front sits 3 m behind the centre, wings 0.7 m out, rear 1.5 m behind.
        let recs = records(1.0);
        assert!(close(recs[0].in_position.y, -3.0));
        assert!(close(recs[1].in_position.x, -0.7));
        assert!(close(recs[3].in_position.y, -1.5));
    }

    #[test]
    fn tighter_boxes_have_smaller_wing_scales() {
        assert!(wing_scale(1.0) < wing_scale(0.5));
        assert!(close(wing_scale(0.5), 0.45));
    }

    #[test]
    fn priorities_follow_the_spec() {
        let p: Vec<u32> = records(0.5).iter().map(|r| r.priority).collect();
        assert_eq!(p, vec![1, 2, 2, 4]);
    }
}
