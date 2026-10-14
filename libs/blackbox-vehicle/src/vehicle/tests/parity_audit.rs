//! Regression cases from the original driving pipeline audit.

use super::*;

#[test]
fn maneuver_downforce_tracks_body_up_and_tire_drag_restores() {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.body.orientation = Quat::from_rotation_z(0.4);
    car.maneuver.extra_downward_acceleration = 19.6256;
    car.maneuver.tire_steering_drag = Some(1.0);
    car.apply_aero();
    let expected = -car.body.rotation().y_axis * car.spec.mass * 19.6256;
    assert!((car.body.pending_force() - expected).length() < 0.01);
    car.begin_tires(20.0);
    assert!(car.tires.iter().all(|t| t.drag_reduction == 1.0));
    car.maneuver = Default::default();
    car.begin_tires(20.0);
    assert!(car.tires.iter().all(|t| t.drag_reduction == 0.15));
}

#[test]
fn tire_velocity_and_drift_use_snapshot_before_wall_correction() {
    let mut base = parked();
    base.set_tire_integration(crate::tires::TireIntegration::Reference);
    base.body.linear_velocity = Vec3::new(10.0, 0.0, 20.0);
    base.body.angular_velocity = Vec3::Y * -0.1;
    base.body_slip = 0.6;
    base.controls.gas = 1.0;
    base.controls.steering = 0.5;
    base.wheels_on_ground_prev = 4;
    let mut wall = base.clone();
    wall.wall_steering_config.yaw_gain = 1.0;
    wall.wall_steering_config.maximum_normal_y = 0.1;
    wall.wall_steering_config.maximum_closing_speed_squared = 56.25;
    wall.wall_steering_config.maximum_nose_speed_squared = 6.25;
    wall.wall_steering_config.end_fraction = 0.75;
    wall.notify_wall_contact(Vec3::Z * 2.0, Vec3::new(0.6, 0.0, -0.8), Vec3::Z, Vec3::Z);
    let snapshot = base.body.angular_velocity;
    wall.apply_wall_steering();
    assert_ne!(wall.body.angular_velocity, snapshot);
    for _ in 0..12 {
        base.update_drift(FIXED_STEP, 30.0, snapshot);
        wall.update_drift(FIXED_STEP, 30.0, snapshot);
    }
    assert_eq!(base.drift_strength(), wall.drift_strength());
    assert_eq!(base.tires[2].drift_friction, wall.tires[2].drift_friction);
    base.begin_tires(30.0);
    wall.begin_tires(30.0);
    let a = base.wheel_forces(FIXED_STEP, &flat(), snapshot);
    let b = wall.wheel_forces(FIXED_STEP, &flat(), snapshot);
    assert_eq!(a.force, b.force);
    assert_eq!(a.torque, b.torque);
}

#[test]
fn reference_feedback_keeps_road_rate_with_zero_force_but_positive_compression() {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.set_tire_integration(crate::tires::TireIntegration::Reference);
    car.corners[0].compression = 0.01;
    car.tires[0].load = 0.0;
    car.tires[0].traction = 1.0;
    car.tires[0].road_speed = 10.0;
    car.tires[0].av = 123.0;
    assert_eq!(car.wheel(0).angular_velocity, 10.0 / car.params[0].radius);
    car.corners[0].compression = 0.0;
    assert_eq!(car.wheel(0).angular_velocity, 123.0);
}

#[test]
fn brake_locked_driven_wheel_keeps_traction_control_grip_multiplier() {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.set_tire_integration(crate::tires::TireIntegration::Reference);
    car.spec.transmission.differential = [0.0; 3];
    car.body.linear_velocity = Vec3::Z * 10.0;
    car.controls.steering = 1.0;
    for tire in &mut car.tires[2..] {
        tire.slip = 30.0;
        tire.lateral_speed = 10.0;
        tire.brake_locked = true;
    }
    car.distribute_drive(1000.0, [0.0; 4], [true; 4]);
    assert_eq!(car.tires[2].drive_torque, 0.0);
    assert_eq!(car.tires[2].traction_boost, 2.0);
}

#[test]
fn collision_mass_override_replaces_material_mass_and_scales_inertia() {
    let mut car = Vehicle::new(VehicleSpec::example());
    let mass = car.body.mass();
    let inertia = car.body.inertia();
    car.maneuver.collision_mass_scale = Some(2.0);
    for material_scale in [0.5, 1.0, 3.0] {
        let mut hit = crate::rigid_body::BodyHit {
            mass_scale: material_scale,
            inertia_scale: Vec3::new(2.0, 3.0, 4.0),
            ..Default::default()
        };
        car.modify_object_hit(&mut hit);
        assert_eq!(hit.mass_scale, 2.0);
        assert_eq!(hit.inertia_scale, Vec3::new(2.0, 3.0, 4.0) * (2.0 / material_scale));
        hit.immobile = true;
        hit.mass_scale = 0.5;
        car.modify_object_hit(&mut hit);
        assert_eq!(hit.mass_scale, 0.5);
    }
    assert_eq!(car.body.mass(), mass);
    assert_eq!(car.body.inertia(), inertia);
}

#[test]
fn landing_updates_sleep_snapshot_but_wall_correction_does_not_change_its_gate() {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.body.linear_velocity = Vec3::Z * 0.1;
    car.body.angular_velocity = Vec3::Y * 0.7;
    car.wheels_on_ground_prev = 0;
    car.wheels_on_ground = 4;
    let snapshot = car.damp_landing(Vec3::Y * 0.4);
    assert_eq!(snapshot, Vec3::Y * 0.2);
    assert_eq!(car.body.angular_velocity, snapshot);
    car.controls.brake = 1.0;
    car.damp_creep(false, snapshot);
    assert!((car.body.angular_velocity.y - 0.02).abs() < 1e-6);
    car.body.linear_velocity = Vec3::Z * 0.1;
    car.body.angular_velocity = Vec3::Y * 0.1;
    car.damp_creep(false, Vec3::Y * 0.4);
    assert_eq!(car.body.linear_velocity.z, 0.1);
}
