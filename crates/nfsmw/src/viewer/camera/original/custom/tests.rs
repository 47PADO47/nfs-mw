use super::*;
use glam::{Mat4, Vec3};

fn lens(eye: Vec3, fov: f32) -> Lens {
    Lens::look_at(eye, Vec3::Z, fov, 0.3, 16.0 / 9.0)
}

#[test]
fn custom_optics_round_trip_preserves_roll_fov_projection_and_direction() {
    for aspect in [4.0 / 3.0, 16.0 / 9.0, 32.0 / 9.0] {
        for fov in [36.0, 60.0, 68.0, 96.0] {
            let mut input = lens(Vec3::new(-7.0, 2.0, 3.0), fov);
            input.view = glam::camera::rh::view::look_at_mat4(
                input.position,
                input.position + input.forward,
                Vec3::new(0.1, 0.2, 1.0).normalize(),
            );
            let sample = custom_sample(input);
            let result = custom_lens(sample, aspect);
            let reference = crate::viewer::camera::projection(fov, aspect, 0.3);
            assert!((result.forward - input.forward).length() < 1e-5);
            assert!((result.fov_y_degrees - fov).abs() < 1e-5);
            assert!(result.view.abs_diff_eq(input.view, 1e-5));
            assert!(result.projection.matrix().abs_diff_eq(reference.matrix(), 1e-5));
        }
    }
}

#[test]
fn free_orbit_low_eye_uses_floor_height_without_shortening_road_distance() {
    for angle in [0.0_f32, 0.4, 1.5, 2.4, 3.1, 4.8] {
        let eye = Vec3::new(angle.cos() * 6.0, angle.sin() * 6.0, -0.2);
        let result = constrain_custom(
            Vec3::ZERO,
            lens(eye, 60.0),
            16.0 / 9.0,
            false,
            &[],
            |_, _| None,
            |query, _, _| {
                assert_eq!(query.z, 0.5);
                Some(0.0)
            },
        );
        assert_eq!(result.position.truncate(), eye.truncate());
        assert_eq!(result.position.z, 0.25);
    }
}

#[test]
fn final_wall_check_has_no_old_minimum_distance_and_is_stateless() {
    for fov in [36.0, 60.0, 96.0] {
        let input = lens(Vec3::new(-10.0, 0.0, 2.0), fov);
        let cast = |a: Vec3, b: Vec3| {
            let t = (-2.0 - a.x) / (b.x - a.x);
            (0.0..=1.0).contains(&t).then(|| CameraHit { point: a.lerp(b, t), normal: Vec3::X })
        };
        let check = || constrain_custom(Vec3::ZERO, input, 16.0 / 9.0, false, &[], cast, |_, _, _| None);
        let result = check();
        assert!(result.position.x > -2.0 && result.position.x < 0.0, "{:?}", result.position);
        assert!(result.view_proj().is_finite());
        for _ in 0..360 {
            assert_eq!(check(), result);
        }
        assert!((result.fov_y_degrees - fov).abs() < 1e-5);
    }
}

#[test]
fn final_body_clearance_uses_full_roof_height_on_the_first_draw() {
    let eye = Vec3::new(-6.0, 0.0, 0.8);
    let body = Avoidable {
        transform: Mat4::from_translation(eye),
        half_extents: Vec3::new(2.0, 1.0, 1.0),
        velocity: Vec3::ZERO,
    };
    let result = constrain_custom(Vec3::ZERO, lens(eye, 60.0), 16.0 / 9.0, false, &[body], |_, _| None, |_, _, _| None);
    assert!((result.position.z - 2.65).abs() < 1e-5);
}

fn replay(fps: u32, scale: f32) -> (super::super::Sample, u64) {
    use super::super::{BodyInput, Frame, NoiseControls, PhysicsFrame, Timeline};
    let body = |time: f32| BodyInput {
        position: Vec3::X * time * 10.0,
        forward: Vec3::X,
        up: Vec3::Z,
        velocity: Vec3::X * 10.0,
        acceleration: Vec3::ZERO,
        speed: 10.0,
        top_speed: 70.0,
        grounded: true,
        nos_engaged: false,
        collision_damping: 0.0,
        drift: 0.0,
        up_influence: Vec3::ZERO,
        zoom: 1.0,
        look_back: false,
        gear: 2,
    };
    let physical = |time: f32| {
        let body = body(time);
        let orbit = time * 1.3;
        let eye = body.position + Vec3::new(-orbit.cos() * 6.0, orbit.sin() * 6.0, 1.8);
        PhysicsFrame {
            body,
            custom: Some(custom_sample(Lens::look_at(eye, body.position + Vec3::Z, 60.0, 0.3, 16.0 / 9.0))),
            avoidables: vec![],
            vehicles: vec![],
            noise: NoiseControls::default(),
            chopper_positions: vec![],
            impacts: vec![],
        }
    };
    let mut camera = Timeline::default();
    camera.place(physical(0.0));
    for i in 1..=181 {
        camera.physics_step(Some(physical(i as f32 / 60.0)));
    }
    for _ in 0..fps * 2 {
        camera.advance(
            Frame {
                real_dt: 1.0 / fps as f32,
                sim_dt: scale / fps as f32,
                view: None,
                custom: true,
                aspect: 16.0 / 9.0,
                regions: None,
                speedbreaker: scale < 1.0,
                look_back: false,
            },
            |dt| dt * scale,
            |a, b| {
                let t = (-2.0 - a.x) / (b.x - a.x);
                (0.0..=1.0).contains(&t).then(|| CameraHit { point: a.lerp(b, t), normal: Vec3::X })
            },
            |_, _, _| Some(0.0),
        );
    }
    let before = camera.sample(body(2.0 * scale), true).unwrap();
    for _ in 0..fps {
        assert_eq!(camera.sample(body(2.0 * scale), true).unwrap(), before);
    }
    (camera.sample(body(2.0 * scale), false).unwrap(), camera.ticks())
}

#[test]
fn retained_custom_clearance_replays_identically_at_low_high_and_slow_motion_fps() {
    for scale in [1.0, 0.25] {
        let (expected, ticks) = replay(60, scale);
        assert_eq!(ticks, 120);
        for fps in [5, 30, 144, 360] {
            let (actual, ticks) = replay(fps, scale);
            assert_eq!(ticks, 120);
            assert!((actual.eye - expected.eye).length() < 0.0001, "fps {fps} scale {scale}");
            assert!((actual.target - expected.target).length() < 0.0001);
            assert_eq!(actual.fov_bangle, expected.fov_bangle);
        }
    }
}

#[test]
fn breaker_uses_gradual_bounded_optics_and_preserves_free_orbit() {
    use super::super::BodyInput;
    let mut effect = Breaker::default();
    let mut body = BodyInput {
        position: Vec3::ZERO,
        forward: Vec3::X,
        up: Vec3::Z,
        velocity: Vec3::ZERO,
        acceleration: Vec3::ZERO,
        speed: 40.0,
        top_speed: 70.0,
        grounded: true,
        nos_engaged: false,
        collision_damping: 0.0,
        drift: 0.0,
        up_influence: Vec3::ZERO,
        zoom: 1.0,
        look_back: false,
        gear: 3,
    };
    let raw = custom_sample(lens(Vec3::new(-6.0, 2.0, 2.0), 60.0));
    let initial = effect.apply(raw, body);
    assert_eq!(initial.eye, raw.eye);
    assert_eq!(initial.fov_bangle, raw.fov_bangle);
    body.zoom = 0.6;
    let first = effect.apply(raw, body);
    assert_eq!(first.eye, raw.eye, "new target must not snap on activation");
    let mut previous = first;
    for _ in 0..240 {
        let held = effect.apply(raw, body);
        assert!((held.eye - previous.eye).length() < 0.08, "abrupt movement");
        assert_eq!(held.eye.z, raw.eye.z);
        assert!((held.target - held.eye - (raw.target - raw.eye)).length() < 1e-5);
        assert!(held.eye.truncate().length() <= raw.eye.truncate().length() * 1.25001);
        assert!(held.fov_bangle >= raw.fov_bangle * 0.8);
        previous = held;
    }
    assert!((previous.fov_bangle / raw.fov_bangle - 0.8).abs() < 0.001);
    assert!((previous.eye.x / raw.eye.x - 1.25).abs() < 0.001);
    body.zoom = 1.0;
    assert!((effect.apply(raw, body).eye - previous.eye).length() < 0.001, "release starts from retained state");
    for _ in 0..240 {
        previous = effect.apply(raw, body);
    }
    assert!((previous.eye - raw.eye).length() < 0.001);
    assert!((previous.fov_bangle / raw.fov_bangle - 1.0).abs() < 1e-5);
}
