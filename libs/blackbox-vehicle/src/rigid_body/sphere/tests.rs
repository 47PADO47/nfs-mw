use glam::Mat3;

use super::*;

fn expect(hit: Option<ObbContact>, point: [f32; 3], normal: [f32; 3], depth: f32) {
    let hit = hit.expect("expected a collision");
    assert!(hit.point.distance(Vec3::from_array(point)) < 0.00001, "point: {:?}", hit.point);
    assert!(hit.normal.distance(Vec3::from_array(normal)) < 0.00001, "normal: {:?}", hit.normal);
    assert!((hit.overlap - depth).abs() < 0.00001, "depth: {}", hit.overlap);
}

fn contact(at: [f32; 3], travel: [f32; 3]) -> Option<ObbContact> {
    let b = Obb { centre: Vec3::ZERO, half: Vec3::ONE, axes: Mat3::IDENTITY };
    sphere_box_contact(Sphere { centre: at.into(), radius: 0.5 }, &b, travel.into())
}

#[test]
fn pc_sphere_box_faces_edges_corners_and_inside_vectors() {
    expect(contact([1.4, 0.2, 0.0], [0.0; 3]), [1.0, 0.2, 0.0], [1.0, 0.0, 0.0], 0.10000002);
    expect(contact([-1.4, 0.2, 0.0], [0.0; 3]), [-1.0, 0.2, 0.0], [-1.0, 0.0, 0.0], 0.10000002);
    expect(contact([1.3, 1.3, 0.2], [0.0; 3]), [1.0, 1.0, 0.2], [0.70710677, 0.70710677, 0.0], 0.07573601);
    expect(contact([1.2; 3], [0.0; 3]), [1.0; 3], [0.57735026; 3], 0.15358976);
    expect(contact([0.3, 0.2, 0.1], [0.0; 3]), [1.5, 0.2, 0.1], [1.0, 0.0, 0.0], 1.2);
    expect(contact([0.0; 3], [0.0; 3]), [-1.5, 0.0, 0.0], [-1.0, 0.0, 0.0], 1.5);
    assert!(contact([1.4, 1.4, 0.4], [0.0; 3]).is_none(), "enclosing boxes falsely hit this corner");
    assert!(contact([1.5, 0.0, 0.0], [0.0; 3]).is_none(), "external tangency is not penetration");
}

#[test]
fn pc_sphere_box_sweeps_catch_crossed_faces_and_corners() {
    expect(contact([0.0, 0.0, -2.0], [0.0, 0.0, -5.0]), [0.0, 0.0, 1.5], [0.0, 0.0, 1.0], 3.5);
    expect(contact([-2.0, 1.2, 0.2], [-5.0, 0.0, 0.0]), [1.0, 1.0, 0.2], [0.9165151, 0.4000001, 0.0], 0.29999995);
    expect(contact([0.3, 0.2, 0.0], [-2.0, 0.0, 0.0]), [1.5, 0.2, 0.0], [1.0, 0.0, 0.0], 1.2);
    expect(contact([1.4, 0.2, 0.0], [-0.2, 0.0, 0.0]), [1.0, 0.2, 0.0], [1.0, 0.0, 0.0], 0.10000002);
}

#[test]
fn pc_flat_box_and_rotated_miss_and_rotation_invariance() {
    let mut b = Obb { centre: Vec3::ZERO, half: Vec3::new(1.0, 0.0, 2.0), axes: Mat3::IDENTITY };
    expect(
        sphere_box_contact(Sphere { centre: Vec3::Y * 0.3, radius: 0.5 }, &b, Vec3::ZERO),
        [0.0; 3],
        [0.0, 1.0, 0.0],
        0.19999999,
    );
    b.half = Vec3::ONE;
    b.axes = Mat3::from_rotation_y(std::f32::consts::FRAC_PI_4);
    assert!(sphere_box_contact(Sphere { centre: Vec3::new(1.1, 0.2, 1.1), radius: 0.5 }, &b, Vec3::ZERO).is_none());
    let at = b.axes * Vec3::new(1.4, 0.2, 0.0);
    expect(
        sphere_box_contact(Sphere { centre: at, radius: 0.5 }, &b, Vec3::ZERO),
        (b.axes * Vec3::new(1.0, 0.2, 0.0)).to_array(),
        b.axes.x_axis.to_array(),
        0.1,
    );
}

#[test]
fn pc_sphere_pair_surface_point_and_tangency() {
    let a = Sphere { centre: Vec3::new(1.1, 0.2, 0.0), radius: 0.5 };
    let b = Sphere { centre: Vec3::ZERO, radius: 1.0 };
    expect(sphere_contact(a, b), [0.60806507, 0.11055729, 0.0], [0.9838699, 0.17888544, 0.0], 0.381966);
    assert!(sphere_contact(Sphere { centre: Vec3::X * 1.5, ..a }, b).is_none());
    expect(sphere_contact(Sphere { centre: Vec3::ZERO, ..a }, b), [0.0; 3], [0.0, -1.0, 0.0], 0.5);
    assert!(sphere_contact(Sphere { radius: f32::NAN, ..a }, b).is_none());
}
