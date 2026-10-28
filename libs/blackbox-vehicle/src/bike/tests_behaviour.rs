//! The behaviour of the final model on flat ground: mass and inertia, the rider, the prescribed lean, the suspension with
//! the tyre in series, the engine and the rider help. These tests were written without running them.

use glam::{Quat, Vec3};

use super::{Bike, BikeInput, BikeSpec, FRONT, REAR};
use crate::FIXED_STEP;
use crate::ground::FlatGround;

fn bike_at(speed: f32) -> (Bike, FlatGround) {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(BikeSpec::example());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    (bike, ground)
}

fn run(bike: &mut Bike, ground: &FlatGround, seconds: f32, input: &BikeInput) {
    for _ in 0..(seconds / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, input, ground);
    }
}

#[test]
fn the_mass_is_the_bike_and_the_rider_and_the_inertia_is_the_radius_of_gyration_times_it() {
    let spec = BikeSpec::example();
    let bike = Bike::new(spec.clone());
    let total = spec.chassis.mass + spec.chassis.rider_mass;
    assert_eq!(spec.total_mass(), total);
    assert!((bike.body().mass() - total).abs() < 1e-3);
    let wanted = spec.chassis.tensor_scale * total;
    assert!((bike.body().inertia() - wanted).abs().max_element() < 1e-2, "{} {wanted}", bike.body().inertia());
}

#[test]
fn the_combined_centre_of_gravity_is_above_the_bikes_own() {
    let spec = BikeSpec::example();
    let bike = Bike::new(spec.clone());
    assert!(bike.body().cog().y > spec.chassis.cg_height);
    assert!(bike.body().cog().y < spec.chassis.cg_height + spec.chassis.rider_offset.y);
}

#[test]
fn a_standing_bike_sits_on_its_suspension_with_the_tyres_squashed_a_little() {
    let (mut bike, ground) = bike_at(0.0);
    run(&mut bike, &ground, 3.0, &BikeInput::default());
    let spec = bike.spec().clone();
    for (index, wheel_spec) in [(FRONT, &spec.front), (REAR, &spec.rear)] {
        let wheel = bike.wheel(index);
        assert!(wheel.on_ground);
        assert!(wheel.tyre_compression > 0.0 && wheel.tyre_compression <= wheel_spec.max_tire_deformation + 1e-4);
        assert!(wheel.compression.abs() < 0.08, "{}", wheel.compression);
        assert!(wheel.compression_speed.abs() < 0.2);
    }
}

#[test]
fn shifting_the_weight_moves_the_centre_of_gravity_and_not_the_bike() {
    let (mut bike, ground) = bike_at(0.0);
    run(&mut bike, &ground, 1.0, &BikeInput::default());
    let before = (bike.body().cog(), bike.body().world_cog());
    bike.set_weight(1.0);
    run(&mut bike, &ground, 1.0, &BikeInput::default());
    assert!(bike.body().cog().z > before.0.z + 0.01);
    assert!(!bike.fallen() && bike.lean().abs() < 0.05);
    assert!(bike.body().world_cog().distance(before.1) < 0.1);
    bike.set_weight(f32::NAN);
    run(&mut bike, &ground, 1.0, &BikeInput::default());
    assert!((bike.body().cog().z - before.0.z).abs() < 0.01);
}

#[test]
fn the_lean_follows_the_target_the_stick_asks_for() {
    let (mut bike, ground) = bike_at(20.0);
    let input = BikeInput { throttle: 0.15, steer: 0.5, ..Default::default() };
    run(&mut bike, &ground, 3.0, &input);
    assert!(bike.target_lean() > 0.3, "{}", bike.target_lean());
    assert!((bike.lean() - bike.target_lean()).abs() < 0.05, "{} {}", bike.lean(), bike.target_lean());
    assert!(bike.lean_limit() <= bike.spec().steering.max_lean + 1e-6);
}

#[test]
fn the_front_tyre_pushes_to_the_inside_of_a_held_turn() {
    let (mut bike, ground) = bike_at(20.0);
    run(&mut bike, &ground, 4.0, &BikeInput { throttle: 0.2, steer: 0.5, ..Default::default() });
    // Along the ground's right (the axis the tyre pushes along), not the leaned bike's: the load on a leaned tyre has a
    // component along the bike's own right axis (about -N sin(lean), here about -380 N) that hides the tyre's side force.
    let wheel = bike.wheel(FRONT);
    let sideways = wheel.force.dot(wheel.side);
    assert!(sideways > 100.0, "{sideways}");
}

#[test]
fn full_throttle_from_a_standstill_does_not_flip_the_bike() {
    let (mut bike, ground) = bike_at(0.0);
    let input = BikeInput { throttle: 1.0, ..Default::default() };
    let mut worst: f32 = 0.0;
    for _ in 0..(8.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &input, &ground);
        worst = worst.max(bike.pitch());
    }
    assert!(!bike.fallen(), "pitch {worst}");
    assert!(worst < 0.7, "pitch {worst}");
}

#[test]
fn the_engine_never_runs_far_past_its_rev_limit() {
    let (mut bike, ground) = bike_at(0.0);
    let input = BikeInput { throttle: 1.0, ..Default::default() };
    let mut worst: f32 = 0.0;
    for _ in 0..(15.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &input, &ground);
        worst = worst.max(bike.rpm());
    }
    assert!(worst < bike.spec().engine.max_rpm * 1.1, "{worst}");
}

#[test]
fn braking_hard_does_not_throw_the_rider_over_the_bars() {
    let (mut bike, ground) = bike_at(30.0);
    let input = BikeInput { front_brake: 1.0, rear_brake: 1.0, ..Default::default() };
    let mut worst: f32 = 0.0;
    for _ in 0..(8.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &input, &ground);
        worst = worst.min(bike.pitch());
    }
    assert!(!bike.fallen(), "pitch {worst}");
    assert!(bike.forward_speed() < 1.0, "{}", bike.forward_speed());
}

#[test]
fn the_anti_lock_brake_keeps_the_front_wheel_turning() {
    let (mut bike, ground) = bike_at(25.0);
    run(&mut bike, &ground, 1.5, &BikeInput { front_brake: 1.0, ..Default::default() });
    assert!(bike.wheel(FRONT).slip_ratio > -0.9, "{}", bike.wheel(FRONT).slip_ratio);
}

#[test]
fn coasting_slows_the_bike_down_with_the_engine_the_air_and_the_tyres() {
    let (mut bike, ground) = bike_at(30.0);
    run(&mut bike, &ground, 3.0, &BikeInput::default());
    assert!(bike.forward_speed() < 29.5 && bike.forward_speed() > 15.0, "{}", bike.forward_speed());
}

#[test]
fn weight_back_and_the_throttle_lift_the_front_wheel_to_a_wheelie() {
    let (mut bike, ground) = bike_at(10.0);
    bike.set_weight(-1.0);
    let mut highest: f32 = 0.0;
    for _ in 0..(4.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 0.7, ..Default::default() }, &ground);
        highest = highest.max(bike.pitch());
    }
    assert!(highest > 0.1, "{highest}");
    assert!(!bike.fallen());
}

#[test]
fn a_bike_rolled_over_is_fallen_and_placing_it_again_stands_the_rider_up() {
    let (mut bike, ground) = bike_at(10.0);
    bike.body_mut().set_orientation(Quat::from_rotation_z(1.4));
    bike.step(FIXED_STEP, &BikeInput::default(), &ground);
    assert!(bike.fallen());
    run(&mut bike, &ground, 0.5, &BikeInput { throttle: 1.0, ..Default::default() });
    assert!(bike.fallen());
    assert!(bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
    assert!(!bike.fallen());
}

#[test]
fn saturated_braking_in_a_held_turn_keeps_the_bike_up_and_pointing_where_it_was() {
    let (mut bike, ground) = bike_at(25.0);
    run(&mut bike, &ground, 3.0, &BikeInput { throttle: 0.3, steer: 0.4, ..Default::default() });
    assert!(!bike.fallen());
    let brake = BikeInput { front_brake: 1.0, rear_brake: 1.0, steer: 0.4, ..Default::default() };
    for _ in 0..(3.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &brake, &ground);
        assert!(!bike.fallen(), "fell at speed {}", bike.forward_speed());
    }
    assert!(bike.forward_speed() < 20.0);
}

#[test]
fn saturated_braking_in_a_straight_line_does_not_yaw_the_bike_round() {
    let (mut bike, ground) = bike_at(30.0);
    let brake = BikeInput { front_brake: 1.0, rear_brake: 1.0, ..Default::default() };
    run(&mut bike, &ground, 4.0, &brake);
    let forward = bike.orientation() * Vec3::Z;
    assert!(forward.z > 0.9 && !bike.fallen(), "{forward}");
}

#[test]
fn the_traction_control_keeps_the_rear_wheel_from_spinning_up_on_a_start() {
    let worst_slip = |tcs: bool| {
        let mut spec = BikeSpec::example();
        if !tcs {
            spec.tcs = None;
        }
        let ground = FlatGround::new(0.0);
        let mut bike = Bike::new(spec);
        assert!(bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
        let input = BikeInput { throttle: 1.0, ..Default::default() };
        let mut worst: f32 = 0.0;
        for _ in 0..(3.0 / FIXED_STEP) as usize {
            bike.step(FIXED_STEP, &input, &ground);
            if bike.forward_speed() > 3.0 {
                worst = worst.max(bike.wheel(REAR).slip_ratio);
            }
        }
        worst
    };
    assert!(worst_slip(true) <= worst_slip(false) + 1e-3, "{} {}", worst_slip(true), worst_slip(false));
}

#[test]
fn the_riders_spring_lets_him_lag_a_hard_acceleration_and_come_back() {
    // Without the auto-rider, which moves the weight forward under the throttle and would hide the lag.
    let mut spec = BikeSpec::example();
    spec.wheelie.as_mut().unwrap().help.auto_rider = None;
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 0.0));
    run(&mut bike, &ground, 0.5, &BikeInput::default());
    let rest = bike.body().cog().z;
    run(&mut bike, &ground, 1.0, &BikeInput { throttle: 0.6, ..Default::default() });
    // Pushed back by the acceleration, the combined centre of gravity moves backwards of its place.
    assert!(bike.body().cog().z < rest, "{} {rest}", bike.body().cog().z);
}
