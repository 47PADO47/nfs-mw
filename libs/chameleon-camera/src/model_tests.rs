//! The behaviour model on the Porsche's synthetic data: follow distance, speed effects, springs, determinism and
//! robustness to bad input. No game data.

use crate::chase::ChaseCamera;
use crate::math::{MPS_TO_MPH, dot, length, wrap_angle};
use crate::motion::{CameraOptions, CameraPose, CameraView, CarMotion, MAX_DT};
use crate::mounted::{MountedRig, RearViewRig};
use crate::read_tests::porsche_cameras;
use crate::state::CameraState;

const DT: f32 = 1.0 / 60.0;

fn driving(speed: f32, position_x: f32) -> CarMotion {
    CarMotion { position: [position_x, 0.0, 0.0], velocity: [speed, 0.0, 0.0], speed, dt: DT, ..CarMotion::at_rest() }
}

/// Run `frames` frames of a car going straight at `speed`, `tweak` adjusting the input each frame.
fn cruise(state: &mut CameraState, speed: f32, frames: usize, tweak: impl Fn(&mut CarMotion)) -> CameraPose {
    let mut car = driving(speed, 0.0);
    let mut pose = state.reset(&car);
    for _ in 0..frames {
        car.position[0] += speed * DT;
        tweak(&mut car);
        pose = state.step(&car);
    }
    pose
}

fn relative(pose: &CameraPose, car_x: f32) -> [f32; 3] {
    [pose.position[0] - car_x, pose.position[1], pose.position[2]]
}

fn assert_close(a: [f32; 3], b: [f32; 3], tolerance: f32) {
    assert!(a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= tolerance), "{a:?} vs {b:?}");
}

fn assert_sound(pose: &CameraPose) {
    let all = [pose.position, pose.forward, pose.up];
    assert!(all.iter().flatten().all(|v| v.is_finite()), "{pose:?}");
    assert!(pose.vertical_fov_deg.is_finite() && pose.shake.is_finite(), "{pose:?}");
    assert!((length(pose.forward) - 1.0).abs() < 1e-3 && (length(pose.up) - 1.0).abs() < 1e-3, "{pose:?}");
    assert!(dot(pose.forward, pose.up).abs() < 1e-3, "forward and up are not perpendicular: {pose:?}");
}

#[test]
fn at_rest_the_camera_sits_at_the_data_offset() {
    let mut state = CameraState::new(&porsche_cameras(), &CameraOptions::default());
    let pose = state.reset(&CarMotion::at_rest());
    assert_close(pose.position, [-4.6, 0.0, 1.6], 1e-5);
    assert!(pose.forward[0] > 0.9 && pose.forward[2] < 0.0, "looks ahead and slightly down at the focus: {pose:?}");
    assert_close(pose.up, [0.0, 0.0, 1.0], 0.1);
    assert_sound(&pose);
    let rest = state.step(&CarMotion::at_rest());
    assert_close(rest.position, pose.position, 1e-4);
}

#[test]
fn steady_state_follow_distance_at_speed() {
    let mut state = CameraState::new(&porsche_cameras(), &CameraOptions::default());
    let speed = 40.0;
    let pose = cruise(&mut state, speed, 600, |_| {});
    let car_x = speed * DT * 600.0;
    // 40 m/s = 89.5 mph of the 180 mph full speed: 0.497 of the way. Back by 1 m * 0.497 more, 1.6 -> 1.59 m high.
    let share = speed * MPS_TO_MPH / 180.0;
    let expected = [-4.6 - share, 0.0, 1.6 + (1.59 - 1.6) * share];
    assert_close(relative(&pose, car_x), expected, 2e-3);
    assert_sound(&pose);
}

#[test]
fn speed_pulls_the_camera_back_and_widens_the_view() {
    let cameras = porsche_cameras();
    let options = CameraOptions::default();
    let slow = cruise(&mut CameraState::new(&cameras, &options), 5.0, 300, |_| {});
    let fast = cruise(&mut CameraState::new(&cameras, &options), 70.0, 300, |_| {});
    let slow_back = -relative(&slow, 5.0 * DT * 300.0)[0];
    let fast_back = -relative(&fast, 70.0 * DT * 300.0)[0];
    assert!(fast_back > slow_back + 0.5, "{slow_back} vs {fast_back}");
    assert!(
        fast.vertical_fov_deg > slow.vertical_fov_deg + 5.0,
        "{} vs {}",
        slow.vertical_fov_deg,
        fast.vertical_fov_deg
    );
    assert!(fast.shake > slow.shake);
    // 70 horizontal degrees at 16:9 is 43.0 vertical; 95 is 63.7.
    let rest = CameraState::new(&cameras, &options).reset(&CarMotion::at_rest());
    assert!(
        (rest.vertical_fov_deg - 2.0 * (35.0f32.to_radians().tan() / (16.0 / 9.0)).atan().to_degrees()).abs() < 1e-3
    );
}

#[test]
fn the_aspect_option_changes_the_vertical_field_of_view() {
    let cameras = porsche_cameras();
    let wide = CameraState::new(&cameras, &CameraOptions { aspect: 21.0 / 9.0, ..CameraOptions::default() })
        .reset(&CarMotion::at_rest());
    let narrow = CameraState::new(&cameras, &CameraOptions { aspect: 4.0 / 3.0, ..CameraOptions::default() })
        .reset(&CarMotion::at_rest());
    assert!(narrow.vertical_fov_deg > wide.vertical_fov_deg);
}

#[test]
fn braking_tips_the_view_down_and_steering_rolls_it_into_the_turn() {
    let cameras = porsche_cameras();
    let options = CameraOptions::default();
    let plain = cruise(&mut CameraState::new(&cameras, &options), 30.0, 240, |_| {});
    let braking = cruise(&mut CameraState::new(&cameras, &options), 30.0, 240, |car| car.brake = 1.0);
    let dip = plain.forward[2] - braking.forward[2];
    assert!(dip > 0.05 && dip < 0.1, "4.5 degrees of dip, got {dip}");
    let turning = cruise(&mut CameraState::new(&cameras, &options), 30.0, 240, |car| car.steer = 1.0);
    assert!((turning.up[1] - 4.7f32.to_radians().sin()).abs() < 0.01, "roll to the left: {:?}", turning.up);
    let right = cruise(&mut CameraState::new(&cameras, &options), 30.0, 240, |car| car.steer = -1.0);
    assert!(right.up[1] < -0.07, "{:?}", right.up);
    assert!(plain.up[1].abs() < 0.01);
}

#[test]
fn acceleration_drops_the_camera_back() {
    let cameras = porsche_cameras();
    let options = CameraOptions::default();
    let mut state = CameraState::new(&cameras, &options);
    let mut car = driving(10.0, 0.0);
    state.reset(&car);
    let mut back = 0.0f32;
    for _ in 0..120 {
        car.speed += 6.0 * DT; // 6 m/s^2
        car.position[0] += car.speed * DT;
        car.velocity[0] = car.speed;
        let pose = state.step(&car);
        back = back.max(-(pose.position[0] - car.position[0]));
    }
    let cruise_back = 4.6 + car.speed * MPS_TO_MPH / 180.0;
    // 6 m/s^2 = 13.4 mph/s of the 50 that count as full: 0.27 * 1.5 m further back.
    assert!(back > cruise_back + 0.25, "{back} vs {cruise_back}");
}

#[test]
fn the_heading_spring_converges_without_overshoot() {
    let mut camera = ChaseCamera::new(&porsche_cameras(), &CameraOptions::default());
    camera.reset(&CarMotion::at_rest());
    let turned = CarMotion { forward: [0.0, 1.0, 0.0], ..CarMotion::at_rest() };
    let target = std::f32::consts::FRAC_PI_2;
    let mut last = 0.0;
    for _ in 0..30 {
        camera.step(&turned);
        let heading = camera.heading();
        assert!(heading >= last - 1e-6 && heading <= target + 1e-4, "{last} -> {heading}");
        last = heading;
    }
    assert!(last > 0.5 && last < target, "still on its way after half a second: {last}");
    for _ in 0..180 {
        camera.step(&turned);
    }
    assert!((camera.heading() - target).abs() < 1e-3, "{}", camera.heading());
}

#[test]
fn the_heading_spring_takes_the_short_way_round() {
    let mut camera = ChaseCamera::new(&porsche_cameras(), &CameraOptions::default());
    let at = |degrees: f32| {
        let radians = degrees.to_radians();
        CarMotion { forward: [radians.cos(), radians.sin(), 0.0], ..CarMotion::at_rest() }
    };
    camera.reset(&at(170.0));
    for _ in 0..90 {
        camera.step(&at(-170.0));
        let from_start = wrap_angle(camera.heading() - 170.0f32.to_radians());
        assert!(from_start >= -1e-4 && from_start <= 20.0f32.to_radians() + 1e-4, "{from_start}");
    }
    assert!(wrap_angle(camera.heading() + 170.0f32.to_radians()).abs() < 1e-3);
}

#[test]
fn an_impact_shakes_the_view_and_dies_away() {
    let cameras = porsche_cameras();
    let options = CameraOptions::default();
    let mut shaken = CameraState::new(&cameras, &options);
    let mut calm = CameraState::new(&cameras, &options);
    let mut car = CarMotion::at_rest();
    shaken.reset(&car);
    calm.reset(&car);
    let mut deviation = 0.0f32;
    for frame in 0..90 {
        car.impact = if frame == 0 { 1.0 } else { 0.0 };
        let (a, b) = (shaken.step(&car), calm.step(&CarMotion { impact: 0.0, ..car }));
        deviation = deviation.max(1.0 - dot(a.forward, b.forward)).max(1.0 - dot(a.up, b.up));
        if frame == 89 {
            assert_close(a.forward, b.forward, 1e-5);
            assert_close(a.up, b.up, 1e-5);
        }
    }
    assert!(deviation > 1e-6, "the shake moved the camera: {deviation}");
}

#[test]
fn the_same_input_gives_the_same_poses() {
    let cameras = porsche_cameras();
    let options = CameraOptions::default();
    let run = || {
        let mut state = CameraState::new(&cameras, &options);
        let mut car = driving(25.0, 0.0);
        let mut poses = vec![state.reset(&car)];
        for frame in 0..200 {
            car.position[0] += 25.0 * DT;
            car.speed = 25.0 + (frame as f32 * 0.1).sin() * 10.0;
            car.steer = (frame as f32 * 0.05).sin();
            car.brake = if frame % 50 > 40 { 1.0 } else { 0.0 };
            car.impact = if frame == 100 { 0.6 } else { 0.0 };
            poses.push(state.step(&car));
        }
        poses
    };
    assert_eq!(run(), run());
}

#[test]
fn bad_input_never_produces_nan() {
    let cameras = porsche_cameras();
    let options = CameraOptions::default();
    let nan = f32::NAN;
    let cases = [
        CarMotion { dt: 0.0, ..CarMotion::at_rest() },
        CarMotion { dt: -1.0, ..CarMotion::at_rest() },
        CarMotion { dt: nan, ..CarMotion::at_rest() },
        CarMotion { dt: f32::INFINITY, speed: 50.0, ..CarMotion::at_rest() },
        CarMotion { forward: [0.0; 3], ..CarMotion::at_rest() },
        CarMotion { forward: [0.0, 0.0, 1.0], up: [1.0, 0.0, 0.0], ..CarMotion::at_rest() },
        CarMotion { up: [0.0; 3], ..CarMotion::at_rest() },
        CarMotion {
            forward: [nan, 0.0, 0.0],
            position: [nan; 3],
            velocity: [f32::INFINITY; 3],
            ..CarMotion::at_rest()
        },
        CarMotion {
            speed: nan,
            steer: nan,
            throttle: nan,
            brake: nan,
            yaw_rate: nan,
            impact: nan,
            ..CarMotion::at_rest()
        },
        CarMotion { speed: 1.0e9, steer: 50.0, brake: 9.0, yaw_rate: 1.0e9, ..CarMotion::at_rest() },
        CarMotion { grounded: false, speed: 30.0, velocity: [0.0; 3], ..CarMotion::at_rest() },
        CarMotion { grounded: false, speed: 30.0, velocity: [0.0, 0.0, 30.0], ..CarMotion::at_rest() },
    ];
    for view in CameraView::ALL {
        for case in &cases {
            let mut state = CameraState::new(&cameras, &options);
            state.set_view(view);
            let pose = state.reset(case);
            assert_sound(&pose);
            for _ in 0..5 {
                assert_sound(&state.step(case));
            }
            // and a normal frame afterwards still works
            assert_sound(&state.step(&driving(10.0, 3.0)));
        }
    }
    // a camera that is stepped without a reset starts itself
    let mut fresh = ChaseCamera::new(&cameras, &options);
    assert_sound(&fresh.step(&CarMotion::at_rest()));
}

#[test]
fn a_long_frame_is_cut_to_the_largest_step() {
    let cameras = porsche_cameras();
    let options = CameraOptions::default();
    let run = |dt: f32| {
        let mut state = CameraState::new(&cameras, &options);
        let mut car = driving(30.0, 0.0);
        state.reset(&car);
        car.steer = 1.0;
        car.brake = 1.0;
        car.dt = dt;
        (0..10).map(|_| state.step(&car)).last().unwrap()
    };
    assert_eq!(run(5.0), run(MAX_DT));
    assert_ne!(run(DT), run(MAX_DT));
}

#[test]
fn the_bonnet_view_rides_in_the_car_frame() {
    let cameras = porsche_cameras();
    let options = CameraOptions::default();
    let mut rig = MountedRig::new(&cameras.bonnet, [0.0; 3], 180.0, &options);
    let pose = rig.reset(&CarMotion::at_rest());
    assert_close(pose.position, [0.207693, 0.0, 1.31785], 1e-5);
    assert!(pose.forward[0] > 0.9 && pose.forward[2] < 0.0);
    assert_sound(&pose);
    // the car turned a quarter left at (10, 5, 0): the eye's forward offset now points along +y
    let turned = CarMotion { position: [10.0, 5.0, 0.0], forward: [0.0, 1.0, 0.0], ..CarMotion::at_rest() };
    let pose = rig.reset(&turned);
    assert_close(pose.position, [10.0, 5.207693, 1.31785], 1e-4);
    assert!(pose.forward[1] > 0.9);
    // an anchor moves the camera with the car's frame
    let mut anchored = MountedRig::new(&cameras.bonnet, [0.0, 0.0, 0.1], 180.0, &options);
    assert_close(anchored.reset(&CarMotion::at_rest()).position, [0.207693, 0.0, 1.41785], 1e-5);
    // faster is wider
    let at_rest = rig.reset(&CarMotion::at_rest());
    let fast = rig.step(&driving(60.0, 0.0));
    assert!(fast.vertical_fov_deg > at_rest.vertical_fov_deg);
}

#[test]
fn the_wind_buffets_more_with_speed() {
    let cameras = porsche_cameras();
    let rig = MountedRig::new(&cameras.bonnet, [0.0; 3], 180.0, &CameraOptions::default());
    assert_eq!(rig.wind_shake(0.0), 0.0);
    let (slow, fast, full) = (rig.wind_shake(10.0), rig.wind_shake(40.0), rig.wind_shake(180.0 / MPS_TO_MPH));
    assert!(slow < fast && fast < full, "{slow} {fast} {full}");
    assert!((full - 0.25).abs() < 1e-4, "the data's strength at full speed: {full}");
    assert_eq!(rig.wind_shake(1.0e6), 0.25, "capped at the data's strength");
}

#[test]
fn the_bumper_view_looks_along_the_car() {
    let cameras = porsche_cameras();
    let mut rig = MountedRig::new(&cameras.bumper, [0.0; 3], 180.0, &CameraOptions::default());
    let pose = rig.reset(&CarMotion::at_rest());
    assert_close(pose.position, [-1.5, 0.0, 0.6], 1e-5);
    assert!(pose.forward[0] > 0.99);
    assert_sound(&pose);
}

#[test]
fn the_rear_view_looks_back_from_the_tail() {
    let cameras = porsche_cameras();
    let rig = RearViewRig::new(&cameras.rear_view, &CameraOptions::default());
    let pose = rig.pose(&CarMotion::at_rest());
    assert_close(pose.position, [-2.2, 0.0, 0.9], 1e-5);
    assert_close(pose.forward, [-1.0, 0.0, 0.0], 1e-6);
    assert_close(pose.up, [0.0, 0.0, 1.0], 1e-6);
    // 60 degrees horizontal on a 3.33:1 mirror
    let expected = 2.0 * (30.0f32.to_radians().tan() / 3.33333).atan().to_degrees();
    assert!((pose.vertical_fov_deg - expected).abs() < 1e-2);
}

#[test]
fn the_state_switches_views_and_cycles() {
    let cameras = porsche_cameras();
    let mut state = CameraState::new(&cameras, &CameraOptions::default());
    assert_eq!(state.view(), CameraView::Chase);
    let car = CarMotion::at_rest();
    let chase = state.reset(&car);
    let seen: Vec<CameraView> = (0..4).map(|_| state.next_view()).collect();
    assert_eq!(seen, [CameraView::Bonnet, CameraView::Bumper, CameraView::RearView, CameraView::Chase]);
    let again = state.step(&car);
    assert_close(again.position, chase.position, 1e-4);
    state.set_view(CameraView::Bonnet);
    let bonnet = state.step(&car);
    assert!(length(crate::math::sub(bonnet.position, chase.position)) > 3.0);
}
