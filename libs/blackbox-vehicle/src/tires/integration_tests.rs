use super::*;
use crate::ground::SurfaceGrip;

fn params() -> TireParams {
    TireParams {
        radius: 0.33,
        grip_scale: 1.0,
        static_grip: 10.0,
        dynamic_grip: 1.0,
        brake_spec: 0.0,
        brake_lock_spec: 0.0,
        ebrake_spec: 0.0,
        front: true,
    }
}

fn input(lat: f32) -> LoadedInput {
    LoadedInput {
        lat_vel: lat,
        fwd_vel: 20.0,
        body_speed: 20.0,
        load: 2000.0,
        dt: crate::FIXED_STEP,
        quarter_mass: 500.0,
        surface: SurfaceGrip::DEFAULT,
    }
}

#[test]
fn reference_sliding_force_uses_kinetic_friction_without_gripping_branch_blend() {
    let mut tire = Tire {
        integration: TireIntegration::Reference,
        load: 2000.0,
        av: 22.0 / 0.33,
        road_speed: 20.0,
        traction: 0.5,
        drive_torque: 3000.0,
        drag_reduction: 0.0,
        ..Tire::default()
    };
    tire.update_loaded(&params(), &input(2.0));
    let expected = 2000.0 / 2.0_f32.sqrt();
    assert!((tire.longitudinal_force - expected).abs() < 0.01);
    assert!((tire.lateral_force + expected).abs() < 0.01);
}

#[test]
fn reference_gripping_wheel_retains_slip_while_following_road_acceleration() {
    let mut tire = Tire {
        integration: TireIntegration::Reference,
        load: 2000.0,
        av: 20.1 / 0.33,
        road_speed: 20.0,
        ..Tire::default()
    };
    let av = tire.av;
    tire.update_loaded(&params(), &input(0.0));
    assert_eq!(tire.traction, 1.0);
    assert!((tire.av - av).abs() < 0.001);
    let mut regular = Tire { load: 2000.0, av, road_speed: 20.0, ..Tire::default() };
    regular.update_loaded(&params(), &input(0.0));
    assert!((regular.av - 20.0 / 0.33).abs() < 0.001);
}

#[test]
fn reference_sliding_limit_preserves_free_coast_and_force_opposing_drive_torque() {
    for torque in [0.0, 100.0, -100.0] {
        let mut tire = Tire {
            integration: TireIntegration::Reference,
            load: 2000.0,
            av: 22.0 / 0.33,
            road_speed: 20.0,
            traction: 0.5,
            drive_torque: torque,
            ..Tire::default()
        };
        tire.update_loaded(&params(), &input(0.0));
        let expected = match torque > 0.0 {
            true => torque / 0.33,
            false => 2000.0,
        };
        assert!((tire.longitudinal_force - expected).abs() < 0.01, "torque{torque}: {}", tire.longitudinal_force);
    }
}

#[test]
fn reference_sliding_limit_matches_six_isolated_original_pc_vectors() {
    let params = TireParams { radius: 0.35, static_grip: 10.0, dynamic_grip: 0.9, ..params() };
    let input = LoadedInput { fwd_vel: 10.0, body_speed: 10.0, load: 1000.0, ..input(0.0) };
    for (av, torque, force) in [
        (0.0, 0.0, -900.0),
        (0.0, 100.0, -900.0),
        (0.0, -100.0, -285.7143),
        (60.0, 0.0, 900.0),
        (60.0, 100.0, 285.7143),
        (60.0, -100.0, 900.0),
    ] {
        let mut tire = Tire {
            integration: TireIntegration::Reference,
            load: 1000.0,
            av,
            road_speed: 10.0,
            traction: 0.5,
            drive_torque: torque,
            ..Tire::default()
        };
        tire.update_loaded(&params, &input);
        assert!(
            (tire.longitudinal_force - force).abs() < 0.003,
            "av{av} torque{torque}:{} !={force}",
            tire.longitudinal_force
        );
    }
}

#[test]
fn reference_wheel_feedback_matches_twelve_original_pc_vectors() {
    for compressed in [false, true] {
        for traction in [0.5, 1.0, 1.01] {
            for locked in [false, true] {
                let tire = Tire {
                    integration: TireIntegration::Reference,
                    load: 0.0,
                    traction,
                    av: 123.0,
                    road_speed: 10.0,
                    brake_locked: locked,
                    ..Tire::default()
                };
                let expected = match (locked, compressed && traction >= 1.0) {
                    (true, _) => 0.0,
                    (_, true) => 40.0,
                    _ => 123.0,
                };
                assert_eq!(tire.display_av(0.25, compressed), expected);
            }
        }
    }
}

#[test]
fn sign_crossing_clamps_even_inside_the_sign_memory_dead_band() {
    for (previous, current) in [(1, -0.000_000_5), (-1, 0.000_000_5)] {
        let mut tire = Tire { last_sign: previous, av: current, ..Default::default() };
        tire.check_sign();
        assert_eq!(tire.av, 0.0);
        assert_eq!(tire.last_sign, 0);
    }
}
