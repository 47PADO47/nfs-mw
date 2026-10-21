use super::*;
use crate::carp::tag4;
use crate::tests::synth::{carp, push_f32, push_u16, push_u32};
use crate::{Article, FLAG_Y_NOT_UP, Grid, Instance};

fn wall(x: f32) -> Barrier {
    Barrier { p0: [x, -1.0, -2.0], p1: [x, 1.0, 2.0], surface: 0, flags: 0, inv_xz_len: 0.25 }
}

fn pack(section: u32, barriers: Vec<Barrier>) -> CollisionPack {
    CollisionPack {
        section,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            half_width: 10.0,
            half_length: 10.0,
            half_height: 10.0,
            translation: [0.0; 3],
            radius: 15.0,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        }],
        articles: vec![Article { strips: vec![], barriers, surfaces: vec![17, 29], intermediate_object: 0, flags: 0 }],
        objects: vec![],
    }
}

fn query(world: &CollisionWorld, point: Vec3, radius: f32) -> Vec<WorldBarrier> {
    let mut result = Vec::new();
    world.barriers_near_into(point, radius, &RayOptions::default(), &mut result);
    result
}

#[test]
fn collects_off_centre_walls_with_original_radius_and_height_boundaries() {
    let mut world = CollisionWorld::new(None);
    let off_centre = Barrier { p0: [2.0, -1.0, 1.0], p1: [2.0, 1.0, 3.0], inv_xz_len: 0.5, ..wall(2.0) };
    world.insert(pack(1, vec![off_centre]));
    assert_eq!(query(&world, [0.0; 3], 3.0)[0].distance_sq, 5.0);
    assert!(query(&world, [0.0; 3], 5.0_f32.sqrt()).is_empty(), "radius tangency is excluded");
    assert!(query(&world, [2.0, -4.0, 2.0], 3.0).is_empty(), "lower height tangency is excluded");
    assert_eq!(query(&world, [2.0, 4.0, 2.0], 3.0).len(), 1, "upper height tangency is included");
}

#[test]
fn keeps_authored_distance_projection_normal_scale_and_transformed_surface() {
    let mut world = CollisionWorld::new(None);
    let barrier = Barrier { inv_xz_len: 0.125, surface: 1, flags: 2, ..wall(1.0) };
    let mut p = pack(7, vec![barrier]);
    p.instances[0].translation = [-10.0, -3.0, -20.0];
    world.insert(p);
    let result = query(&world, [10.0, 3.0, 20.0], 3.0);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].barrier.p0, [11.0, 2.0, 18.0]);
    assert_eq!(result[0].barrier.p1, [11.0, 4.0, 22.0]);
    assert_eq!(result[0].barrier.inv_xz_len, 0.125);
    assert_eq!(result[0].normal, [0.5, 0.0, 0.0]);
    assert_eq!(result[0].distance_sq, 3.25, "authored fraction is 0.125, not the geometric 0.5");
    assert_eq!((result[0].surface_hash, result[0].barrier.surface, result[0].barrier.flags), (29, 1, 2));
    assert_eq!((result[0].section, result[0].instance), (7, 0));
}

#[test]
fn tilted_endpoint_normal_remains_horizontal_and_is_not_renormalized() {
    let mut p = pack(1, vec![wall(0.0)]);
    p.instances[0].flags = FLAG_Y_NOT_UP;
    p.instances[0].row_z = [0.0, -0.6, 0.8];
    let mut world = CollisionWorld::new(None);
    world.insert(p);
    let hit = query(&world, [0.0; 3], 3.0)[0];
    assert_eq!(hit.normal[1], 0.0);
    assert!((hit.normal[0] - 0.5).abs() < 1e-6);
    assert_eq!(hit.barrier.inv_xz_len, 0.25);
}

#[test]
fn sorts_distances_stably_and_always_tests_supplemental_packs() {
    let mut world = CollisionWorld::new(None);
    world.insert(pack(3, vec![wall(2.0), wall(1.0), wall(-1.0)]));
    world.insert_ungridded(pack(3, vec![wall(0.5)]));
    let hits = query(&world, [0.0; 3], 3.0);
    assert_eq!(hits.iter().map(|h| h.barrier.p0[0]).collect::<Vec<_>>(), [0.5, 1.0, -1.0, 2.0]);
    world.remove_ungridded(3);
    assert_eq!(query(&world, [0.0; 3], 3.0).len(), 3);
}

#[test]
fn source_group_and_instance_surface_exclusion_gates_are_independent() {
    let mut p = pack(1, vec![Barrier { flags: 8, ..wall(1.0) }, wall(2.0)]);
    p.instances[0].group = 7;
    let mut world = CollisionWorld::new(None);
    world.insert(p);
    assert!(query(&world, [0.0; 3], 3.0).is_empty(), "group gate applies even with default ray options");
    world.set_group_enabled(7, true);
    let mut hits = Vec::new();
    let options = RayOptions { exclude: u32::from(GROUP_EXCLUSION) | 8, ..RayOptions::default() };
    world.barriers_near_into([0.0; 3], 3.0, &options, &mut hits);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].barrier.p0[0], 2.0);
    assert_eq!(hits[0].barrier.flags, GROUP_EXCLUSION, "output retains authored/runtime flags");
    let mut p = world.remove(1).unwrap();
    p.instances[0].flags |= 16;
    world.insert(p);
    world.barriers_near_into([0.0; 3], 3.0, &RayOptions { exclude: 16, ..options }, &mut hits);
    assert!(hits.is_empty(), "instance exclusion is separate from the surface mask");
}

fn grid() -> Grid {
    let mut head = Vec::new();
    for value in [-4.0, 0.0, -4.0, 1.0, 4.0, 0.25] {
        push_f32(&mut head, value);
    }
    for value in [2, 2, 0] {
        push_u32(&mut head, value);
    }
    let mut nodes = Vec::new();
    for index in [1, 2] {
        push_u32(&mut nodes, 0);
        push_u16(&mut nodes, index);
        push_u16(&mut nodes, 0);
        nodes.extend_from_slice(&[2, 0, 0, 0]);
        for offset in [0, 8, 8, 8] {
            push_u16(&mut nodes, offset);
        }
        push_u32(&mut nodes, 1 << 16);
        push_u32(&mut nodes, 2 << 16); // Unloaded pack is a valid streaming gap.
    }
    Grid::parse(&carp(tag4(b"CDat"), &[(tag4(b"CGrd"), 1, head), (tag4(b"CGcn"), 2, nodes)])).unwrap()
}

#[test]
fn square_grid_query_covers_off_diagonal_cells_deduplicates_and_skips_missing_packs() {
    let mut world = CollisionWorld::new(Some(grid()));
    world.insert(pack(1, vec![wall(1.0)]));
    world.insert(pack(9, vec![wall(-1.0)])); // Not named by the grid.
    world.insert_ungridded(pack(8, vec![wall(-2.0)]));
    let hits = query(&world, [0.0; 3], 3.0);
    assert_eq!(hits.len(), 2, "square cells include both references, but each instance is tested once");
    assert_eq!(hits.iter().map(|h| h.section).collect::<Vec<_>>(), [1, 8]);
    world.remove(1);
    assert_eq!(query(&world, [0.0; 3], 3.0).iter().map(|h| h.section).collect::<Vec<_>>(), [8]);
}

#[test]
fn invalid_or_disabled_queries_clear_reused_output() {
    let mut world = CollisionWorld::new(None);
    world.insert(pack(1, vec![wall(1.0)]));
    let mut out = query(&world, [0.0; 3], 3.0);
    for radius in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
        world.barriers_near_into([0.0; 3], radius, &RayOptions::default(), &mut out);
        assert!(out.is_empty());
    }
    world.barriers_near_into([f32::NAN, 0.0, 0.0], 3.0, &RayOptions::default(), &mut out);
    assert!(out.is_empty());
    world.barriers_near_into([0.0; 3], 3.0, &RayOptions { barriers: false, ..RayOptions::default() }, &mut out);
    assert!(out.is_empty());
}
