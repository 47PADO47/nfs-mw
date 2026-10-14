use glam::{Quat, Vec3};

use super::*;
use crate::VehicleSpec;

fn config() -> WallSteeringConfig {
    WallSteeringConfig {
        maximum_normal_y: 0.1,
        maximum_closing_speed_squared: 56.25,
        maximum_nose_speed_squared: 6.25,
        end_fraction: 0.75,
        side_epsilon: 1e-6,
        yaw_gain: 0.125,
    }
}

fn car() -> Vehicle {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.wall_steering_config = config();
    car.wheels_on_ground_prev = 4;
    car.controls.steering = 0.5;
    car.controls.gas = 1.0;
    car.body.angular_velocity = Vec3::Y * 0.25;
    car
}

#[test]
fn original_numeric_yaw_vectors_and_side_overwrite() {
    for (nose, side, steer, grounded, reverse, expected) in [
        (-1.0, 0.0, -1.0, 4, false, 0.125),
        (0.4, 0.0, 0.5, 4, false, 0.275),
        (0.4, 0.0, -0.5, 4, false, 0.25),
        (-1.0, 0.0, -1.0, 2, false, 0.25),
        (0.0, 0.6, 0.5, 4, true, 0.2125),
        (0.0, 0.6, 0.5, 4, false, 0.25),
        (0.4, 0.6, 0.5, 4, true, 0.2125),
        (-0.7, 0.6, 0.5, 4, true, 0.25),
    ] {
        let delta = WallSteering { nose, side }.yaw(steer, 1.0, grounded, reverse, 0.125);
        assert!((0.25 + delta - expected).abs() < 1e-6);
    }
}

#[test]
fn original_contact_thresholds_are_strict_and_first_nonzero_value_survives() {
    let mut car = car();
    let point = Vec3::Z * 2.0;
    for (normal, point, closing, velocity) in [
        (Vec3::NEG_Z, point, Vec3::Z * 7.5, Vec3::Z),
        (Vec3::NEG_Z, point, Vec3::Z, Vec3::Z * 2.5),
        (Vec3::new(0.0, 0.1, -0.995), point, Vec3::Z, Vec3::Z),
        (Vec3::NEG_Z, Vec3::Z * (car.spec.dimension.z * 0.75), Vec3::Z, Vec3::Z),
    ] {
        car.notify_wall_contact(point, normal, closing, velocity);
        assert_eq!(car.wall_steering.nose, 0.0);
    }
    car.notify_wall_contact(point, Vec3::NEG_Z, Vec3::Z, Vec3::Z);
    assert_eq!(car.wall_steering.nose, -1.0);
    car.notify_wall_contact(point, Vec3::new(0.6, 0.0, -0.8), Vec3::Z, Vec3::Z);
    assert_eq!(car.wall_steering.nose, -1.0);
    car.wall_steering = Default::default();
    car.notify_wall_contact(point, Vec3::new(0.6, 0.0, 0.8), Vec3::NEG_Z, Vec3::NEG_Z);
    assert_eq!(car.wall_steering.nose, 0.0);
    assert!((car.wall_steering.side - 0.6).abs() < 1e-6);
}

#[test]
fn contact_yaw_uses_body_up_and_is_consumed_once_without_dt_scaling() {
    let mut car = car();
    car.body.orientation = Quat::from_rotation_z(0.3);
    let up = car.body.rotation().y_axis;
    let initial = car.body.angular_velocity;
    car.wall_steering.nose = 0.4;
    car.apply_wall_steering();
    assert!((car.body.angular_velocity - initial - up * 0.025).length() < 1e-6);
    let after = car.body.angular_velocity;
    car.apply_wall_steering();
    assert_eq!(car.body.angular_velocity, after);
}

#[test]
fn neutral_config_disables_contacts_hardware_preserves_pending_and_placement_clears() {
    let mut car = car();
    car.wall_steering_config = Default::default();
    car.notify_wall_contact(Vec3::Z * 2.0, Vec3::NEG_Z, Vec3::Z, Vec3::Z);
    assert_eq!(car.wall_steering.nose, 0.0);
    car.wall_steering_config = config();
    car.notify_wall_contact(Vec3::Z * 2.0, Vec3::NEG_Z, Vec3::Z, Vec3::Z);
    car.set_spec(car.spec().clone());
    assert_eq!(car.wall_steering.nose, -1.0);
    car.place(Vec3::ZERO, Quat::IDENTITY);
    assert_eq!(car.wall_steering.nose, 0.0);
    assert_eq!(car.wall_steering.side, 0.0);
    assert_eq!(car.wall_steering_config, config());
}
