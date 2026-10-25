use super::*;

fn physical(time: f32) -> PhysicsFrame {
    let position = Vec3::new(time * 12.0, time * 2.0, 3.0);
    PhysicsFrame {
        body: BodyInput {
            position,
            forward: Vec3::X,
            up: Vec3::Z,
            velocity: Vec3::new(12.0, 2.0, 0.0),
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
        },
        custom: Some(Sample {
            eye: position + Vec3::new(-5.0, 0.0, 2.0),
            target: position + Vec3::X,
            up: Vec3::Z,
            fov_bangle: 10000.0,
        }),
        avoidables: vec![],
        vehicles: vec![],
        noise: NoiseControls::default(),
        chopper_positions: vec![],
        explosions: vec![],
        impacts: vec![],
    }
}

#[test]
fn physical_history_uses_actual_sixty_144_and_576_deltas() {
    for rate in [60, 144, 576] {
        let mut camera = Timeline::default();
        camera.place(physical(0.0));
        let dt = 1.0 / rate as f32;
        for tick in 1..=rate {
            camera.physics_step_with_delta(Some(physical(tick as f32 * dt)), dt);
        }
        let expected = f64::from(dt) * f64::from(rate);
        assert!((camera.physics_time - expected).abs() < 1e-12, "rate {rate}");
        assert!((camera.bodies.back().unwrap().time - expected).abs() < 1e-12);
        let at = expected * 0.5;
        let body = camera.body_at(at).unwrap().body;
        assert!((body.position - physical(at as f32).body.position).length() < 1e-5, "rate {rate}");
    }
}

#[test]
fn missing_snapshots_advance_time_but_invalid_deltas_do_not() {
    let mut camera = Timeline::default();
    camera.place(physical(0.0));
    for dt in [0.0, -0.1, f32::NAN, f32::INFINITY] {
        camera.physics_step_with_delta(Some(physical(100.0)), dt);
    }
    assert_eq!(camera.physics_time, 0.0);
    assert_eq!(camera.bodies.len(), 1);
    camera.physics_step_with_delta(None, 0.01);
    camera.physics_step_with_delta(Some(physical(0.02)), 0.01);
    assert_eq!(camera.bodies.len(), 2);
    assert_eq!(camera.bodies.back().unwrap().time, 2.0 * f64::from(0.01_f32));
    camera.physics_step(None);
    assert_eq!(camera.physics_time, 2.0 * f64::from(0.01_f32) + STEP);
}

fn replay(rate: u32, scale: f32, fps: u32) -> (Sample, u64, f64) {
    let mut camera = Timeline::default();
    camera.place(physical(0.0));
    let physics_dt = scale / rate as f32;
    for tick in 1..=rate * 3 {
        camera.physics_step_with_delta(Some(physical(tick as f32 * physics_dt)), physics_dt);
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
            |offset| offset * scale,
            |_, _| None,
            |_, _, _| None,
        );
    }
    (camera.sample(physical(2.0 * scale).body, false).unwrap(), camera.camera_ticks, camera.physics_time)
}

#[test]
fn actual_physics_deltas_keep_camera_sampling_at_sixty_hz_across_render_grouping() {
    for (rate, scale) in [(60, 1.0), (144, 1.0), (60, 0.25), (144, 0.25)] {
        let (reference, ticks, time) = replay(rate, scale, 60);
        assert_eq!(ticks, 120);
        assert!((time - f64::from(3.0 * scale)).abs() < 2e-7);
        for fps in [5, 30, 144, 360] {
            let (actual, ticks, actual_time) = replay(rate, scale, fps);
            assert_eq!(ticks, 120, "physics {rate}, scale {scale}, render {fps}");
            assert_eq!(actual_time, time);
            assert!((actual.eye - reference.eye).length() < 1e-4);
            assert!((actual.target - reference.target).length() < 1e-4);
            assert!((actual.fov_bangle - reference.fov_bangle).abs() < 1e-3);
        }
    }
}
