//! Independent original-PC loaded-tire numeric observations; no captured bytes.

use super::{LoadedInput, Tire, TireIntegration, TireParams};
use crate::ground::SurfaceGrip;

fn params() -> TireParams {
    TireParams {
        radius: 1.0,
        grip_scale: 1.0,
        static_grip: 0.5,
        dynamic_grip: 1.0,
        brake_spec: 0.0,
        brake_lock_spec: 0.0,
        ebrake_spec: 0.0,
        front: true,
    }
}

#[test]
fn reference_spinning_tire_has_friction_below_the_patch_speed_epsilon() {
    let mut tire = Tire {
        integration: TireIntegration::Reference,
        av: 1.0,
        load: 1024.0,
        traction: 0.5,
        last_sign: 1,
        max_slip: 1.0,
        ..Default::default()
    };
    let input = LoadedInput {
        lat_vel: 0.0,
        fwd_vel: 1.0 / 4_194_304.0,
        body_speed: 1.0,
        load: 1024.0,
        dt: 1.0 / 64.0,
        quarter_mass: 100.0,
        surface: SurfaceGrip::DEFAULT,
    };
    tire.update_loaded(&params(), &input);
    // Exact independently executed original-code outputs for this reduced vector.
    assert_eq!(tire.longitudinal_force, 512.0);
    assert_eq!(tire.last_torque, -256.0);
    assert!(!tire.brake_locked);
}

#[test]
fn reference_brake_lock_clears_previous_wheel_direction_in_both_directions() {
    let params = TireParams { brake_spec: 4096.0, brake_lock_spec: 4096.0, ..params() };
    for direction in [-1, 1] {
        let mut tire = Tire {
            integration: TireIntegration::Reference,
            av: f32::from(direction) * 10.0,
            load: 1024.0,
            traction: 0.5,
            last_sign: direction,
            brake: 1.0,
            ..Default::default()
        };
        let input = LoadedInput {
            lat_vel: 1.0,
            fwd_vel: f32::from(direction) * 10.0,
            body_speed: 10.0,
            load: 1024.0,
            dt: 1.0 / 144.0,
            quarter_mass: 100.0,
            surface: SurfaceGrip::DEFAULT,
        };
        tire.update_loaded(&params, &input);
        assert!(tire.brake_locked);
        assert_eq!(tire.av, 0.0);
        assert_eq!(tire.angular_acc, 0.0);
        assert_eq!(tire.last_sign, 0);
    }
}
