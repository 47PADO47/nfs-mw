use super::{
    follow::{BodyInput, Follow},
    spline::Scalar,
};
use glam::Vec3;
use nfsmw_data::camera::View;

fn view() -> View {
    View {
        fov: [90.0; 2],
        height: [1.7; 2],
        lag: [-5.44; 2],
        angle: [0.0; 2],
        stiffness: [0.5; 2],
        lat_offset: [0.0; 2],
        tilting: [true; 2],
        selectable: [true; 2],
    }
}

fn body() -> BodyInput {
    BodyInput {
        position: Vec3::ZERO,
        forward: Vec3::X,
        up: Vec3::Z,
        velocity: Vec3::ZERO,
        acceleration: Vec3::ZERO,
        speed: 0.0,
        top_speed: 71.0,
        grounded: true,
        nos_engaged: false,
        collision_damping: 0.0,
        drift: 0.0,
        up_influence: Vec3::ZERO,
        zoom: 1.0,
        look_back: false,
        gear: 1,
    }
}

#[test]
fn rolling_cubic_matches_unchanged_native_scalar_vectors() {
    let mut cubic = Scalar::default();
    cubic.target(10.0);
    let expected = [0.005285926, 0.020814722, 0.04603459, 0.08041237, 0.12343314];
    for expected in expected {
        cubic.advance(1.0 / 60.0, 1.25);
        assert!((cubic.value - expected).abs() < 3e-6, "{} != {expected}", cubic.value);
    }
    let mut cubic = Scalar::default();
    cubic.target(10.0);
    for expected in [5.0, 9.375, 10.15625] {
        cubic.advance(0.5, 1.0);
        assert!((cubic.value - expected).abs() < 1e-6);
    }
}

#[test]
fn body_origin_and_outside_view_switch_are_retained() {
    let mut follow = Follow::default();
    let first = follow.update(3, &view(), body(), 1.0 / 60.0);
    assert!((first.eye - Vec3::new(-5.44, 0.0, 1.7)).length() < 1e-6);
    let mut further = view();
    further.lag = [-20.0; 2];
    let switched = follow.update(4, &further, body(), 1.0 / 60.0);
    assert_eq!(switched.eye, first.eye);
    let advanced = follow.update(4, &further, body(), 1.0 / 60.0);
    assert!(advanced.eye.x < switched.eye.x);
    assert!(advanced.eye.x > -20.0);
    let mut bumper = view();
    bumper.lag = [0.5; 2];
    bumper.height = [0.746; 2];
    let inside = follow.update(0, &bumper, body(), 1.0 / 60.0);
    assert!((inside.eye.x - 0.5).abs() < 1e-6);
    assert!((inside.eye.z - 0.746).abs() < 1e-6);
}

#[test]
fn reverse_does_not_turn_the_follow_camera_to_the_front() {
    let mut follow = Follow::default();
    let mut reverse = body();
    reverse.velocity = Vec3::new(-30.0, 0.0, 0.0);
    reverse.speed = 30.0;
    for _ in 0..180 {
        let sample = follow.update(3, &view(), reverse, 1.0 / 60.0);
        assert!(sample.eye.x < 0.0);
        assert!((sample.target - sample.eye).x > 0.0);
    }
}

#[test]
fn movement_relative_follow_changes_after_the_native_delay() {
    let mut follow = Follow::default();
    follow.update(3, &view(), body(), 1.0 / 60.0);
    let mut turning = body();
    turning.speed = 30.0;
    turning.velocity = Vec3::new(20.0, 20.0, 0.0);
    let first = follow.update(3, &view(), turning, 1.0 / 60.0);
    assert!(first.eye.y.abs() < 1e-6);
    for _ in 0..90 {
        follow.update(3, &view(), turning, 1.0 / 60.0);
    }
    let final_sample = follow.update(3, &view(), turning, 1.0 / 60.0);
    assert!(final_sample.eye.y < -1.0);
    assert!(final_sample.up.is_finite());
}

#[test]
fn nitrous_adds_the_native_fov_boost_only_to_moving_outside_views() {
    let mut driving = body();
    driving.speed = 20.0;
    driving.nos_engaged = true;
    let outside = Follow::default().update(3, &view(), driving, 1.0 / 60.0);
    assert_eq!(outside.fov_bangle, 16384.0 + 1638.0);
    let inside = Follow::default().update(1, &view(), driving, 1.0 / 60.0);
    assert_eq!(inside.fov_bangle, 16384.0);
    driving.speed = 0.0;
    let stationary = Follow::default().update(3, &view(), driving, 1.0 / 60.0);
    assert_eq!(stationary.fov_bangle, 16384.0);
}
