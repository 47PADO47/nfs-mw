use blackbox_collision::{Article, Barrier, CollisionPack, Instance, PackedVert, Strip};

use super::*;

fn world(faces: bool, barriers: Vec<Barrier>) -> CollisionWorld {
    let strips = match faces {
        true => vec![Strip {
            center: [0.0; 3],
            radius: 8.0,
            flags: 0,
            verts: [(-4, -4), (-4, 4), (4, -4), (4, 4)]
                .map(|(x, z)| PackedVert { x: x * 128, y: 0, z: z * 128, surface: 0, flags: 0 })
                .to_vec(),
        }],
        false => vec![],
    };
    let pack = CollisionPack {
        section: 7,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            half_width: 4.0,
            half_height: 4.0,
            half_length: 4.0,
            translation: [0.0; 3],
            radius: 8.0,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        }],
        articles: vec![Article { strips, barriers, surfaces: vec![7], intermediate_object: 0, flags: 0 }],
        objects: vec![],
    };
    let mut collision = CollisionWorld::new(None);
    collision.insert(pack);
    collision
}

fn barrier(x: f32, front: bool, two_sided: bool) -> Barrier {
    let ends = match front {
        true => [4.0, -4.0],
        false => [-4.0, 4.0],
    };
    Barrier {
        p0: [x, -2.0, ends[0]],
        p1: [x, 2.0, ends[1]],
        surface: 0,
        flags: 2 | u8::from(two_sided) * BARRIER_TWO_SIDED,
        inv_xz_len: 1.0 / 8.0,
    }
}

fn no_props(_: Vec3, _: Mat3, _: Vec3) -> Vec<WallContact> {
    vec![]
}

#[test]
fn a_road_seen_from_below_is_not_a_wall_for_a_tilted_body() {
    let collision = world(true, vec![]);
    let position = Vec3::new(0.0, -0.2, 0.0);
    let rotation = Mat3::from_rotation_z(0.5);
    let half = Vec3::new(0.9, 0.6, 2.2);
    let raw = collision
        .ray_cast(
            position.to_array(),
            (position + rotation * Vec3::new(half.x, 0.0, 0.0)).to_array(),
            &RayOptions::default(),
        )
        .unwrap();
    assert_eq!(raw.normal, [0.0, -1.0, 0.0], "the raw normal correctly faces the origin below the road");
    assert!(find(&world_cast(&collision), &no_props, position, rotation, half).is_empty());
    // The generic cast contract also handles callers with their own origin-facing floor normals.
    let cast =
        |from: Vec3, to: Vec3| Some(Hit { point: from.lerp(to, 0.5), normal: Vec3::NEG_Y, prop: None, info: None });
    assert!(find(&cast, &no_props, position, rotation, half).is_empty());
}

#[test]
fn vehicle_query_sees_the_front_wall_after_a_rear_one_way_barrier() {
    let collision = world(false, vec![barrier(1.0, false, false), barrier(2.0, true, false)]);
    let hit = world_cast(&collision)(Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0)).unwrap();
    assert_eq!(hit.point, Vec3::new(2.0, 0.0, 0.0));
    assert_eq!(hit.normal, Vec3::NEG_X);
    assert!(hit.info.unwrap().barrier);
    let reverse = world_cast(&collision)(Vec3::new(3.0, 0.0, 0.0), Vec3::ZERO).unwrap();
    assert_eq!(reverse.point, Vec3::new(1.0, 0.0, 0.0));
}

#[test]
fn vehicle_query_keeps_two_sided_walls_but_looks_past_floors() {
    let collision = world(true, vec![barrier(2.0, false, true)]);
    let hit = world_cast(&collision)(Vec3::new(0.0, -0.5, 0.0), Vec3::new(3.0, 1.0, 0.0)).unwrap();
    assert_eq!(hit.point, Vec3::new(2.0, 0.5, 0.0));
    assert_eq!(hit.normal, Vec3::NEG_X);
}

#[test]
fn vertical_prop_contacts_are_kept() {
    let props = |_: Vec3, _: Mat3, _: Vec3| {
        vec![WallContact { point: Vec3::ZERO, normal: Vec3::Y, depth: 0.1, prop: Some((1, None)), info: None }]
    };
    let no_cast = |_: Vec3, _: Vec3| None;
    let contacts = find(&no_cast, &props, Vec3::ZERO, Mat3::IDENTITY, Vec3::ONE);
    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0].prop, Some((1, None)));
}
