use super::*;
use crate::{ground::SurfaceGrip, math::FT_LB_TO_NM};

fn response() -> CoastingTireResponse {
    CoastingTireResponse {
        angle_range: 12.0 / 360.0,
        lateral_gain: 1.54,
        brake_torque_scale: 2.5,
        brake_force_gain: 2.5,
        corner_drag: 0.5,
    }
}
fn params(front: bool) -> TireParams {
    TireParams {
        radius: 0.35,
        grip_scale: 1.0,
        static_grip: 1.2,
        dynamic_grip: 1.1,
        brake_spec: 100.0 * FT_LB_TO_NM * 4.0,
        brake_lock_spec: 100.0,
        ebrake_spec: 150.0 * FT_LB_TO_NM * 10.0,
        front,
    }
}

#[test]
fn coasting_matches_original_no_slip_braking_sideways_reverse_and_surface_vectors() {
    let cases = [
        (0.0, 30.0, 0.25, false, 0.0, 1.0, 1.0, 0.0, -968.42865, 1.0),
        (10.0, 30.0, 0.25, false, 0.0, 1.0, 1.0, -7392.0, 200.01974, 1.0),
        (30.0, 2.0, 0.25, false, 0.0, 1.0, 1.0, -7392.0, 2_719.393, 1.0),
        (-10.0, -20.0, 0.25, false, 0.0, 1.0, 1.0, 7392.0, 2621.3237, 1.0),
        (0.1, 0.4, 1.0, false, 0.0, 1.0, 1.0, -739.2, -13061.226, 1.0),
        (30.0, 2.0, 1.0, false, 0.0, 1.0, 1.0, -7392.0, -185.89284, 1.0),
        (5.0, 20.0, 0.0, false, 0.0, 0.3, 0.4, -1440.0, 232.81456, 0.64935064),
        (5.0, 20.0, 0.0, true, 0.0, 0.3, 0.4, -4800.0, 582.036_4, 0.64935064),
        (5.0, 20.0, 0.25, false, 0.0, 0.3, 0.4, -7392.0, -72.092636, 1.0),
        (2.0, 30.0, 0.25, false, -300.0, 1.0, 1.0, -2348.3494, -1747.5038, 1.0),
        (20.0, 0.0, 0.0, false, 0.0, 1.0, 1.0, -4800.0, 0.0, 0.64935064),
        (0.0, 30.0, 0.0, false, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0),
        (30.0, 2.0, 0.1, false, 0.0, 1.0, 1.0, -5_991.779, 2675.2664, 0.81057614),
    ];
    for (lat, fwd, brake, front, torque, lateral_surface, drive_surface, expected_y, expected_x, traction) in cases {
        let p = params(front);
        let mut tire = Tire {
            load: 4000.0,
            av: fwd / p.radius,
            road_speed: fwd,
            brake,
            drive_torque: torque,
            grip_boost: 1.2,
            ..Default::default()
        };
        let input = LoadedInput {
            lat_vel: lat,
            fwd_vel: fwd,
            body_speed: fwd.abs(),
            load: 4000.0,
            dt: 1.0 / 60.0,
            quarter_mass: 500.0,
            surface: SurfaceGrip { lateral: lateral_surface, drive: drive_surface, ..SurfaceGrip::DEFAULT },
        };
        tire.update_coasting(&p, &input, response());
        assert!(
            (tire.lateral_force - expected_y).abs() < 1.0,
            "lateral {lat}/{fwd}: {} != {expected_y}",
            tire.lateral_force
        );
        assert!(
            (tire.longitudinal_force - expected_x).abs() < 1.0,
            "longitudinal {lat}/{fwd}: {} != {expected_x}",
            tire.longitudinal_force
        );
        assert!((tire.traction - traction).abs() < 0.0001);
        assert_eq!(tire.slip, 0.0);
        assert!(!tire.brake_locked);
        assert_eq!(tire.av, fwd / p.radius);
    }
}

#[test]
fn coasting_original_quarter_brake_and_sideways_grip_are_not_the_racer_response() {
    let p = params(false);
    let input = LoadedInput {
        lat_vel: 30.0,
        fwd_vel: 2.0,
        body_speed: 30.1,
        load: 4000.0,
        dt: 1.0 / 60.0,
        quarter_mass: 500.0,
        surface: SurfaceGrip::DEFAULT,
    };
    let original =
        Tire { load: 4000.0, av: 2.0 / p.radius, road_speed: 2.0, brake: 0.25, grip_boost: 1.2, ..Default::default() };
    let mut racer = original;
    racer.update_loaded(&p, &input);
    let mut coasting = original;
    coasting.update_coasting(&p, &input, response());
    assert!((coasting.lateral_force + 7392.0).abs() < 1.0);
    assert!(
        (racer.lateral_force - coasting.lateral_force).abs() > 2000.0,
        "old racer path must expose the original sideways-force mismatch"
    );
}
