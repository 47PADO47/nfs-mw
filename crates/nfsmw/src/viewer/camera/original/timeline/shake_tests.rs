use super::*;

fn sample() -> Sample {
    Sample { eye: Vec3::new(-5.0, 0.0, 2.0), target: Vec3::Z, up: Vec3::Z, fov_bangle: 16000.0 }
}

fn physical(speed: f32, custom: bool) -> PhysicsFrame {
    PhysicsFrame {
        body: BodyInput {
            position: Vec3::ZERO,
            forward: Vec3::X,
            up: Vec3::Z,
            velocity: Vec3::X * speed,
            speed,
            top_speed: 100.0,
            grounded: true,
            acceleration: Vec3::ZERO,
            nos_engaged: false,
            collision_damping: 0.0,
            drift: 0.0,
            up_influence: Vec3::ZERO,
            zoom: 1.0,
            look_back: false,
            gear: 2,
        },
        custom: custom.then(sample),
        avoidables: vec![],
        vehicles: vec![],
        noise: NoiseControls::default(),
        chopper_positions: vec![],
        explosions: vec![],
        impacts: vec![],
    }
}

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

fn advance(camera: &mut Timeline, seconds: f32, mode: usize, custom: bool) {
    camera.advance(
        Frame {
            real_dt: seconds,
            sim_dt: seconds,
            view: (!custom).then_some((mode, &view())),
            custom,
            aspect: 16.0 / 9.0,
            regions: None,
            speedbreaker: false,
            look_back: false,
        },
        |dt| dt,
        |_, _| None,
        |_, _, _| None,
    );
}

#[test]
fn native_inside_and_outside_views_admit_anchor_shake_but_custom_is_not_doubled() {
    for custom in [false, true] {
        for mode in 0..7 {
            let mut camera = Timeline::default();
            camera.place(physical(30.0, custom));
            camera.physics_step(Some(physical(20.0, custom)));
            advance(&mut camera, 2.0 / 60.0, mode, custom);
            let shaken = camera.shake.apply(sample());
            assert_eq!(shaken == sample(), custom, "mode {mode}, custom={custom}");
            assert_eq!(shaken.fov_bangle, sample().fov_bangle);
            if custom {
                continue;
            }
            let current = camera.current.unwrap();
            let expected = camera.shake.apply(sample());
            assert_ne!(expected.up, sample().up, "native shake includes rotation");
            assert!(current.sample.eye.is_finite());
            let pulse = camera.shake.apply(sample());
            advance(&mut camera, 0.0, mode, custom);
            assert_eq!(camera.shake.apply(sample()), pulse, "paused cameras retain the pulse");
            camera.place(physical(20.0, false));
            assert_eq!(camera.shake.apply(sample()), sample(), "placement clears old impact shake");
        }
    }
}

#[test]
fn light_prop_sized_velocity_change_does_not_receive_a_heavy_collision_pulse() {
    let mut camera = Timeline::default();
    camera.place(physical(30.0, false));
    camera.physics_step(Some(physical(29.0, false)));
    advance(&mut camera, 2.0 / 60.0, 3, false);
    assert_eq!(camera.shake.apply(sample()), sample());
}

#[test]
fn native_explosion_camera_snapshots_reach_the_stationary_camera_eye() {
    let mut input = physical(0.0, false);
    input.explosions.push(super::super::Explosion {
        position: Vec3::ZERO,
        expansion_speed: 10.0,
        maximum_radius: 20.0,
    });
    let mut camera = Timeline::default();
    camera.place(input);
    advance(&mut camera, 2.0 / 60.0, 3, false);
    assert_ne!(camera.shake.apply(sample()), sample(), "explosions shake without a car velocity change");
}

#[test]
fn native_impact_shake_replay_agrees_across_display_rates() {
    let replay = |fps: u32| {
        let mut camera = Timeline::default();
        camera.place(physical(30.0, false));
        for i in 1..=60 {
            camera.physics_step(Some(physical(
                match i < 15 {
                    true => 30.0,
                    false => 20.0,
                },
                false,
            )));
        }
        for _ in 0..fps * 2 / 5 {
            advance(&mut camera, 1.0 / fps as f32, 3, false);
        }
        camera.sample(physical(20.0, false).body, false).unwrap()
    };
    let expected = replay(60);
    for fps in [5, 30, 60, 120, 240, 360] {
        let shown = replay(fps);
        assert!((shown.eye - expected.eye).length() < 1e-4, "{fps} FPS eye");
        assert!((shown.target - expected.target).length() < 1e-4, "{fps} FPS target");
        assert!((shown.up - expected.up).length() < 1e-4, "{fps} FPS up");
    }
}
