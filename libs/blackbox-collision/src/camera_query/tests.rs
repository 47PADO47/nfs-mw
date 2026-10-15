use super::*;
use crate::{Barrier, CollisionPack, RayOptions};

fn world() -> CollisionWorld {
    let instance = Instance {
        row_x: [1.0, 0.0, 0.0],
        half_width: 10.0,
        row_z: [0.0, 0.0, 1.0],
        half_length: 10.0,
        translation: [0.0; 3],
        radius: 10.0,
        half_height: 10.0,
        iter_stamp: 0,
        flags: 0,
        group: 0,
        article: 0,
    };
    let article = Article {
        strips: vec![],
        barriers: vec![Barrier { p0: [0.0, -1.0, -2.0], p1: [0.0, 1.0, 2.0], surface: 0, flags: 0, inv_xz_len: 0.25 }],
        surfaces: vec![123],
        intermediate_object: 0,
        flags: 0,
    };
    let mut world = CollisionWorld::default();
    world.insert(CollisionPack { section: 1, instances: vec![instance], articles: vec![article], objects: vec![] });
    world
}

#[test]
fn native_height_limits_and_sloping_rays_match_the_actual_barrier_producer() {
    let world = world();
    for (y, slope, expected) in [
        (-1.0001, 0.0, false),
        (-1.0, 0.0, false),
        (-0.9999, 0.0, true),
        (0.0, 0.0, true),
        (0.9999, 0.0, true),
        (1.0, 0.0, false),
        (1.0001, 0.0, false),
        (0.0, 2.0, true),
        (0.9, 0.5, true),
        (1.0, 0.5, false),
        (-1.0, 0.5, false),
    ] {
        let from = [-2.0, y - slope, 0.0];
        let to = [2.0, y + slope, 0.0];
        assert_eq!(world.camera_ray_cast(from, to, |_| false).is_some(), expected, "{y}/{slope}");
    }
}

#[test]
fn native_endpoint_numerator_tolerance_does_not_expand_normalized_fractions() {
    let world = world();
    for (z, expected) in
        [(-2.00001, true), (-2.0, true), (-1.99999, true), (1.99999, true), (2.0, true), (2.00001, false)]
    {
        let (a, b) = ([-2.0, 0.0, z], [2.0, 0.0, z]);
        for (from, to) in [(a, b), (b, a)] {
            assert_eq!(world.camera_ray_cast(from, to, |_| false).is_some(), expected, "{z}/{from:?}");
        }
    }
    assert!(world.ray_cast([-2.0, 0.0, 2.00001], [2.0, 0.0, 2.00001], &RayOptions::default()).is_some());
    let hit = world.camera_ray_cast([0.00001, 0.0, 0.0], [4.0, 0.0, 0.0], |_| false).unwrap();
    assert!(hit.t < 0.0, "native tolerates this crossing before the ray origin");
    assert!(hit.point[0].abs() < 1e-10, "the hit must not clamp to the ray origin");
}

#[test]
fn native_tilted_wall_uses_transformed_endpoints_and_authored_inverse_length() {
    let mut world = world();
    let mut pack = world.remove(1).unwrap();
    pack.instances[0].flags = crate::FLAG_Y_NOT_UP;
    pack.instances[0].row_z = [0.0, 0.3_f32.sin(), 0.3_f32.cos()];
    world.insert(pack);
    let (from, to) = ([-2.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
    let hit = world.camera_ray_cast(from, to, |_| false).unwrap();
    assert!((hit.normal[0] + 1.103_096_6).abs() < 2e-6);
    assert_eq!(hit.normal[1], 0.0);
    let old = world.ray_cast(from, to, &RayOptions::default()).unwrap();
    assert_eq!(old.normal, [-1.0, 0.0, 0.0]);
    let reverse = world.camera_ray_cast(to, from, |_| false).unwrap();
    assert!((reverse.normal[0] - 1.103_096_6).abs() < 2e-6);
    let mut pack = world.remove(1).unwrap();
    pack.instances[0].radius = 0.5;
    world.insert(pack);
    assert!(world.camera_ray_cast([-2.0, 0.0, 1.0], [2.0, 0.0, 1.0], |_| false).is_none());
    assert!(world.ray_cast([-2.0, 0.0, 1.0], [2.0, 0.0, 1.0], &RayOptions::default()).is_some());
}

#[test]
fn native_group_activation_is_independent_of_surface_exclusion() {
    let mut world = world();
    let mut pack = world.remove(1).unwrap();
    pack.instances[0].group = 7;
    world.insert(pack);
    let (from, to) = ([-2.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
    assert!(world.camera_ray_cast(from, to, |_| false).is_none());
    let hit = world.camera_ray_cast(from, to, |group| group == 7).unwrap();
    assert_eq!(hit.section, 1);
    assert_eq!(hit.instance, 0);
    assert_eq!(hit.surface_hash, 123);
    assert_eq!(hit.surface_flags & crate::GROUP_EXCLUSION, crate::GROUP_EXCLUSION);
}
