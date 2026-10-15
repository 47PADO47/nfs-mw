use glam::{Quat, Vec3};
use nfsmw_data::camera::View;

use super::{BodyInput, Frame, NoiseControls, PhysicsFrame, Timeline};

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

fn body(time: f32) -> BodyInput {
    let yaw = time * 0.3;
    BodyInput {
        position: Vec3::new(time * 12.0, time * time, 3.0),
        forward: Quat::from_rotation_z(yaw) * Vec3::X,
        up: Vec3::Z,
        velocity: Vec3::new(12.0, time * 2.0, 0.0),
        acceleration: Vec3::ZERO,
        speed: 12.0,
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

fn physical(time: f32) -> PhysicsFrame {
    PhysicsFrame {
        body: body(time),
        custom: None,
        avoidables: vec![],
        vehicles: vec![],
        noise: NoiseControls::default(),
        chopper_positions: vec![],
        impacts: vec![],
    }
}

fn advance(camera: &mut Timeline, dt: f32, scale: f32, mode: usize, speedbreaker: bool, back: bool) {
    camera.advance(
        Frame {
            custom: false,
            aspect: 16.0 / 9.0,
            regions: None,
            real_dt: dt,
            sim_dt: dt * scale,
            view: Some((mode, &view())),
            speedbreaker,
            look_back: back,
        },
        |offset| offset * scale,
        |_, _| None,
        |_, _, _| None,
    );
}

fn replay(fps: u32, scale: f32, breaker: bool) -> (super::Sample, u64) {
    let mut camera = Timeline::default();
    camera.place(physical(0.0));
    for i in 1..=180 {
        camera.physics_step(Some(physical(i as f32 / 60.0)));
    }
    for _ in 0..fps * 2 {
        advance(&mut camera, 1.0 / fps as f32, scale, 3, breaker, false);
    }
    (camera.sample(body(2.0 * scale), false).unwrap(), camera.ticks())
}

#[test]
fn fixed_camera_replay_matches_at_5_30_60_144_and_360_fps() {
    for (scale, breaker) in [(1.0, false), (0.25, true)] {
        let reference = replay(60, scale, breaker);
        assert_eq!(reference.1, 120);
        for fps in [5, 30, 144, 360] {
            let (actual, ticks) = replay(fps, scale, breaker);
            assert_eq!(ticks, 120, "fps {fps}");
            assert!((actual.eye - reference.0.eye).length() < 0.0001, "fps {fps}: {actual:?}");
            assert!((actual.target - reference.0.target).length() < 0.0001, "fps {fps}");
            assert_eq!(actual.fov_bangle, reference.0.fov_bangle);
        }
    }
}

#[test]
fn long_frame_keeps_camera_ticks_and_pause_does_not_advance_them() {
    let mut camera = Timeline::default();
    camera.place(physical(0.0));
    advance(&mut camera, 5.0, 1.0, 3, false, false);
    assert_eq!(camera.ticks(), 300);
    let before = camera.sample(body(0.0), true).unwrap();
    for invalid in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        advance(&mut camera, invalid, 1.0, 3, false, false);
        assert_eq!(camera.sample(body(0.0), true).unwrap(), before);
        assert_eq!(camera.ticks(), 300);
    }
}

#[test]
fn inside_offsets_follow_rendered_body_pitch_roll_and_translation() {
    let mut camera = Timeline::default();
    camera.place(physical(0.0));
    advance(&mut camera, 1.0 / 60.0, 1.0, 1, false, false);
    let initial = body(0.0);
    let old = camera.sample(initial, true).unwrap();
    let rotation = Quat::from_euler(glam::EulerRot::ZYX, 0.7, 0.3, 0.2);
    let moved = BodyInput {
        position: Vec3::new(80.0, 20.0, 12.0),
        forward: rotation * Vec3::X,
        up: rotation * Vec3::Z,
        ..initial
    };
    let shown = camera.sample(moved, true).unwrap();
    assert!((shown.eye - moved.position - rotation * (old.eye - initial.position)).length() < 1e-5);
    assert!((shown.target - moved.position - rotation * (old.target - initial.position)).length() < 1e-5);
    assert!((shown.up - rotation * old.up).length() < 1e-5);
}

#[test]
fn reset_and_lookback_do_not_blend_through_the_car() {
    let mut camera = Timeline::default();
    camera.place(physical(0.0));
    advance(&mut camera, 1.0 / 60.0, 1.0, 3, false, false);
    let before = camera.sample(body(0.0), false).unwrap();
    advance(&mut camera, 1.0 / 60.0, 1.0, 3, false, true);
    let backward = camera.sample(body(0.0), true).unwrap();
    assert!(before.eye.x < 0.0 && backward.eye.x > 0.0);
    camera.place(physical(10.0));
    assert!(camera.sample(body(10.0), true).is_none());
    advance(&mut camera, 1.0 / 60.0, 1.0, 3, false, false);
    let placed = camera.sample(body(10.0), true).unwrap();
    assert!(placed.eye.distance(body(10.0).position) < 10.0);
}

#[test]
fn slowed_physics_contact_is_consumed_once_and_decays_on_real_time() {
    let mut camera = Timeline::default();
    camera.place(physical(0.0));
    let mut collision = physical(1.0 / 60.0);
    collision.impacts.push((super::ImpactKind::World, 45.0));
    camera.physics_step(Some(collision));
    // The contact is read on real tick four. The same physics pose is held
    // through tick seven, but its contact must not refresh the timer each tick.
    advance(&mut camera, 7.0 / 60.0, 0.25, 3, false, false);
    assert!((camera.collision_damping() - (1.0 - 4.0 / 120.0)).abs() < 1e-6);
    advance(&mut camera, 1.0, 0.25, 3, false, false);
    assert!((camera.collision_damping() - (0.5 - 4.0 / 120.0)).abs() < 1e-6);
}

#[test]
fn speedbreaker_entry_and_exit_have_the_same_camera_response_at_every_fps() {
    let replay = |fps| {
        let mut camera = Timeline::default();
        camera.place(physical(0.0));
        for i in 1..=180 {
            camera.physics_step(Some(physical(i as f32 / 60.0)));
        }
        for (scale, active) in [(1.0, false), (0.25, true), (1.0, false)] {
            for _ in 0..fps {
                advance(&mut camera, 1.0 / fps as f32, scale, 3, active, false);
            }
        }
        (camera.sample(body(2.25), false).unwrap(), camera.ticks())
    };
    let reference = replay(60);
    for fps in [5, 30, 144, 360] {
        let (sample, ticks) = replay(fps);
        assert_eq!(ticks, reference.1);
        assert!((sample.eye - reference.0.eye).length() < 1e-4, "fps {fps}");
        assert_eq!(sample.fov_bangle, reference.0.fov_bangle);
    }
}

#[test]
fn quarter_speed_interpolates_camera_inputs_without_acceleration_spikes() {
    let mut camera = Timeline::default();
    let input = |i: u32| {
        let mut input = physical(i as f32 / 60.0);
        input.body.velocity = Vec3::X * (10.0 + i as f32);
        input.body.speed = 10.0 + i as f32;
        input.body.forward = Vec3::X;
        input
    };
    camera.place(input(0));
    camera.physics_step(Some(input(1)));
    camera.physics_step(Some(input(2)));
    for tick in 1..=8 {
        advance(&mut camera, 1.0 / 60.0, 0.25, 3, true, false);
        if tick > 4 {
            assert!((camera.acceleration().x - 15.0).abs() < 0.001, "tick {tick}");
        }
    }
}

#[test]
fn under_vehicle_pulse_keeps_static_floor_but_suppresses_body_raise() {
    use super::{Avoidable, VehicleBounds};
    use glam::Mat4;
    let mut input = physical(0.0);
    let actor = Avoidable {
        transform: Mat4::from_rotation_translation(
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            input.body.position,
        ),
        half_extents: Vec3::new(2.0, 1.0, 0.7),
        velocity: Vec3::ZERO,
    };
    input.vehicles.push(VehicleBounds { world_id: 2, body: actor, radius: 3.0, active: true, loading: false });
    let mut camera = Timeline::default();
    camera.place(input);
    let mut floors = 0;
    camera.advance(
        Frame {
            real_dt: 1.0 / 60.0,
            sim_dt: 1.0 / 60.0,
            view: Some((3, &view())),
            custom: false,
            aspect: 16.0 / 9.0,
            regions: None,
            speedbreaker: false,
            look_back: false,
        },
        |dt| dt,
        |_, _| None,
        |_, _, _| {
            floors += 1;
            Some(8.0)
        },
    );
    assert_eq!(floors, 1);
    assert!(camera.sample(body(0.0), false).unwrap().eye.z >= 8.25);
}

#[test]
fn helicopter_noise_uses_camera_horizontal_distance_and_is_suppressed_under_cover() {
    use blackbox_roads::{TrackZone, TrackZones};
    use glam::Vec2;
    let zones = TrackZones {
        zones: vec![TrackZone {
            kind: 3,
            elevation: 100.0,
            position: Vec2::ZERO,
            direction: Vec2::X,
            visit_info: 0,
            data: [0; 4],
            bbox_min: Vec2::splat(-100.0),
            bbox_max: Vec2::splat(100.0),
            polygon: vec![
                Vec2::new(-100.0, -100.0),
                Vec2::new(100.0, -100.0),
                Vec2::splat(100.0),
                Vec2::new(-100.0, 100.0),
            ],
        }],
    };
    let replay = |height: Option<f32>, covered: bool| {
        let mut input = physical(0.0);
        input.body.velocity = Vec3::ZERO;
        input.body.speed = 0.0;
        if let Some(height) = height {
            input.chopper_positions.push(Vec3::new(0.0, 3.0, height));
        }
        let mut camera = Timeline::default();
        camera.place(input);
        camera.advance(
            Frame {
                real_dt: 0.2,
                sim_dt: 0.0,
                view: Some((3, &view())),
                custom: false,
                aspect: 16.0 / 9.0,
                regions: covered.then_some(&zones),
                speedbreaker: false,
                look_back: false,
            },
            |_| 0.0,
            |_, _| None,
            |_, _, _| None,
        );
        camera.sample(body(0.0), false).unwrap()
    };
    let low = replay(Some(0.0), false);
    assert_eq!(low, replay(Some(10000.0), false));
    assert_ne!(low, replay(None, false));
    assert_eq!(replay(Some(0.0), true), replay(None, true));
    assert_eq!(replay(None, false), replay(None, true));
}

#[test]
fn custom_breaker_entry_exit_and_pause_match_at_low_and_high_frame_rates() {
    let replay = |fps: u32| {
        let mut input = physical(0.0);
        input.custom = Some(super::custom_sample(crate::viewer::camera::Lens::look_at(
            input.body.position + Vec3::new(-6.0, 2.0, 2.0),
            input.body.position + Vec3::Z,
            60.0,
            0.3,
            16.0 / 9.0,
        )));
        let mut camera = Timeline::default();
        camera.place(input);
        let mut keys = Vec::new();
        for (active, scale) in [(false, 1.0), (true, 0.25), (false, 1.0)] {
            for _ in 0..fps {
                camera.advance(
                    Frame {
                        real_dt: 1.0 / fps as f32,
                        sim_dt: scale / fps as f32,
                        view: None,
                        custom: true,
                        aspect: 16.0 / 9.0,
                        regions: None,
                        speedbreaker: active,
                        look_back: false,
                    },
                    |dt| dt * scale,
                    |_, _| None,
                    |_, _, _| None,
                );
            }
            let before = camera.sample(body(0.0), false).unwrap();
            camera.advance(
                Frame {
                    real_dt: 0.0,
                    sim_dt: 0.0,
                    view: None,
                    custom: true,
                    aspect: 16.0 / 9.0,
                    regions: None,
                    speedbreaker: active,
                    look_back: false,
                },
                |_| 0.0,
                |_, _| panic!("pause must not query"),
                |_, _, _| None,
            );
            assert_eq!(camera.sample(body(0.0), false).unwrap(), before);
            keys.push(before);
        }
        assert!(keys[1].fov_bangle < keys[0].fov_bangle);
        assert!(keys[2].fov_bangle > keys[1].fov_bangle);
        assert_eq!(camera.ticks(), 180);
        keys
    };
    let reference = replay(60);
    for fps in [5, 30, 144, 240, 360] {
        for (a, b) in replay(fps).iter().zip(&reference) {
            assert!((a.eye - b.eye).length() < 1e-5, "fps {fps}");
            assert!((a.fov_bangle - b.fov_bangle).abs() < 1e-5, "fps {fps}");
        }
    }
}
