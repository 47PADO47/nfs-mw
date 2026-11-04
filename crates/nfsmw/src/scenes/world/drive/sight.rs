//! The cops' line of sight (`docs/specs/ai-pursuit-cop-cars.md` section 2): a ray from a cop's eyes to the player's
//! that the world's geometry blocks. It is the world query with faces and barriers and no exclusion mask
//! (`docs/formats/collision.md`, "Query semantics" 1, 4, 6 and 7). Road decks, bridge undersides and buildings block, so a
//! cop below a deck or around a corner does not see the player. A barrier blocks from either side: the query has no
//! facing test. A scenery group that is switched off is not in the world and blocks nothing. This is a separate query
//! from the body's wall probe (`walls.rs`), which ignores floors on purpose.

use blackbox_collision::{CollisionWorld, HitKind, RayOptions};
use glam::Vec3;

use super::walls::{Hit, HitInfo};

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
    fn a_scenery_group_that_is_off_does_not_block_the_sight() {
        let mut collision = wall(217);
        assert!(sight_cast(&collision)(FRONT, BACK).is_none(), "a group that is off is not in the world");
        collision.set_group_enabled(217, true);
        assert!(sight_cast(&collision)(FRONT, BACK).is_some(), "a group that is on blocks like any wall");
    }
}
