use super::synth::*;
use crate::{BARRIER_TWO_SIDED, CollisionPack, CollisionWorld, HitKind, RayOptions};

const ASPHALT: u32 = 0x19DB_2F1E;
const GRASS: u32 = 0x772F_B736;

/// A flat 8 x 8 m ground tile at y = 5 around (100, 5, 200), plus a wall along x = 104.
fn world() -> CollisionWorld {
    let ground = article(&[ground_strip(1)], &[], &[ASPHALT, GRASS]);
    let wall = article(
        &[],
        &[BarrierSpec { p0: [4.0, -1.0, -4.0], p1: [4.0, 3.0, 4.0], surface: 0, flags: 0x02 }],
        &[ASPHALT],
    );
    let instances = [
        InstanceSpec::upright([100.0, 5.0, 200.0], [4.0, 1.0, 4.0], 0),
        InstanceSpec::upright([100.0, 6.0, 200.0], [4.0, 3.0, 4.0], 1),
    ];
    let mut w = CollisionWorld::new(None);
    w.insert(CollisionPack::parse(&pack(7, &[ground, wall], &instances)).unwrap());
    w
}

#[test]
fn ground_ray_hits_the_tile() {
    let w = world();
    let hit = w.ground(101.0, 201.0, 20.0, -20.0).expect("hit");
    assert_eq!(hit.kind, HitKind::Face);
    assert!((hit.point[1] - 5.0).abs() < 1e-4, "{:?}", hit.point);
    assert!((hit.t - 15.0 / 40.0).abs() < 1e-5);
    assert_eq!(hit.normal, [0.0, 1.0, 0.0]);
    // Triangles take the surface of their third vertex: this strip's are surface 1 = grass.
    assert_eq!((hit.surface_index, hit.surface_hash), (1, GRASS));
    assert_eq!((hit.section, hit.instance), (7, 0));
    // Off the tile, and a segment that stops short of it.
    assert!(w.ground(120.0, 201.0, 20.0, -20.0).is_none());
    assert!(w.ground(101.0, 201.0, 20.0, 8.0).is_none());
}

#[test]
fn normal_faces_the_start_of_the_segment() {
    let w = world();
    let up = w.ray_cast([101.0, 20.0, 201.0], [101.0, 0.0, 201.0], &RayOptions::default()).unwrap();
    let down = w.ray_cast([101.0, -9.0, 201.0], [101.0, 9.0, 201.0], &RayOptions::default()).unwrap();
    assert_eq!(up.normal, [0.0, 1.0, 0.0]);
    assert_eq!(down.normal, [0.0, -1.0, 0.0]);
}

#[test]
fn barrier_blocks_horizontally_within_its_height() {
    let w = world();
    let opts = RayOptions::default();
    let hit = w.ray_cast([90.0, 6.0, 200.0], [110.0, 6.0, 200.0], &opts).expect("wall");
    assert_eq!(hit.kind, HitKind::Barrier);
    assert!((hit.point[0] - 104.0).abs() < 1e-4);
    assert_eq!(hit.normal, [-1.0, 0.0, 0.0]);
    assert_eq!((hit.surface_hash, hit.surface_flags), (ASPHALT, 0x02));
    // Above the wall (it spans y 5..9 in the world).
    assert!(w.ray_cast([90.0, 9.5, 200.0], [110.0, 9.5, 200.0], &opts).is_none());
    // Coming from the other side flips the normal.
    let back = w.ray_cast([110.0, 6.0, 200.0], [90.0, 6.0, 200.0], &opts).unwrap();
    assert_eq!(back.normal, [1.0, 0.0, 0.0]);
    // Only one of the two sides is the barrier's front.
    assert_ne!(hit.front_facing, back.front_facing);
    // Barriers can be switched off.
    let faces_only = RayOptions { barriers: false, ..opts };
    assert!(w.ray_cast([90.0, 6.0, 200.0], [110.0, 6.0, 200.0], &faces_only).is_none());
}

#[test]
fn nearest_of_face_and_barrier_wins() {
    let w = world();
    // Slants down through the wall at x = 104 first, then would reach the ground.
    let hit = w.ray_cast([95.0, 8.0, 200.0], [115.0, 4.0, 200.0], &RayOptions::default()).unwrap();
    assert_eq!(hit.kind, HitKind::Barrier);
    // The other way round it reaches the ground (x ~ 102) before the wall.
    let hit = w.ray_cast([101.0, 8.0, 200.0], [110.0, -1.0, 200.0], &RayOptions::default()).unwrap();
    assert_eq!(hit.kind, HitKind::Face);
}

#[test]
fn exclusion_mask_skips_flagged_surfaces() {
    let w = world();
    let mask = |exclude| RayOptions { exclude, ..RayOptions::default() };
    // The wall carries flag 0x02.
    assert!(w.ray_cast([90.0, 6.0, 200.0], [110.0, 6.0, 200.0], &mask(0x02)).is_none());
    assert!(w.ray_cast([90.0, 6.0, 200.0], [110.0, 6.0, 200.0], &mask(0x04)).is_some());
    assert_eq!(BARRIER_TWO_SIDED, 0x10);
}

#[test]
fn tiny_and_empty_segments() {
    let w = world();
    assert!(w.ray_cast([101.0, 5.5, 201.0], [101.0, 5.5, 201.0], &RayOptions::default()).is_none());
    // Shorter than 1 cm: lengthened so it can still touch what is right below it.
    let hit = w.ray_cast([101.0, 5.004, 201.0], [101.0, 4.999, 201.0], &RayOptions::default()).unwrap();
    assert_eq!(hit.kind, HitKind::Face);
}

#[test]
fn rotated_instance_transforms_the_ray() {
    // A quarter turn about y: local x = world z, local z = -world x.
    let ground = article(&[ground_strip(0)], &[], &[ASPHALT]);
    let mut inst = InstanceSpec::upright([0.0, 0.0, 0.0], [4.0, 1.0, 4.0], 0);
    inst.row_x = [0.0, 0.0, -1.0];
    inst.row_z = [1.0, 0.0, 0.0];
    inst.position = [30.0, -2.0, 10.0];
    let mut p = CollisionPack::parse(&pack(1, &[ground], &[inst])).unwrap();
    let i = &p.instances[0];
    for q in [[0.3, 0.4, -0.5], [3.0, 1.0, -2.0]] {
        let l = i.to_local(q);
        let back = i.to_world(l);
        assert!((0..3).all(|k| (back[k] - q[k]).abs() < 1e-5), "{q:?} -> {l:?} -> {back:?}");
    }
    assert!(i.to_local(i.position()).iter().all(|c| c.abs() < 1e-5));
    p.apply_group_exclusion();
    let mut w = CollisionWorld::new(None);
    w.insert(p);
    let hit = w.ground(33.0, 8.0, 10.0, -10.0).expect("inside the rotated tile");
    assert!((hit.point[1] + 2.0).abs() < 1e-4);
    assert!(w.ground(41.0, 10.0, 10.0, -10.0).is_none());
}

#[test]
fn skip_groups_ignores_scenery_groups() {
    let mut inst = InstanceSpec::upright([100.0, 5.0, 200.0], [4.0, 1.0, 4.0], 0);
    inst.group = 9;
    let mut w = CollisionWorld::new(None);
    w.insert(CollisionPack::parse(&pack(1, &[article(&[ground_strip(0)], &[], &[ASPHALT])], &[inst])).unwrap());
    assert!(w.ground(101.0, 201.0, 20.0, -20.0).is_some());
    let opts = RayOptions { skip_groups: true, ..RayOptions::default() };
    assert!(w.ray_cast([101.0, 20.0, 201.0], [101.0, -20.0, 201.0], &opts).is_none());
}
