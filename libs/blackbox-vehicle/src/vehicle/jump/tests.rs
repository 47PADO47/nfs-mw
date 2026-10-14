use super::*;

fn fixture_config() -> JumpConfig {
    JumpConfig {
        extra_airborne_acceleration: 3.0,
        landing: LandingConfig {
            acceleration: 10.0,
            minimum_time: 1.0,
            minimum_peak_altitude: 2.0,
            maximum_altitude: 6.0,
            minimum_alignment: 0.96,
            maximum_speed: 20.0,
        },
        passive: PassiveConfig {
            minimum_up: 0.8,
            maximum_grounded_wheels: 2,
            maximum_speed: 5.0,
            damping: &[(0.0, 0.0), (0.4, 0.15), (2.0, 5.0)],
        },
        alignment: AlignmentConfig {
            minimum_ground_up: 0.9,
            minimum_body_up: 0.1,
            minimum_alignment: 0.8,
            minimum_altitude: 1e-6,
            maximum_altitude: 15.0,
            maximum_speed: 5.0,
            correction_threshold: 0.99,
            pitch: 40.0,
            roll: 20.0,
        },
    }
}

fn input() -> JumpInput {
    JumpInput {
        dt: 1.0 / 60.0,
        mass: 1000.0,
        inertia: Vec3::new(1000.0, 2000.0, 3000.0),
        rotation: Mat3::IDENTITY,
        linear_velocity: Vec3::new(0.0, 0.0, 20.0),
        angular_velocity: Vec3::ZERO,
        speed: 20.0,
        grounded_wheels: 0,
        body_contact: false,
        ground: Some(GroundReference { normal: Vec3::Y, separation: -2.0 }),
    }
}

fn close(actual: Vec3, wanted: Vec3) {
    assert!((actual - wanted).abs().max_element() < 0.003, "actual {actual:?}, expected {wanted:?}");
}

#[test]
fn matches_eight_original_force_consumer_vectors() {
    // Original machine-code replay with synthetic plane/body fixtures; see provenance.
    let c = fixture_config();
    let mut state = JumpState::default();
    let result = state.update(&c, &input());
    close(result.force, Vec3::new(0.0, -3000.0, 0.0));
    close(result.torque, Vec3::ZERO);
    assert!((state.airborne_time - 1.0 / 60.0).abs() < 1e-6);
    assert_eq!(state.peak_altitude, 2.0);

    let mut state = JumpState { airborne_time: 1.1, peak_altitude: 4.0 };
    let falling = JumpInput {
        linear_velocity: Vec3::new(0.0, -1.0, 20.0),
        ground: Some(GroundReference { normal: Vec3::Y, separation: -3.0 }),
        ..input()
    };
    close(state.update(&c, &falling).force, Vec3::new(0.0, -8000.0, 0.0));
    close(state.update(&c, &JumpInput { grounded_wheels: 1, ..input() }).force, Vec3::ZERO);
    assert_eq!(state.airborne_time, 0.0);
    assert_eq!(state.peak_altitude, 0.0);

    let angular = JumpInput { angular_velocity: Vec3::new(0.4, 1.0, 0.5), ..input() };
    close(state.update(&c, &angular).torque, Vec3::new(-1014.7639, 0.0, -3805.3647));
    close(state.update(&c, &JumpInput { body_contact: true, ..angular }).torque, Vec3::ZERO);

    let pitch = JumpInput { rotation: Mat3::from_rotation_x(10.0_f32.to_radians()), ..input() };
    close(state.update(&c, &pitch).torque, Vec3::new(-526.665_34, 0.0, 0.0));
    let roll = JumpInput { rotation: Mat3::from_rotation_z(10.0_f32.to_radians()), ..input() };
    close(state.update(&c, &roll).torque, Vec3::new(0.0, 0.0, -263.332_67));
    let high = JumpInput { ground: Some(GroundReference { normal: Vec3::Y, separation: -16.0 }), ..pitch };
    close(state.update(&c, &high).torque, Vec3::ZERO);
}

#[test]
fn neutral_config_has_no_effect_and_missing_plane_skips_terrain_alignment() {
    let mut state = JumpState::default();
    assert_eq!(state.update(&JumpConfig::default(), &input()), JumpForces::default());
    let unknown = JumpInput { ground: None, rotation: Mat3::from_rotation_x(0.2), ..input() };
    let result = state.update(&fixture_config(), &unknown);
    close(result.force, Vec3::new(0.0, -3000.0, 0.0));
    close(result.torque, Vec3::ZERO);
}
