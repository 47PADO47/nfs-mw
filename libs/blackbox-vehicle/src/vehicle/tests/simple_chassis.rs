//! Whole-vehicle selection of support, steering and landing state.

use super::*;
use crate::ground::NoGround;

fn config() -> SimpleChassisConfig {
    SimpleChassisConfig {
        ride_extra: [-8.0, 1.0, 3.0],
        steering_range: [35.0, 55.0],
        landing_angular_retention: Vec3::new(1.0, 0.4, 1.0),
    }
}

#[test]
fn simple_chassis_cog_uses_previous_support_without_moving_the_origin_or_changing_mass() {
    let mut heights = Vec::new();
    for count in [0, 1, 2, 4] {
        let mut vehicle = Vehicle::new(VehicleSpec::example());
        vehicle.set_simple_chassis(Some(config()));
        vehicle.wheels_on_ground = count;
        let mass = vehicle.body.mass();
        let inertia = vehicle.body.inertia();
        vehicle.step(FIXED_STEP, &InputState::default(), &NoGround);
        heights.push(vehicle.body.cog().y);
        assert_eq!(vehicle.body.position, Vec3::ZERO);
        assert_eq!(vehicle.body.mass(), mass);
        assert_eq!(vehicle.body.inertia(), inertia);
        if count == 0 {
            let midpoint = vehicle.spec.chassis.front_axle - vehicle.spec.chassis.wheel_base * 0.5;
            assert!((vehicle.body.cog().z - midpoint).abs() < 1e-6);
        }
    }
    assert!((heights[0] - heights[1] - 9.0 * 0.0254).abs() < 1e-6);
    assert!((heights[1] - heights[2] - 2.0 * 0.0254).abs() < 1e-6);
    assert_eq!(heights[2], heights[3]);
}

#[test]
fn simple_chassis_steering_changes_with_pedals_and_reports_unscaled_range() {
    let mut vehicle = Vehicle::new(VehicleSpec::example());
    vehicle.spec.tires.steering = 0.8;
    vehicle.set_simple_chassis(Some(config()));
    assert_eq!(vehicle.maximum_steering_degrees(), 35.0);
    for (gas, brake, handbrake, expected) in [
        (1.0, 0.0, 0.0, 35.0),
        (0.5, 0.0, 0.0, 45.0),
        (1.0, 1.0, 0.0, 55.0),
        (1.0, 0.0, 1.0, 55.0),
        (0.0, 0.0, 0.0, 55.0),
    ] {
        vehicle.controls = crate::input::Controls { gas, brake, handbrake, steering: 1.0, ..Default::default() };
        vehicle.update_steering(FIXED_STEP, 20.0);
        assert_eq!(vehicle.maximum_steering_degrees(), expected);
        assert!((vehicle.wheel_angles.right.to_degrees() - expected * 0.8).abs() < 1e-5);
    }
}

#[test]
fn simple_chassis_first_landing_preserves_roll_and_pitch_while_damping_yaw_once() {
    let mut vehicle = Vehicle::new(VehicleSpec::example());
    vehicle.set_simple_chassis(Some(config()));
    vehicle.body.place(Vec3::ZERO, Quat::from_rotation_y(0.4));
    let local = Vec3::new(0.7, 1.0, 1.3);
    let incoming = vehicle.body.rotation() * local;
    vehicle.body.angular_velocity = incoming;
    vehicle.wheels_on_ground_prev = 0;
    vehicle.wheels_on_ground = 1;
    let landed = vehicle.damp_landing(incoming);
    assert!((vehicle.body.rotation().transpose() * landed - Vec3::new(0.7, 0.4, 1.3)).length() < 1e-6);
    vehicle.wheels_on_ground_prev = 1;
    assert_eq!(vehicle.damp_landing(landed), landed);
    vehicle.set_simple_chassis(None);
    vehicle.wheels_on_ground_prev = 0;
    let player = vehicle.damp_landing(incoming);
    assert!((vehicle.body.rotation().transpose() * player - Vec3::new(0.7, 0.5, 0.65)).length() < 1e-6);
}
