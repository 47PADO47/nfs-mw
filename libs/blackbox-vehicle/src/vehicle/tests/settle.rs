use super::*;
use crate::tires::TireIntegration;

fn fixture(integration: TireIntegration) -> Vehicle {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.set_tire_integration(integration);
    car.body.linear_velocity = Vec3::X * 0.25;
    car.body.apply_force(Vec3::new(64.0, 128.0, 256.0));
    car.body.apply_torque(Vec3::new(32.0, 64.0, 128.0));
    car
}

#[test]
fn reference_lateral_response_adds_the_other_force_and_torque_axes_again() {
    let mut car = fixture(TireIntegration::Reference);
    car.damp_creep(false, Vec3::ZERO);
    assert_eq!(car.body.pending_force(), Vec3::new(16.0, 256.0, 512.0));
    assert_eq!(car.body.pending_torque(), Vec3::new(64.0, 16.0, 256.0));
    assert_eq!(car.body.linear_velocity, Vec3::X * 0.0625);
}

#[test]
fn regularized_lateral_response_keeps_its_existing_pending_force_policy() {
    let mut car = fixture(TireIntegration::Regularized);
    car.damp_creep(false, Vec3::ZERO);
    assert_eq!(car.body.pending_force(), Vec3::new(48.0, 128.0, 256.0));
    assert_eq!(car.body.pending_torque(), Vec3::new(32.0, 48.0, 128.0));
}

#[test]
fn reference_full_stop_uses_damping_amount_rounding_and_epsilon_zero() {
    let mut car = fixture(TireIntegration::Reference);
    car.wheels_on_ground = 4;
    car.controls.brake = 1.0;
    car.body.linear_velocity = Vec3::Z * 0.1;
    car.damp_creep(false, Vec3::ZERO);
    assert_eq!(car.body.pending_force(), Vec3::new(6.4000015, 12.800003, 25.600006));
    assert_eq!(car.body.linear_velocity.z, 0.010000003);

    car.body.linear_velocity = Vec3::Z * 0.0000001;
    car.body.angular_velocity = Vec3::X * 0.1;
    car.damp_creep(false, car.body.angular_velocity);
    assert_eq!(car.body.linear_velocity, Vec3::ZERO);
    assert_eq!(car.body.angular_velocity, Vec3::ZERO);
    assert_eq!(car.body.pending_force(), Vec3::ZERO);
    assert_eq!(car.body.pending_torque(), Vec3::ZERO);
}

#[test]
fn collision_rejects_full_stop_but_keeps_reference_lateral_response() {
    let mut car = fixture(TireIntegration::Reference);
    car.wheels_on_ground = 4;
    car.controls.brake = 1.0;
    for tire in &mut car.tires {
        tire.av = 10.0;
    }
    car.damp_creep(true, Vec3::ZERO);
    assert_eq!(car.body.pending_force(), Vec3::new(16.0, 256.0, 512.0));
    assert!(car.tires.iter().all(|t| t.av == 10.0));
}
