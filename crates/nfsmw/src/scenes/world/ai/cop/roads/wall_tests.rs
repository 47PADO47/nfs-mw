//! Authored barriers, not road faces, supply the close-action wall force.

use super::*;
use blackbox_collision::{Article, Barrier, CollisionPack, HitKind, Instance, PackedVert, STRIP_FACING_UNKNOWN, Strip};

fn body() -> Body {
    Body { position: Vec3::Y, velocity: Vec3::Z * 20.0, forward: Vec2::Y, half_width: 1.0, half_length: 2.0 }
}

fn slope() -> Strip {
    let points = [(-1280, 0, 0), (-1280, 640, 2560), (1280, 0, 0), (1280, 640, 2560)];
    Strip {
        center: [0.0; 3],
        radius: 30.0,
        flags: STRIP_FACING_UNKNOWN,
        verts: points.into_iter().map(|(x, y, z)| PackedVert { x, y, z, surface: 0, flags: 0 }).collect(),
    }
}

fn barrier(bottom: f32, top: f32) -> Barrier {
    Barrier { p0: [-10.0, bottom, 10.0], p1: [10.0, top, 10.0], surface: 0, flags: 0, inv_xz_len: 0.05 }
}

fn world(strips: Vec<Strip>, barriers: Vec<Barrier>) -> CollisionWorld {
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
            group: 0,
            article: 0,
        }],
        articles: vec![Article { strips, barriers, surfaces: vec![0], intermediate_object: 0, flags: 0 }],
        objects: vec![],
    });
    collision
}

#[test]
fn close_wall_query_ignores_a_sloped_road_face() {
    let collision = world(vec![slope()], vec![]);
    let me = body();
    let hit = collision
        .ray_cast(me.position.to_array(), (me.position + Vec3::Z * 30.0).to_array(), &RayOptions::default())
        .expect("fixture road face must intersect the velocity ray");
    assert_eq!(hit.kind, HitKind::Face);
    assert!(hit.normal[2] < -0.1, "the sloped face could supply a false horizontal wall force");
    assert_eq!(wall(&collision, &me), Vec2::ZERO);
}

#[test]
fn close_wall_query_finds_a_barrier_beyond_a_nearer_road_face() {
    let collision = world(vec![slope()], vec![barrier(0.0, 4.0)]);
    let me = body();
    let hit = collision
        .ray_cast(me.position.to_array(), (me.position + Vec3::Z * 30.0).to_array(), &RayOptions::default())
        .expect("fixture road face must precede the barrier");
    assert_eq!(hit.kind, HitKind::Face);
    assert!(hit.point[2] < 10.0);
    let expected = close::wall(me.velocity, -Vec3::Z, 10.0);
    assert!(expected.y < -10.0);
    assert!((wall(&collision, &me) - expected).length() < 0.0001);
}

#[test]
fn close_wall_query_starts_at_the_body_height_without_an_offset() {
    let collision = world(vec![], vec![barrier(0.75, 1.25)]);
    let me = body();
    let raised = me.position + Vec3::Y * 0.5;
    assert!(
        collision.ray_cast(raised.to_array(), (raised + Vec3::Z * 30.0).to_array(), &RayOptions::default()).is_none(),
        "fixture low barrier must miss the raised ray"
    );
    let expected = close::wall(me.velocity, -Vec3::Z, 10.0);
    assert!(expected.y < -10.0);
    assert!((wall(&collision, &me) - expected).length() < 0.0001);
}
