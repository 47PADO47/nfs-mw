use super::*;
use crate::ground::SurfaceGrip;
use crate::math::FT_LB_TO_NM;

fn params() -> TireParams {
    TireParams {
        radius: 0.35,
        grip_scale: 1.0,
        static_grip: 1.2,
        dynamic_grip: 1.1,
        brake_spec: 100.0 * FT_LB_TO_NM * 4.0,
        brake_lock_spec: 100.0,
        ebrake_spec: 150.0 * FT_LB_TO_NM * 10.0,
        front: false,
    }
}

fn response(curve: bool) -> RollingTireResponse {
    match curve {
        false => RollingTireResponse::Linear { gain: 2.0, corner_drag: 0.5 },
        true => RollingTireResponse::Curve {
            samples: [0.0, 0.4, 0.8, 1.2, 1.6, 1.75, 1.65],
            range: 1.0 / 3.0,
            rear_scale: [1.5, 0.75],
            corner_drag: 0.5,
        },
    }
}

#[test]
fn rolling_tires_match_isolated_original_straight_corner_reverse_and_braking_vectors() {
    // Independent isolated original routines; input surface multipliers are deliberately absent from these models.
    // Curve outputs allow the original's discrete angle lookup rather than widening any contact tolerance.
    let cases = [
        (false, 0.0, 20.0, 1.0, 0.0, 4000.0, 0.0, 0.0, -1549.4857),
        (false, 2.0, 20.0, 1.0, 0.0, 4000.0, 0.0, -8000.0, -1151.6929),
        (false, 0.3, 15.0, 0.3, 0.0, 4000.0, 0.0, -2400.0, -440.91724),
        (false, 0.1, 0.4, 1.0, 0.0, 4000.0, 0.0, -80.0, -1549.4857),
        (false, -2.0, -10.0, 0.5, 0.0, 4000.0, 0.0, 8000.0, 1558.8651),
        (false, 1.0, 20.0, 1.0, 0.0, 12000.0, 0.0, -24000.0, -951.48096),
        (false, 0.5, 15.0, 0.0, 0.0, 4000.0, 500.0, -4000.0, 1495.0956),
        (true, 0.0, 20.0, 1.0, 0.0, 4000.0, 0.0, 0.0, -1549.4857),
        (true, 2.0, 20.0, 1.0, 0.0, 4000.0, 0.0, -684.8877, -1515.4303),
        (true, 0.3, 15.0, 0.3, 0.0, 4000.0, 0.0, -137.10938, -463.47876),
        (true, 0.1, 0.4, 1.0, 0.0, 4000.0, 0.0, -168.42041, -1549.4857),
        (true, -2.0, -10.0, 0.5, 0.0, 4000.0, 0.0, 1356.5918, 907.7096),
        (true, 2.0, 20.0, 0.0, 1.0, 4000.0, 0.0, -258.8632, -4379.5073),
        (true, 1.0, 20.0, 1.0, 0.0, 12000.0, 0.0, -1028.3203, -1523.8632),
        (true, 0.5, 15.0, 0.0, 0.0, 4000.0, 500.0, -228.73535, 1432.3755),
    ];
    let params = params();
    for (curve, lat, fwd, brake, ebrake, load, torque, lateral, longitudinal) in cases {
        let mut tire = Tire { load, av: fwd / params.radius, brake, ebrake, drive_torque: torque, ..Tire::default() };
        let input = LoadedInput {
            lat_vel: lat,
            fwd_vel: fwd,
            body_speed: fwd.abs(),
            load,
            dt: 1.0 / 60.0,
            quarter_mass: 500.0,
            surface: SurfaceGrip::DEFAULT,
        };
        tire.update_rolling(&params, &input, response(curve));
        assert!((tire.lateral_force - lateral).abs() < 1.0, "{curve} {lat} {fwd}: {} != {lateral}", tire.lateral_force);
        assert!(
            (tire.longitudinal_force - longitudinal).abs() < 1.0,
            "{curve} {lat} {fwd}: {} != {longitudinal}",
            tire.longitudinal_force
        );
        assert_eq!(tire.brake_locked, ebrake > 0.0, "regular brakes must not use the racing lock test");
        if ebrake == 0.0 {
            assert_eq!(tire.av, fwd / params.radius, "ordinary braking retains rolling wheel speed");
        }
    }
}

#[test]
fn rolling_free_tires_coast_or_stop_without_racing_wheel_torque_integration() {
    for (brake, ebrake, expected) in [(0.0, 0.0, 20.0), (1.0, 0.0, 0.0), (0.0, 0.1, 0.0)] {
        let mut tire = Tire { av: 20.0, brake, ebrake, drive_torque: 1000.0, load: 4000.0, ..Default::default() };
        tire.update_rolling_free();
        assert_eq!(tire.av, expected);
        assert_eq!((tire.load, tire.slip, tire.lateral_force, tire.longitudinal_force), (0.0, 0.0, 0.0, 0.0));
    }
}
