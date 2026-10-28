//! Behaviour of the whole bike on flat ground: it stands, accelerates, brakes, leans into a turn and balances.

use glam::Vec3;

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
fn a_bike_with_nothing_asked_of_it_stands_on_both_wheels() {
    let (mut bike, ground) = bike_at(0.0);
    run(&mut bike, &ground, 4.0, &BikeInput::default());
    assert_eq!(bike.wheels_on_ground(), 2);
    assert!(bike.lean().abs() < 0.03, "lean {}", bike.lean());
    assert!(bike.forward_speed().abs() < 0.2);
    assert!(!bike.fallen());
}

#[test]
fn the_static_load_splits_by_the_weight_balance() {
    let (mut bike, ground) = bike_at(0.0);
    run(&mut bike, &ground, 4.0, &BikeInput::default());
    let total = bike.wheel(FRONT).load + bike.wheel(REAR).load;
    let weight = bike.spec().total_mass() * 9.81;
    assert!((total - weight).abs() / weight < 0.05, "{total} {weight}");
    // The weight balance of the data is the bike's own; with the rider the front share is the spec's combined one.
    let front = bike.wheel(FRONT).load / total;
    assert!((front - bike.spec().front_share()).abs() < 0.05, "{front}");
}

#[test]
fn full_throttle_accelerates_in_a_straight_line_without_wobbling() {
    let (mut bike, ground) = bike_at(0.0);
    let input = BikeInput { throttle: 1.0, ..Default::default() };
    let mut worst: f32 = 0.0;
    for _ in 0..(12.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &input, &ground);
        worst = worst.max(bike.lean().abs());
    }
    assert!(bike.forward_speed() > 30.0, "speed {}", bike.forward_speed());
    assert!(worst < 0.1, "worst lean {worst}");
    // 2 m, not 0.5 m: the lean controller holds the lean, not the heading, so the yaw the launch gives (the rear tyre breaks
    // away for a moment) stays: 0.3 degrees of heading after 12 s, 1.3 m sideways over 270 m. The old balance controller
    // damped it; a rider who wants a line steers for it.
    assert!(bike.position().x.abs() < 2.0, "drifted {}", bike.position().x);
    assert!(bike.gear() > 1);
}

#[test]
fn braking_shifts_the_weight_forwards_and_stops_the_bike() {
    let (mut bike, ground) = bike_at(25.0);
    let weight = bike.spec().total_mass() * 9.81;
    let static_front = bike.spec().front_share() * weight;
    let input = BikeInput { front_brake: 0.7, rear_brake: 0.5, ..Default::default() };
    run(&mut bike, &ground, 1.0, &input);
    assert!(bike.wheel(FRONT).load > static_front * 1.3, "{} {}", bike.wheel(FRONT).load, static_front);
    run(&mut bike, &ground, 6.0, &input);
    assert!(bike.forward_speed() < 1.0, "speed {}", bike.forward_speed());
    assert!(!bike.fallen());
}

#[test]
fn a_stick_to_the_right_countersteers_then_leans_right_and_turns_right() {
    let (mut bike, ground) = bike_at(20.0);
    run(&mut bike, &ground, 1.0, &BikeInput { throttle: 0.15, ..Default::default() });
    let input = BikeInput { throttle: 0.15, steer: 0.5, ..Default::default() };
    let mut first_steer = 0.0;
    for step in 0..(0.25 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &input, &ground);
        if step == 6 {
            first_steer = bike.steer_angle();
        }
    }
    assert!(first_steer < 0.0, "countersteer {first_steer}");
    run(&mut bike, &ground, 4.0, &input);
    assert!(bike.lean() > 0.15, "lean {}", bike.lean());
    assert!(bike.position().x > 5.0, "x {}", bike.position().x);
    assert!(!bike.fallen());
}

#[test]
fn a_steady_turn_leans_by_the_angle_the_speed_and_the_radius_call_for() {
    let (mut bike, ground) = bike_at(18.0);
    let input = BikeInput { throttle: 0.45, steer: 0.4, ..Default::default() };
    run(&mut bike, &ground, 2.0, &input);
    let forward = bike.orientation() * Vec3::Z;
    let yaw_rate = -0.0 + bike_yaw_rate(&mut bike, &ground, &input);
    let speed = bike.velocity().dot(forward);
    let expected = (speed * yaw_rate / 9.81).atan();
    assert!(
        // 0.15, not 0.12: the lean now follows the target through three filters (stick, target, spring-damper), so after
        // 2 s a little of the approach is still left.
        (bike.lean() - expected).abs() < 0.15,
        "lean {} expected {expected} (v {speed}, yaw {yaw_rate})",
        bike.lean()
    );
    assert!(bike.wheel(FRONT).on_ground && bike.wheel(REAR).on_ground);
}

/// The yaw rate over a short stretch of steady cornering (rad/s, positive turning right).
fn bike_yaw_rate(bike: &mut Bike, ground: &FlatGround, input: &BikeInput) -> f32 {
    let heading = |b: &Bike| (b.orientation() * Vec3::Z).x.atan2((b.orientation() * Vec3::Z).z);
    let start = heading(bike);
    run(bike, ground, 0.5, input);
    let mut turned = heading(bike) - start;
    while turned > std::f32::consts::PI {
        turned -= std::f32::consts::TAU;
    }
    while turned < -std::f32::consts::PI {
        turned += std::f32::consts::TAU;
    }
    turned / 0.5
}

#[test]
fn the_turn_is_the_same_mirrored() {
    let (mut right, ground) = bike_at(18.0);
    let (mut left, _) = bike_at(18.0);
    run(&mut right, &ground, 5.0, &BikeInput { throttle: 0.2, steer: 0.4, ..Default::default() });
    run(&mut left, &ground, 5.0, &BikeInput { throttle: 0.2, steer: -0.4, ..Default::default() });
    assert!((right.lean() + left.lean()).abs() < 0.03, "{} {}", right.lean(), left.lean());
    assert!((right.position().x + left.position().x).abs() < 0.5);
}

#[test]
fn a_slow_bike_is_held_up_by_the_rider() {
    let (mut bike, ground) = bike_at(0.0);
    run(&mut bike, &ground, 2.0, &BikeInput { steer: 0.8, ..Default::default() });
    assert!(bike.lean().abs() < 0.2 && !bike.fallen(), "lean {}", bike.lean());
}

#[test]
fn a_bike_flung_off_the_ground_has_no_wheel_contacts() {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(BikeSpec::example());
    assert!(bike.place_on_ground(&ground, 0.0, 0.0, 3.0, 0.0));
    bike.step(FIXED_STEP, &BikeInput::default(), &crate::ground::NoGround);
    assert_eq!(bike.wheels_on_ground(), 0);
}

#[test]
fn a_bike_cannot_be_placed_without_ground() {
    let mut bike = Bike::new(BikeSpec::example());
    assert!(!bike.place_on_ground(&crate::ground::NoGround, 0.0, 0.0, 3.0, 0.0));
}
