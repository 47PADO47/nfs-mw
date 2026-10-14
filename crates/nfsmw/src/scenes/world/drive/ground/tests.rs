use super::*;
use blackbox_collision::{Article, CollisionPack, Instance, PackedVert, STRIP_UP_FACING, Strip};

fn world(group: u16) -> CollisionWorld {
    let strip = Strip {
        center: [0.0; 3],
        radius: 6.0,
        flags: STRIP_UP_FACING,
        verts: [(-128, -128), (-128, 128), (128, -128), (128, 128)]
            .map(|(x, z)| PackedVert { x, y: 0, z, surface: 0, flags: 0 })
            .to_vec(),
    };
    let mut pack = CollisionPack {
        section: 1,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            translation: [0.0; 3],
            half_width: 4.0,
            half_length: 4.0,
            half_height: 1.0,
            radius: 6.0,
            iter_stamp: 0,
            flags: 0,
            group,
            article: 0,
        }],
        articles: vec![Article {
            strips: vec![strip],
            barriers: vec![],
            surfaces: vec![0],
            intermediate_object: 0,
            flags: 0,
        }],
        objects: vec![],
    };
    pack.apply_group_exclusion();
    let mut world = CollisionWorld::new(None);
    world.insert(pack);
    world
}

#[test]
fn free_roam_ground_excludes_grouped_floor_strips() {
    let surfaces = SurfaceTable::default();
    let active = world(0);
    let grouped = world(9);
    // The exclusion bits do not cover grouped instances that contain strips.
    assert!(grouped.ground(0.0, 0.0, 2.0, -2.0).is_some());
    let origin = Vec3::Y * 2.0;
    assert!(WorldGround { collision: &active, surfaces: &surfaces }.hit(origin, Vec3::NEG_Y, 4.0).is_some());
    assert!(WorldGround { collision: &grouped, surfaces: &surfaces }.hit(origin, Vec3::NEG_Y, 4.0).is_none());
}
