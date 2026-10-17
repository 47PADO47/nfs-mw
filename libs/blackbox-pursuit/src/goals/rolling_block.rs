//! The rolling block goal's five records. Spec: `docs/specs/ai-pursuit-goals.md` sections 2.2 and 9.
//!
//! All five pursuit slots sit 14 m ahead of the target's centre (the goal spec's table). The formation spec's table lists
//! the inner right slot at `(2.5, 10)` instead. The goal spec's reading (14) is kept here; the owner of the formation
//! table should check it (goals spec section 10, item 1). [unconfirmed]

use super::record::{Record, front_depth};
use glam::Vec2;

/// The lateral scale of the in-position offsets: `s' = 1 - 0.8 t`.
pub fn lateral_scale(tightness: f32) -> f32 {
    1.0 - 0.8 * tightness
}

/// The five rolling block records for a tightness: a centre car and two inner and two outer cars, in a row.
pub fn records(tightness: f32) -> [Record; 5] {
    let depth = front_depth(tightness);
    let s = lateral_scale(tightness);
    [
        Record { name: "centre", pursuit: Vec2::new(0.0, 14.0), in_position: Vec2::new(0.0, depth), priority: 1 },
        Record {
            name: "inner left",
            pursuit: Vec2::new(-2.5, 14.0),
            in_position: Vec2::new(-2.5 * s, depth),
            priority: 2,
        },
        Record {
            name: "inner right",
            pursuit: Vec2::new(2.5, 14.0),
            in_position: Vec2::new(2.5 * s, depth),
            priority: 2,
        },
        Record {
            name: "outer left",
            pursuit: Vec2::new(-5.0, 14.0),
            in_position: Vec2::new(-5.0 * s, depth),
            priority: 3,
        },
        Record {
            name: "outer right",
            pursuit: Vec2::new(5.0, 14.0),
            in_position: Vec2::new(5.0 * s, depth),
            priority: 3,
        },
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
    fn the_row_is_14_metres_ahead_of_a_target_facing_plus_z() {
        let recs = records(0.5);
        for r in &recs {
            let world = slot_point(Vec2::ZERO, Vec2::Y, r.pursuit);
            assert!(close(world.y, 14.0), "{} at {world:?}", r.name);
        }
        let world_x: Vec<f32> = recs.iter().map(|r| slot_point(Vec2::ZERO, Vec2::Y, r.pursuit).x).collect();
        assert_eq!(world_x, vec![0.0, -2.5, 2.5, -5.0, 5.0]);
    }

    #[test]
    fn tightness_half_gives_the_loose_offsets() {
        // t = 0.5: s' = 0.6, foff = -0.5.
        let recs = records(0.5);
        assert!(close(recs[0].in_position.y, -0.5));
        assert!(close(recs[1].in_position.x, -1.5), "inner left {}", recs[1].in_position.x);
        assert!(close(recs[3].in_position.x, -3.0), "outer left {}", recs[3].in_position.x);
    }

    #[test]
    fn full_tightness_gives_the_tight_offsets() {
        // t = 1: s' = 0.2, foff = -3. Inner at +-0.5, outer at +-1.0, all 3 m behind the centre.
        let recs = records(1.0);
        assert!(close(recs[1].in_position.x, -0.5));
        assert!(close(recs[2].in_position.x, 0.5));
        assert!(close(recs[4].in_position.x, 1.0));
        assert!(recs.iter().all(|r| close(r.in_position.y, -3.0)));
    }

    #[test]
    fn the_priorities_are_one_two_three() {
        let p: Vec<u32> = records(0.5).iter().map(|r| r.priority).collect();
        assert_eq!(p, vec![1, 2, 2, 3, 3]);
    }

    #[test]
    fn the_lateral_scale_shrinks_with_tightness() {
        assert!(close(lateral_scale(0.5), 0.6));
        assert!(lateral_scale(1.0) < lateral_scale(0.5));
    }
}
