//! Exercise the same fixed-step car and camera timeline used by the world scene.

use glam::Vec3;

use super::{FixedClock, STEP};
use crate::viewer::camera::{ChaseCamera, Followed};

fn follow(camera: &mut ChaseCamera, position: Vec3) {
    camera.update(Followed { position, heading: 0.0, speed: 60.0 }, STEP, |_, _| None);
}

#[test]
fn car_and_camera_keep_their_relative_position_between_ticks() {
    for fps in [5, 30, 60, 120, 144, 240] {
        let mut clock = FixedClock::default();
        let mut camera = ChaseCamera::default();
        let (mut previous, mut current) = (Vec3::ZERO, Vec3::ZERO);
        follow(&mut camera, current);
        let offset = current - camera.position;
        for frame in 0..fps * 3 {
            for _ in 0..clock.advance(1.0 / fps as f32) {
                previous = current;
                current.x += 60.0 * STEP;
                follow(&mut camera, current);
            }
            let car = previous.lerp(current, clock.alpha());
            let view = camera.sample(clock.alpha());
            let error = (car - view.position - offset).length();
            assert!(error < 0.0001, "{fps} FPS, frame {frame}: car jumps {error} m relative to the camera");
        }
    }
}

#[test]
fn sampling_the_camera_does_not_change_fixed_tick_follow_behavior() {
    let run = |fps: u32| {
        let mut clock = FixedClock::default();
        let mut camera = ChaseCamera::default();
        let mut ticks = Vec::new();
        let mut time = 0.0;
        follow(&mut camera, Vec3::ZERO);
        for _ in 0..fps * 2 {
            for _ in 0..clock.advance(1.0 / fps as f32) {
                time += STEP;
                if ticks.len() == 30 || ticks.len() == 67 {
                    camera.impact(Vec3::new(600.0, -300.0, 250.0));
                }
                camera.update(
                    Followed {
                        position: Vec3::new(time * 30.0, time * time, 0.5 * time.sin()),
                        heading: time * 0.3,
                        speed: time * 20.0,
                    },
                    STEP,
                    |_, _| (time > 1.0 && time < 1.2).then_some(0.5),
                );
                ticks.push(camera.view());
            }
            let current = camera.view();
            let _ = camera.sample(clock.alpha()).view_proj(16.0 / 9.0);
            assert_eq!(camera.view(), current, "rendering cannot modify the physics camera");
        }
        ticks
    };
    let reference = run(60);
    for fps in [5, 30, 120, 144, 240] {
        assert_eq!(run(fps), reference, "{fps} FPS changed camera collision/follow timing");
    }
}

#[test]
fn shake_changes_only_the_presented_view_and_a_teleport_clears_it() {
    let mut camera = ChaseCamera::default();
    follow(&mut camera, Vec3::ZERO);
    camera.impact(Vec3::Z * 600.0);
    follow(&mut camera, Vec3::X);
    let physics = camera.view();
    let shaken = camera.sample(1.0);
    assert_ne!(shaken, physics);
    assert!(shaken.view_proj(16.0 / 9.0).is_finite());
    assert_eq!(camera.view(), physics);
    camera.snap();
    follow(&mut camera, Vec3::new(1000.0, 500.0, 10.0));
    assert_eq!(camera.sample(1.0), camera.view());
}

#[test]
fn respawning_snaps_every_render_pose_to_the_new_car() {
    let mut camera = ChaseCamera::default();
    follow(&mut camera, Vec3::ZERO);
    follow(&mut camera, Vec3::X);
    camera.snap();
    follow(&mut camera, Vec3::new(1000.0, 500.0, 10.0));
    for alpha in [0.0, 0.25, 0.5, 1.0] {
        assert_eq!(camera.sample(alpha), camera.view(), "a reset must not interpolate across the map");
    }
}

#[test]
fn camera_moves_on_frames_without_a_physics_tick_and_at_uneven_rates() {
    let frames = [1.0 / 240.0, 1.0 / 120.0, 0.029, 1.0 / 144.0, 0.05, 0.003];
    let mut clock = FixedClock::default();
    let mut camera = ChaseCamera::default();
    let mut car = Vec3::ZERO;
    follow(&mut camera, car);
    let start = camera.position;
    let mut elapsed = 0.0;
    for dt in frames.into_iter().cycle().take(600) {
        elapsed += dt;
        for _ in 0..clock.advance(dt) {
            car.x += 60.0 * STEP;
            follow(&mut camera, car);
        }
        let expected = start.x + ((elapsed - STEP) * 60.0).max(0.0);
        let actual = camera.sample(clock.alpha()).position.x;
        assert!((actual - expected).abs() < 0.002, "camera at {actual}, expected {expected} after {elapsed}s");
    }
}

#[test]
fn contact_shake_presents_the_same_view_at_the_same_elapsed_time() {
    let run = |frames: &[f32]| {
        let mut clock = FixedClock::default();
        let mut camera = ChaseCamera::default();
        let mut baseline = ChaseCamera::default();
        let (mut elapsed, mut tick) = (0.0_f64, 0_u32);
        follow(&mut camera, Vec3::ZERO);
        follow(&mut baseline, Vec3::ZERO);
        for &frame in frames.iter().cycle() {
            let remaining = 1.42 - elapsed;
            if remaining <= 1e-8 {
                break;
            }
            let dt = f64::from(frame).min(remaining) as f32;
            elapsed += f64::from(dt);
            for _ in 0..clock.advance(dt) {
                tick += 1;
                if tick == 60 {
                    camera.impact(Vec3::Z * 600.0);
                }
                let time = tick as f32 * STEP;
                let followed = Followed {
                    position: Vec3::new(time * 30.0, time * time, 0.5 * time.sin()),
                    heading: time * 0.3,
                    speed: time * 20.0,
                };
                camera.update(followed, STEP, |_, _| None);
                baseline.update(followed, STEP, |_, _| None);
            }
            let _ = camera.sample(clock.alpha()).view_proj(16.0 / 9.0);
        }
        let view = camera.sample(clock.alpha()).view_proj(16.0 / 9.0);
        assert!(
            !view.abs_diff_eq(baseline.sample(clock.alpha()).view_proj(16.0 / 9.0), 1e-5),
            "compare an active pulse"
        );
        view
    };
    let reference = run(&[1.0 / 60.0]);
    for fps in [5, 10, 30, 120, 144, 240] {
        assert!(run(&[1.0 / fps as f32]).abs_diff_eq(reference, 0.0005), "{fps} FPS changed the shaken view");
    }
    assert!(run(&[0.003, 0.2, 1.0 / 144.0, 0.1, 1.0 / 240.0]).abs_diff_eq(reference, 0.0005));
}
