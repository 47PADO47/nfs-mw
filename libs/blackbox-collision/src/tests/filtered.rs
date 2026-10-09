use super::synth::*;
use crate::{BARRIER_TWO_SIDED, CollisionPack, CollisionWorld, Hit, HitKind, RayOptions};

const SURFACE: u32 = 7;

fn barrier(x: f32, flags: u8) -> BarrierSpec {
    BarrierSpec { p0: [x, -1.0, -4.0], p1: [x, 3.0, 4.0], surface: 0, flags }
}

fn accept_wall(hit: &Hit) -> bool {
    if hit.kind == HitKind::Face && hit.normal[1].abs() > 0.7 {
        return false;
    }
    hit.kind != HitKind::Barrier || hit.front_facing || hit.surface_flags & BARRIER_TWO_SIDED != 0
}

fn world(articles: &[Vec<u8>], instances: &[InstanceSpec]) -> CollisionWorld {
    let mut w = CollisionWorld::new(None);
    w.insert(CollisionPack::parse(&pack(7, articles, instances)).unwrap());
    w
}

#[test]
fn rejected_barrier_does_not_hide_a_wall_in_the_same_article() {
    let w = world(
        &[article(&[], &[barrier(1.0, 2), barrier(2.0, 2 | BARRIER_TWO_SIDED)], &[SURFACE])],
        &[InstanceSpec::upright([0.0; 3], [4.0; 3], 0)],
    );
    let (a, b, options) = ([0.0, 1.0, 0.0], [3.0, 1.0, 0.0], RayOptions::default());
    let raw = w.ray_cast(a, b, &options).unwrap();
    assert_eq!(raw.point[0], 1.0);
    assert!(!raw.front_facing);
    let filtered = w.ray_cast_filtered(a, b, &options, accept_wall).unwrap();
    assert_eq!(filtered.point[0], 2.0);
    assert!(!filtered.front_facing, "the farther wall is two-sided");
    assert_eq!((filtered.section, filtered.instance, filtered.surface_hash), (7, 0, SURFACE));
    assert_eq!(w.ray_cast_filtered(a, b, &options, |_| true), Some(raw));
    assert!(w.ray_cast_filtered(a, b, &options, |_| false).is_none());
}

#[test]
fn rejected_barrier_does_not_hide_another_instance() {
    let w = world(
        &[
            article(&[], &[barrier(1.0, 2)], &[SURFACE]),
            article(&[], &[barrier(2.0, 2 | BARRIER_TWO_SIDED)], &[SURFACE]),
        ],
        &[InstanceSpec::upright([0.0; 3], [4.0; 3], 0), InstanceSpec::upright([0.0; 3], [4.0; 3], 1)],
    );
    let mut observed = Vec::new();
    let hit = w
        .ray_cast_filtered([0.0, 1.0, 0.0], [3.0, 1.0, 0.0], &RayOptions::default(), |candidate| {
            observed.push((candidate.section, candidate.instance));
            accept_wall(candidate)
        })
        .unwrap();
    assert_eq!(hit.instance, 1);
    assert_eq!(observed, [(7, 0), (7, 1)], "the predicate sees populated identity");
}

#[test]
fn rejected_floor_from_either_side_does_not_hide_a_barrier() {
    let w = world(
        &[article(&[ground_strip(0)], &[barrier(2.0, 2 | BARRIER_TWO_SIDED)], &[SURFACE])],
        &[InstanceSpec::upright([0.0; 3], [4.0; 3], 0)],
    );
    for (start_y, end_y) in [(-0.5, 1.0), (0.5, -0.75)] {
        let (a, b) = ([0.0, start_y, 0.0], [3.0, end_y, 0.0]);
        let raw = w.ray_cast(a, b, &RayOptions::default()).unwrap();
        assert_eq!(raw.kind, HitKind::Face);
        assert_eq!(raw.normal[1], start_y.signum());
        let hit = w.ray_cast_filtered(a, b, &RayOptions::default(), accept_wall).unwrap();
        assert_eq!(hit.kind, HitKind::Barrier);
        assert!((hit.point[0] - 2.0).abs() < 1e-5);
    }
}

#[test]
fn rejected_floor_does_not_hide_a_steep_face() {
    let steep = StripSpec {
        center: [2.0, 0.0, 0.0],
        radius: 8 * 16,
        flags: 0,
        verts: vec![
            vert(0, -128, -512, 0, 0),
            vert(0, 384, -512, 0, 0),
            vert(0, -128, 512, 0, 0),
            vert(0, 384, 512, 0, 0),
        ],
    };
    let w =
        world(&[article(&[ground_strip(0), steep], &[], &[SURFACE])], &[InstanceSpec::upright([0.0; 3], [4.0; 3], 0)]);
    let hit = w.ray_cast_filtered([0.0, -0.5, 0.0], [3.0, 1.0, 0.0], &RayOptions::default(), accept_wall).unwrap();
    assert_eq!(hit.kind, HitKind::Face);
    assert!((hit.point[0] - 2.0).abs() < 1e-5);
    assert_eq!(hit.normal, [-1.0, 0.0, 0.0]);
}

#[test]
fn nearest_eligible_candidate_can_come_from_a_later_pack() {
    let mut w = world(&[article(&[], &[barrier(1.0, 2)], &[SURFACE])], &[InstanceSpec::upright([0.0; 3], [4.0; 3], 0)]);
    w.insert(
        CollisionPack::parse(&pack(
            8,
            &[article(&[], &[barrier(2.0, 2 | BARRIER_TWO_SIDED)], &[SURFACE])],
            &[InstanceSpec::upright([0.0; 3], [4.0; 3], 0)],
        ))
        .unwrap(),
    );
    let hit = w.ray_cast_filtered([0.0, 1.0, 0.0], [3.0, 1.0, 0.0], &RayOptions::default(), accept_wall).unwrap();
    assert_eq!((hit.section, hit.instance), (8, 0));
    assert_eq!(hit.point[0], 2.0);
}

#[test]
fn world_space_predicate_handles_rotated_instances() {
    let mut instance = InstanceSpec::upright([10.0, 0.0, 20.0], [4.0; 3], 0);
    instance.row_x = [0.0, 0.0, -1.0];
    instance.row_z = [1.0, 0.0, 0.0];
    let w = world(&[article(&[], &[barrier(1.0, 2 | BARRIER_TWO_SIDED)], &[SURFACE])], &[instance]);
    let hit = w
        .ray_cast_filtered([10.0, 1.0, 20.0], [10.0, 1.0, 23.0], &RayOptions::default(), |h| {
            assert_eq!(h.normal, [0.0, 0.0, -1.0]);
            assert_eq!((h.section, h.instance), (7, 0));
            true
        })
        .unwrap();
    assert!((hit.point[2] - 21.0).abs() < 1e-5);
}
