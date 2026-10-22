use super::*;
use crate::{FIXED_STEP, FlatGround, InputState, VehicleSpec};

fn config() -> RollingChassisConfig {
    RollingChassisConfig {
        tires: RollingTireResponse::Linear { gain: 2.0, corner_drag: 0.5 },
        steering_degrees: 45.0,
        force_arm_height: 0.5,
        front_drive_share: 0.0,
        handbrake_front: false,
        cog: None,
        load_from_up: false,
    }
}

#[test]
fn rolling_axle_selection_reaches_engine_coupling_and_direct_wheel_torque() {
    let mut spec = VehicleSpec::example();
    spec.transmission.torque_split = 1.0;
    let mut car = Vehicle::new(spec);
    car.set_rolling_chassis(Some(config()));
    assert_eq!(car.powertrain.transmission_spec().torque_split, 0.0);
    assert_eq!(car.spec.transmission.torque_split, 1.0, "authored source value remains intact");
    car.distribute_drive(1000.0, [5.0, 50.0, 10.0, 20.0], [true; 4]);
    assert_eq!(car.tires.map(|t| t.drive_torque), [0.0, 0.0, 500.0, 500.0]);
    car.set_spec(car.spec().clone());
    assert_eq!(
        car.powertrain.transmission_spec().torque_split,
        0.0,
        "hardware refresh retains normalized axle selection"
    );
    car.set_rolling_chassis(None);
    assert_eq!(car.powertrain.transmission_spec().torque_split, 1.0);
}

#[test]
fn rolling_chassis_controls_keep_fixed_parallel_steering_and_direct_brakes() {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.set_rolling_chassis(Some(config()));
    for gas in [0.0, 0.5, 1.0] {
        car.controls = crate::input::Controls { gas, brake: 0.8, handbrake: 0.4, steering: 0.5, ..Default::default() };
        car.update_steering(FIXED_STEP, 20.0);
        assert_eq!(car.maximum_steering_degrees(), 45.0);
        assert_eq!(car.wheel_angles.left, 22.5_f32.to_radians());
        assert_eq!(car.wheel_angles.right, car.wheel_angles.left);
        car.tune_tires(FIXED_STEP, 20.0, Vec3::ZERO);
        assert_eq!(car.tires.map(|t| t.brake), [0.8; 4]);
        assert_eq!(car.tires.map(|t| t.ebrake), [0.0, 0.0, 0.4, 0.4]);
    }
}

#[test]
fn rolling_trailer_cog_is_independent_of_axle_position_and_does_not_change_stored_mass() {
    let mut car = Vehicle::new(VehicleSpec::example());
    let (mass, inertia) = (car.body.mass(), car.body.inertia());
    let mut selected = config();
    selected.cog = Some(Vec3::new(0.0, -1.0, 0.0));
    car.set_rolling_chassis(Some(selected));
    car.update_chassis_cog();
    assert_eq!(car.body.cog(), Vec3::new(0.0, -1.0, 0.0));
    car.set_rolling_cog(Vec3::ZERO);
    car.update_chassis_cog();
    assert_eq!(car.body.cog(), Vec3::ZERO);
    assert_eq!((car.body.mass(), car.body.inertia()), (mass, inertia));
}

#[test]
fn rolling_ordinary_brakes_reach_the_loaded_tires_without_the_racing_lock_policy() {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.set_rolling_chassis(Some(config()));
    let ground = FlatGround::new(0.0);
    assert!(car.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0));
    for _ in 0..120 {
        car.step(FIXED_STEP, &InputState::default(), &ground);
    }
    car.body.linear_velocity = Vec3::Z * 20.0;
    for (tire, params) in car.tires.iter_mut().zip(car.params) {
        tire.av = 20.0 / params.radius;
    }
    let mut loaded = 0;
    for _ in 0..30 {
        car.step(FIXED_STEP, &InputState { brake: 1.0, ..Default::default() }, &ground);
        for tire in car.tires.iter().filter(|t| t.load > 0.0) {
            loaded += 1;
            assert!(!tire.brake_locked, "normal rolling brake cannot turn into a rear handbrake");
        }
    }
    assert!(loaded > 0, "the production wheel-force path must actually carry load");
}
