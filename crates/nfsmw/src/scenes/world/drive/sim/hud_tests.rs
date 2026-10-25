use super::*;

/// A body on a circle of `radius` metres about the y axis (physics space: y up), at `speed` m/s, `t` seconds in: its
/// rotation (nose along the velocity), velocity and angular velocity. It starts moving along +x and turns about +y.
fn on_circle(radius: f32, speed: f32, t: f32) -> (Mat3, Vec3, Vec3) {
    let rate = speed / radius;
    let angle = rate * t;
    let velocity = Vec3::new(angle.cos(), 0.0, -angle.sin()) * speed;
    let rotation = Mat3::from_rotation_y(std::f32::consts::FRAC_PI_2 + angle);
    (rotation, velocity, Vec3::new(0.0, rate, 0.0))
}

#[test]
fn the_nose_of_the_circling_body_points_along_its_velocity() {
    let (rotation, velocity, _) = on_circle(40.0, 20.0, 0.7);
    assert!((rotation * Vec3::Z).abs_diff_eq(velocity.normalize(), 1e-5));
}

#[test]
fn lateral_g_on_a_known_circle_is_v_squared_over_r() {
    // 20 m/s on a 40 m circle: 10 m/s², 1.02 g.
    let (rotation, velocity, angular) = on_circle(40.0, 20.0, 0.0);
    let g = lateral_g(rotation, velocity, angular, true);
    assert!((g.abs() - 10.0 / G).abs() < 1e-4, "{g}");
}

#[test]
fn lateral_g_points_the_way_the_velocity_turns() {
    // The velocity's change over a short step, along the body's right axis, has the sign the formula gives.
    let (dt, t) = (1e-3, 0.4);
    let (rotation, velocity, angular) = on_circle(40.0, 20.0, t);
    let (_, later, _) = on_circle(40.0, 20.0, t + dt);
    let measured = ((later - velocity) / dt).dot(rotation * Vec3::X) / G;
    let g = lateral_g(rotation, velocity, angular, true);
    assert!((g - measured).abs() < 1e-2, "formula {g}, measured {measured}");
    let (rotation, velocity, angular) = on_circle(40.0, 20.0, 0.0);
    let mirrored = lateral_g(rotation, velocity, -angular, true);
    assert!((mirrored + g).abs() < 1e-4, "turning the other way flips the sign");
}

#[test]
fn a_car_in_the_air_or_driving_straight_reads_no_lateral_g() {
    let (rotation, velocity, angular) = on_circle(40.0, 20.0, 0.0);
    assert_eq!(lateral_g(rotation, velocity, angular, false), 0.0);
    assert_eq!(lateral_g(Mat3::IDENTITY, Vec3::new(0.0, 0.0, 30.0), Vec3::ZERO, true), 0.0);
}

#[test]
fn the_shift_point_is_the_current_gears_up_shift() {
    // Gear ids: 0 reverse, 1 neutral, 2 first, ... 5 top.
    let up = [0.0, 0.0, 6800.0, 7000.0, 7100.0, 7400.0];
    assert_eq!(shift_up_rpm(&up, 2, 5), Some(6800.0));
    assert_eq!(shift_up_rpm(&up, 4, 5), Some(7100.0));
    assert_eq!(shift_up_rpm(&up, 5, 5), None, "no gear after the top one");
    assert_eq!(shift_up_rpm(&up, 0, 5), None, "reverse");
    assert_eq!(shift_up_rpm(&up, 1, 5), None, "neutral");
    assert_eq!(shift_up_rpm(&[], 2, 5), None, "a box without a table");
}

#[test]
fn a_tyre_reads_its_state_without_the_fuse() {
    assert_eq!(tire_condition(TireState::None), TireCondition::Sound);
    assert_eq!(tire_condition(TireState::Punctured(0.3)), TireCondition::Punctured);
    assert_eq!(tire_condition(TireState::Blown), TireCondition::Blown);
}
