//! The cops' line of sight (`docs/specs/ai-pursuit-cop-cars.md` section 2): a ray from a cop's eyes to the player's
//! that the world's geometry blocks. It is the world query with faces and barriers and no exclusion mask
//! (`docs/formats/collision.md`, "Query semantics" 1, 4, 6 and 7). Road decks, bridge undersides and buildings block, so a
//! cop below a deck or around a corner does not see the player. A barrier blocks from either side: the query has no
//! facing test. A scenery group that is switched off is not in the world and blocks nothing. This is a separate query
//! from the body's wall probe (`walls.rs`), which ignores floors on purpose.
//!
//! The drive ray ([`drivable_cast`]) is the other world line test of a cop: whether it can drive straight to the target.
//! It tests barriers only, so road faces never block it (`docs/specs/ai-pursuit-formations.md` section 3).
//!
//! The helicopter's sight ([`heli_clear`]) tests faces only: a barrier, a wall the cars cannot cross, does not hide the
//! player from the air (`docs/specs/ai-helicopter.md` section 4).

use blackbox_collision::{CollisionWorld, HitKind, RayOptions};
use glam::Vec3;

use super::walls::{Hit, HitInfo};

/// Body positions farther apart than this (200 m) are not drivable to each other.
const DRIVABLE_RANGE: f32 = 200.0;
/// The drive ray runs this far above both body positions, metres.
const DRIVABLE_LIFT: f32 = 0.5;
/// A barrier hit within this squared distance of the ray's end (0.5 m^2, about 0.71 m) does not block it.
const DRIVABLE_END_SQ: f32 = 0.5;

/// The sight's query: faces and barriers, no exclusion bits, and only the scenery groups that are in the world.
fn sight_options() -> RayOptions {
    RayOptions { exclude: 0, skip_groups: true, ..RayOptions::default() }
}

/// Ray casts for the cops' sight: the first face or barrier on the segment.
pub fn sight_cast(collision: &CollisionWorld) -> impl Fn(Vec3, Vec3) -> Option<Hit> + '_ {
    let options = sight_options();
    move |from, to| {
        let hit = collision.ray_cast(from.to_array(), to.to_array(), &options)?;
        let info = HitInfo {
            barrier: hit.kind == HitKind::Barrier,
            surface: hit.surface_hash,
            section: hit.section,
            instance: hit.instance,
        };
        Some(Hit { point: Vec3::from(hit.point), normal: Vec3::from(hit.normal), prop: None, info: Some(info) })
    }
}

/// The helicopter sight's query: faces only, no exclusion bits, and only the scenery groups that are in the world.
fn heli_options() -> RayOptions {
    RayOptions { barriers: false, ..sight_options() }
}

/// Whether the helicopter's line from `from` to `to` (physics space) is clear: no face of the world on it. Barriers do
/// not block it.
pub fn heli_clear(collision: &CollisionWorld) -> impl Fn(Vec3, Vec3) -> bool + '_ {
    let options = heli_options();
    move |from, to| collision.ray_cast(from.to_array(), to.to_array(), &options).is_none()
}

/// The drive ray's query: barriers only, no exclusion bits, and only the scenery groups that are in the world.
fn drivable_options() -> RayOptions {
    RayOptions { faces: false, exclude: 0, skip_groups: true, ..RayOptions::default() }
}

/// Whether a car at body position `from` can drive straight to body position `to` (physics space): the two are at most
/// 200 m apart, and the barrier ray between them, 0.5 m up, meets no barrier short of its last 0.71 m.
pub fn drivable_cast(collision: &CollisionWorld) -> impl Fn(Vec3, Vec3) -> bool + '_ {
    let options = drivable_options();
    move |from, to| {
        let (from, to) = (from + Vec3::Y * DRIVABLE_LIFT, to + Vec3::Y * DRIVABLE_LIFT);
        let hit = collision.ray_cast(from.to_array(), to.to_array(), &options);
        drivable(from, to, hit.map(|hit| Vec3::from(hit.point)))
    }
}

/// The drive ray's verdict from its lifted ends and the point of its first barrier hit, if any.
fn drivable(from: Vec3, to: Vec3, hit: Option<Vec3>) -> bool {
    if from.distance_squared(to) > DRIVABLE_RANGE * DRIVABLE_RANGE {
        return false;
    }
    hit.is_none_or(|point| point.distance_squared(to) <= DRIVABLE_END_SQ)
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_collision::{Article, Barrier, CollisionPack, Instance};

    /// A ray in front of the wall (its normal side) and one behind it.
    const FRONT: Vec3 = Vec3::new(0.0, 1.0, 0.0);
    const BACK: Vec3 = Vec3::new(0.0, 1.0, 20.0);

    /// A one-sided wall across z = 10 (x from -10 to 10, 0 to 4 m high) whose normal faces -z, in scenery group `group`.
    fn wall(group: u16) -> CollisionWorld {
        let barrier = Barrier { p0: [-10.0, 0.0, 10.0], p1: [10.0, 4.0, 10.0], surface: 0, flags: 0, inv_xz_len: 0.05 };
        let mut collision = CollisionWorld::new(None);
        collision.insert(CollisionPack {
            section: 1,
            instances: vec![Instance {
                row_x: [1.0, 0.0, 0.0],
                row_z: [0.0, 0.0, 1.0],
                translation: [0.0; 3],
                half_width: 30.0,
                half_length: 30.0,
                half_height: 10.0,
                radius: 45.0,
                iter_stamp: 0,
                flags: 0,
                group,
                article: 0,
            }],
            articles: vec![Article {
                strips: vec![],
                barriers: vec![barrier],
                surfaces: vec![0],
                intermediate_object: 0,
                flags: 0,
            }],
            objects: vec![],
        });
        collision
    }

    #[test]
    fn the_sight_tests_faces_and_barriers_with_no_exclusion() {
        let options = sight_options();
        assert!(options.faces && options.barriers, "both primitive kinds");
        assert_eq!(options.exclude, 0, "no surface exclusion mask");
        assert!(options.skip_groups, "only the scenery groups that are in the world");
    }

    #[test]
    fn a_barrier_blocks_the_sight_from_either_side() {
        let collision = wall(0);
        let cast = sight_cast(&collision);
        assert!(cast(FRONT, BACK).is_some_and(|hit| (hit.point.z - 10.0).abs() < 1e-3), "from its front");
        assert!(cast(BACK, FRONT).is_some(), "from behind: the world query has no facing test");
    }

    #[test]
    fn the_helicopter_sees_over_a_barrier_and_tests_faces_only() {
        let options = heli_options();
        assert!(options.faces && !options.barriers);
        assert_eq!(options.exclude, 0);
        assert!(options.skip_groups);
        let collision = wall(0);
        assert!(heli_clear(&collision)(FRONT, BACK), "the wall that hides the player from the cars");
    }

    #[test]
    fn the_drive_ray_tests_barriers_only() {
        let options = drivable_options();
        assert!(!options.faces && options.barriers, "road faces never block a drive");
        assert_eq!(options.exclude, 0);
        assert!(options.skip_groups);
    }

    #[test]
    fn a_barrier_short_of_the_end_blocks_the_drive_ray() {
        let collision = wall(0);
        let drive = drivable_cast(&collision);
        assert!(!drive(FRONT, BACK), "the wall is in the way");
        assert!(drive(FRONT, Vec3::new(0.0, 1.0, 9.5)), "the wall is beyond the end");
        assert!(drive(FRONT, Vec3::new(0.0, 1.0, 10.5)), "a hit within 0.71 m of the end does not count");
    }

    #[test]
    fn a_target_more_than_200_m_away_is_not_drivable() {
        assert!(drivable(Vec3::ZERO, Vec3::new(0.0, 0.0, 199.0), None));
        assert!(!drivable(Vec3::ZERO, Vec3::new(0.0, 0.0, 201.0), None));
    }

    #[test]
    fn a_scenery_group_that_is_off_does_not_block_the_sight() {
        let mut collision = wall(217);
        assert!(sight_cast(&collision)(FRONT, BACK).is_none(), "a group that is off is not in the world");
        collision.set_group_enabled(217, true);
        assert!(sight_cast(&collision)(FRONT, BACK).is_some(), "a group that is on blocks like any wall");
    }
}
