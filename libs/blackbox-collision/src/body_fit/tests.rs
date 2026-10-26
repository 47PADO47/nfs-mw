use super::*;
use crate::{BodyGeometry, GeometryOptions, Shape, read_bounds_sets, write_bounds_pack};
use glam::Vec3 as GVec;

/// A synthetic car: a long low body 1.8 m wide, 4.4 m long and 0.9 m tall with a narrower, higher cabin on its rear
/// half, on a ground at y = 0. Sampled on a grid so every face has points.
fn car() -> Vec<Vec3> {
    let mut points = Vec::new();
    let grid = |lo: f32, hi: f32, n: usize| (0..=n).map(move |i| lo + (hi - lo) * i as f32 / n as f32);
    for x in grid(-0.9, 0.9, 6) {
        for y in grid(0.0, 0.9, 4) {
            for z in grid(-2.2, 2.2, 12) {
                points.push([x, y, z]);
            }
        }
    }
    for x in grid(-0.6, 0.6, 4) {
        for y in grid(0.9, 1.3, 3) {
            for z in grid(-1.4, 0.6, 8) {
                points.push([x, y, z]);
            }
        }
    }
    points
}

fn fitted() -> BoundsSet {
    fit_body(0xABCD, &car(), &FitOptions::default()).unwrap()
}

#[test]
fn the_root_box_is_the_extent_of_the_points() {
    let set = fitted();
    let root = set.root();
    assert_eq!(root.shape(), Shape::Box);
    assert_eq!(root.half_dimensions, [0.9, 0.65, 2.2]);
    assert_eq!(root.pivot, [0.0, 0.65, 0.0]);
    assert_eq!(root.child_count as usize, set.nodes.len() - 1);
}

#[test]
fn the_tree_is_valid_and_a_vehicle_body() {
    let set = fitted();
    assert!(set.is_tree());
    set.check_vehicle_body(&BodyLimits::default()).unwrap();
    assert_eq!(set.point_clouds.len(), 1);
    assert_eq!(set.nodes.iter().filter(|n| n.point_cloud == Some(0)).count(), 1);
}

#[test]
fn the_lower_box_covers_the_footprint_up_to_the_lower_height() {
    let set = fitted();
    let lower = &set.nodes[1];
    assert_eq!(lower.flags, BODY_BOX);
    assert_eq!(lower.half_dimensions, [0.9, 0.376, 2.2]);
    // Relative to the root pivot (y = 0.65): the box spans 0 to 0.752.
    assert_eq!(lower.position, [0.0, -0.274, 0.0]);
    assert_eq!(lower.pivot, [0.0, 0.376, 0.0]);
}

#[test]
fn the_cabin_box_stands_over_the_high_part_only() {
    let set = fitted();
    let cabin = &set.nodes[2];
    assert_eq!(cabin.flags, BODY_BOX);
    assert!((cabin.half_dimensions[0] - 0.6).abs() < 0.002, "{:?}", cabin.half_dimensions);
    // The cabin's z span is the roof footprint, -1.4..0.6.
    assert!((cabin.pivot[2] - -0.4).abs() < 0.002 && (cabin.half_dimensions[2] - 1.0).abs() < 0.002);
    // It sits above the lower box (top 0.752) and reaches the roof (1.3).
    let bottom = cabin.pivot[1] - cabin.half_dimensions[1];
    let top = cabin.pivot[1] + cabin.half_dimensions[1];
    assert!(bottom >= 0.752 && (top - 1.3).abs() < 0.002, "{bottom} {top}");
}

#[test]
fn corner_spheres_sit_at_the_nose_and_the_cabin_roof() {
    let set = fitted();
    let spheres: Vec<&Bounds> = set.nodes.iter().filter(|n| n.shape() == Shape::Sphere).collect();
    assert_eq!(spheres.len(), 6);
    for sphere in &spheres {
        assert_eq!(sphere.flags, CORNER_SPHERE);
        assert_eq!(sphere.half_dimensions, [q(sphere.radius); 3]);
    }
    let front: Vec<_> = spheres.iter().filter(|s| s.radius > 0.3).collect();
    assert_eq!(front.len(), 4);
    for s in &front {
        assert!(s.pivot[2] > 1.9 && s.pivot[2] < 2.2, "nose {:?}", s.pivot);
        assert!(s.pivot[0].abs() > 0.5 && s.pivot[0].abs() < 0.9);
    }
    let roof: Vec<_> = spheres.iter().filter(|s| s.radius < 0.3).collect();
    assert_eq!(roof.len(), 2);
    for s in &roof {
        assert!(s.pivot[1] > 1.1 && s.pivot[1] < 1.3, "roof {:?}", s.pivot);
        assert!(s.pivot[0].abs() < 0.6);
    }
    // Left and right mirror each other.
    let sum: f32 = spheres.iter().map(|s| s.pivot[0]).sum();
    assert!(sum.abs() < 0.01);
}

#[test]
fn children_pivots_are_the_root_pivot_plus_their_position() {
    let set = fitted();
    let root = set.root();
    for node in &set.nodes[1..] {
        for a in 0..3 {
            assert!((node.pivot[a] - (root.pivot[a] + node.position[a])).abs() < 0.0011, "{node:?}");
        }
    }
}

#[test]
fn the_cloud_is_a_mirrored_silhouette_inside_the_root_box() {
    let set = fitted();
    let cloud = &set.point_clouds[0];
    assert!(cloud.len() >= 6 && cloud.len() % 2 == 0 && cloud.len() <= 16);
    let root = set.root();
    for p in cloud {
        for a in 0..3 {
            assert!(p[a].abs() <= root.half_dimensions[a] + 0.002, "{p:?} outside {:?}", root.half_dimensions);
        }
    }
    let (right, left) = cloud.split_at(cloud.len() / 2);
    for (r, l) in right.iter().zip(left) {
        assert!((r[0] + l[0]).abs() < 1e-5 && r[1] == l[1] && r[2] == l[2]);
    }
    // Relative to the root pivot, so the lowest points are at -half height and the tallest at the roof.
    let low = cloud.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
    let high = cloud.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
    assert!((low - -0.65).abs() < 1e-3 && (high - 0.65).abs() < 1e-3);
}

#[test]
fn every_point_lies_inside_the_root_box() {
    let set = fitted();
    let root = set.root();
    for p in car() {
        for a in 0..3 {
            assert!((p[a] - root.pivot[a]).abs() <= root.half_dimensions[a] + 0.001);
        }
    }
}

#[test]
fn the_vehicle_code_reads_the_fitted_set_as_two_boxes_six_spheres_and_a_ground_mesh() {
    let set = fitted();
    let bytes = write_bounds_pack(std::slice::from_ref(&set)).unwrap();
    let read = read_bounds_sets(&bytes).unwrap().remove(0);
    let options = GeometryOptions {
        fallback_half: GVec::from_array(read.root().half_dimensions),
        fallback_shape: Shape::Box,
        box_padding: GVec::ZERO,
        surface: 0,
    };
    let body = BodyGeometry::from_bounds(&read, 0, options).unwrap();
    assert!(!body.used_fallback);
    assert_eq!(body.primitives.iter().filter(|p| p.shape == Shape::Box).count(), 2);
    assert_eq!(body.primitives.iter().filter(|p| p.shape == Shape::Sphere).count(), 6);
    assert_eq!(body.ground_meshes.len(), 1);
    assert_eq!(body.ground_meshes[0].points.len(), set.point_clouds[0].len());
    let cog = body.centre_of_gravity();
    assert!(cog.y < 0.3 && cog.length() < 1.0, "{cog}");
}

#[test]
fn a_narrow_body_gets_smaller_spheres() {
    let narrow: Vec<Vec3> = car().into_iter().map(|p| [p[0] * 0.4, p[1], p[2]]).collect();
    let set = fit_body(1, &narrow, &FitOptions::default()).unwrap();
    let largest = set.nodes.iter().filter(|n| n.shape() == Shape::Sphere).map(|n| n.radius).fold(0.0, f32::max);
    assert!(largest <= 0.4 * set.root().half_dimensions[0] + 0.001);
}

#[test]
fn a_body_with_no_high_part_has_no_cabin_and_no_roof_spheres() {
    let flat: Vec<Vec3> = car().into_iter().filter(|p| p[1] <= 0.5).collect();
    let set = fit_body(1, &flat, &FitOptions::default()).unwrap();
    assert_eq!(set.nodes.iter().filter(|n| n.shape() == Shape::Box && n.flags == BODY_BOX).count(), 1);
    assert_eq!(set.nodes.iter().filter(|n| n.shape() == Shape::Sphere).count(), 4);
    set.check_vehicle_body(&BodyLimits::default()).unwrap();
}

#[test]
fn too_few_points_or_too_small_a_body_is_refused() {
    assert!(fit_body(1, &car()[..5], &FitOptions::default()).is_err());
    let tiny: Vec<Vec3> = car().into_iter().map(|p| [p[0] * 0.1, p[1] * 0.1, p[2] * 0.1]).collect();
    assert!(fit_body(1, &tiny, &FitOptions::default()).is_err());
    let mut nan = car();
    nan.iter_mut().for_each(|p| p[0] = f32::NAN);
    assert!(fit_body(1, &nan, &FitOptions::default()).is_err());
}

#[test]
fn points_that_are_not_finite_are_ignored() {
    let mut points = car();
    points.push([f32::NAN, 0.0, 0.0]);
    points.push([0.0, f32::INFINITY, 0.0]);
    assert_eq!(
        fit_body(1, &points, &FitOptions::default()).unwrap(),
        fit_body(1, &car(), &FitOptions::default()).unwrap()
    );
}

#[test]
fn validation_refuses_what_a_vehicle_cannot_be() {
    let limits = BodyLimits::default();
    let mut set = fitted();
    set.nodes[0].flags = flags::SPHERE;
    assert!(set.check_vehicle_body(&limits).is_err(), "root not a box");
    let mut set = fitted();
    set.nodes[0].half_dimensions[2] = 40.0;
    assert!(set.check_vehicle_body(&limits).is_err(), "too long");
    let mut set = fitted();
    set.nodes[3].first_child = 2;
    set.nodes[3].child_count = 1;
    assert!(set.check_vehicle_body(&limits).is_err(), "node reached twice");
    let mut set = fitted();
    set.point_clouds[0].clear();
    assert!(set.check_vehicle_body(&limits).is_err(), "empty cloud");
    let mut set = fitted();
    for node in &mut set.nodes[1..] {
        node.flags = flags::BOX;
    }
    assert!(set.check_vehicle_body(&limits).is_err(), "nothing collides");
}
