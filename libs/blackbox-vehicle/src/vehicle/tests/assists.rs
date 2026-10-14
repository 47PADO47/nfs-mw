use super::*;

#[test]
fn reference_profile_survives_placement_and_hardware_replacement() {
    let mut v = Vehicle::new(VehicleSpec::example());
    v.set_tire_integration(crate::tires::TireIntegration::Reference);
    v.place(Vec3::ZERO, identity());
    v.set_spec(v.spec().clone());
    assert_eq!(v.tire_integration(), crate::tires::TireIntegration::Reference);
    assert!(v.tires.iter().all(|t| t.integration == crate::tires::TireIntegration::Reference));
}

#[test]
fn maneuver_applies_front_grip_and_restores_neutral_on_exit() {
    let mut v = Vehicle::new(VehicleSpec::example());
    v.controls.steering = 0.5;
    v.maneuver.front_grip_gain = 0.75;
    v.begin_tires(20.0);
    let boost = v.tires[0].traction_boost;
    v.tune_tires(FIXED_STEP, 20.0, v.body.angular_velocity);
    assert!((v.tires[0].traction_boost / boost - 1.375).abs() < 1e-5);
    assert_eq!(v.tires[0].lateral_boost, 1.375);
    assert_eq!(v.tires[2].lateral_boost, 1.0);
    v.maneuver = Default::default();
    v.begin_tires(20.0);
    v.tune_tires(FIXED_STEP, 20.0, v.body.angular_velocity);
    assert_eq!(v.tires[0].traction_boost, boost);
    assert_eq!(v.tires[0].lateral_boost, 1.0);
}

#[test]
fn maneuver_scales_aero_and_steering_without_changing_dynamic_mass() {
    let mut base = Vehicle::new(VehicleSpec::example());
    base.body.linear_velocity = Vec3::new(0.0, 0.0, 20.0);
    base.controls.gas = 1.0;
    base.controls.steering = 1.0;
    let mut assist = base.clone();
    assist.maneuver = crate::ManeuverAssist {
        extra_downward_acceleration: 19.62,
        tire_steering_drag: Some(1.0),
        collision_mass_scale: Some(2.0),
        drag_scale: 0.25,
        front_grip_gain: 0.75,
        steering_degrees: Some(60.0),
    };
    base.apply_aero();
    assist.apply_aero();
    let base_force = base.body.pending_force();
    let force = assist.body.pending_force();
    assert!((force.z / base_force.z - 0.25).abs() < 1e-5);
    assert!((force.y - base_force.y + assist.spec.mass * 19.62).abs() < 0.01);
    assert_eq!(assist.body.mass(), base.body.mass());
    assist.update_steering(FIXED_STEP, 20.0);
    assert!((assist.wheel_angles.right.to_degrees() - 60.0).abs() < 0.001);
}

#[test]
fn reference_sleep_damps_net_force_and_preserves_gravity_support() {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.set_tire_integration(crate::tires::TireIntegration::Reference);
    car.wheels_on_ground = 4;
    car.controls.brake = 1.0;
    car.body.linear_velocity = Vec3::new(0.0, 0.0, 0.1);
    car.body.begin_frame(FIXED_STEP);
    let gravity = Vec3::Y * (car.body.spec().gravity * car.body.mass());
    car.body.apply_force(-gravity + Vec3::Z * 1000.0);
    car.damp_creep(false, car.body.angular_velocity);
    let pending = car.body.pending_force();
    assert!(pending.y.abs() < 0.01);
    assert!((pending.z - 100.0).abs() < 0.01);
    assert!((car.body.linear_velocity.z - 0.01).abs() < 1e-6);
    let height = car.body.position.y;
    car.body.begin_frame(FIXED_STEP);
    assert_eq!(car.body.linear_velocity.y, 0.0, "balanced support must not inject upward acceleration");
    assert_eq!(car.body.position.y, height);
}

#[test]
fn cornering_wheelspin_moves_torque_and_grip_to_the_other_side() {
    let mut v = Vehicle::new(VehicleSpec::example());
    v.spec.transmission.differential = [0.0; 3];
    v.body.linear_velocity = Vec3::new(0.0, 0.0, 10.0);
    v.controls.steering = 1.0;
    for tire in &mut v.tires[2..] {
        tire.slip = 30.0;
        tire.lateral_speed = 10.0;
    }
    v.distribute_drive(1000.0, [0.0; 4], [true; 4]);
    assert_eq!(v.tires[2].drive_torque, 250.0);
    assert_eq!(v.tires[3].drive_torque, 0.0);
    assert_eq!(v.tires[2].traction_boost, 2.0);
    assert_eq!(v.tires[3].traction_boost, 1.0);
    // Steering against the slide and engine braking do not trigger the assist.
    for torque in [1000.0, -1000.0] {
        v.controls.steering = -1.0;
        v.begin_tires(10.0);
        v.distribute_drive(torque, [0.0; 4], [true; 4]);
        assert_eq!(v.tires[2].drive_torque, torque * 0.5);
        assert_eq!(v.tires[3].drive_torque, torque * 0.5);
    }
}

#[test]
fn drifting_lowers_rear_friction_and_damps_opposing_yaw_then_recovers() {
    let mut v = Vehicle::new(VehicleSpec::example());
    v.body_slip = 30.0_f32.to_radians();
    v.body.angular_velocity = Vec3::new(0.0, -1.0, 0.0);
    v.controls.gas = 1.0;
    v.wheels_on_ground_prev = 4;
    for _ in 0..12 {
        v.update_drift(FIXED_STEP, 30.0, v.body.angular_velocity);
    }
    assert_eq!(v.drift.value, 1.0);
    assert!(v.tires[2].drift_friction < 1.0);
    assert_eq!(v.tires[0].drift_friction, 1.0);
    v.body.begin_frame(FIXED_STEP);
    assert!(v.body.angular_velocity.y > -1.0, "yaw {}", v.body.angular_velocity.y);
    v.body_slip = 0.0;
    v.controls.gas = 0.0;
    for _ in 0..40 {
        v.begin_tires(0.0);
        v.update_drift(FIXED_STEP, 0.0, v.body.angular_velocity);
    }
    assert_eq!(v.drift.value, 0.0);
    assert_eq!(v.tires[2].drift_friction, 1.0);
    v.place(Vec3::ZERO, identity());
    assert_eq!(v.drift.value, 0.0);
}
