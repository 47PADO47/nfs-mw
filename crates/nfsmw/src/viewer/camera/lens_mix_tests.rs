use super::*;

fn lens(eye: Vec3, target: Vec3, fov: f32) -> Lens {
    Lens::look_at(eye, target, fov, 0.3, 16.0 / 9.0)
}

#[test]
fn the_ends_are_the_lenses_themselves() {
    let (a, b) = (lens(Vec3::ZERO, Vec3::X, 60.0), lens(Vec3::new(0.0, 5.0, 2.0), Vec3::new(3.0, 5.0, 0.0), 80.0));
    assert_eq!(a.mix(&b, 0.0), a);
    assert_eq!(a.mix(&b, 1.0), b);
    assert_eq!(a.mix(&b, -3.0), a);
    assert_eq!(a.mix(&b, 7.0), b);
}

#[test]
fn the_middle_is_half_way_in_place_and_field_of_view() {
    let a = lens(Vec3::new(0.0, 0.0, 1.0), Vec3::new(10.0, 0.0, 1.0), 60.0);
    let b = lens(Vec3::new(0.0, 10.0, 3.0), Vec3::new(10.0, 10.0, 3.0), 80.0);
    let m = a.mix(&b, 0.5);
    assert!((m.position - Vec3::new(0.0, 5.0, 2.0)).length() < 1e-5, "{}", m.position);
    assert!((m.fov_y_degrees - 70.0).abs() < 1e-5);
    assert!((m.forward - Vec3::X).length() < 1e-4, "both look the same way: {}", m.forward);
    // The view matrix puts the lens' own place at the origin and looks down -Z.
    assert!(m.view.transform_point3(m.position).length() < 1e-4);
    assert!((m.view.transform_vector3(Vec3::X) - Vec3::NEG_Z).length() < 1e-4);
}

#[test]
fn the_turn_is_blended_and_the_roll_kept() {
    let a = lens(Vec3::ZERO, Vec3::X, 60.0);
    let mut rolled = lens(Vec3::ZERO, Vec3::X, 60.0);
    rolled.view = glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::X, Vec3::new(0.0, 1.0, 1.0).normalize());
    let m = a.mix(&rolled, 0.5);
    // A point up on the world's Z lands to the side of the middle of the picture once rolled.
    let sideways = m.view.transform_point3(Vec3::new(5.0, 0.0, 1.0)).x;
    assert!(sideways.abs() > 0.1, "rolled by half: {sideways}");
    let full = rolled.view.transform_point3(Vec3::new(5.0, 0.0, 1.0)).x;
    assert!(sideways.abs() < full.abs(), "less than the whole roll");
}

#[test]
fn the_projection_takes_the_mixed_field_of_view_and_the_nearer_near_plane() {
    let a = Lens::look_at(Vec3::ZERO, Vec3::X, 60.0, 0.1, 2.0);
    let b = Lens::look_at(Vec3::Y, Vec3::X, 100.0, 0.5, 2.0);
    let Projection::PerspectiveInfiniteReverse { fov_y, aspect, near } = a.mix(&b, 0.5).projection else {
        panic!("a perspective");
    };
    assert!((fov_y.to_degrees() - 80.0).abs() < 1e-3 && aspect == 2.0 && near == 0.1);
}
