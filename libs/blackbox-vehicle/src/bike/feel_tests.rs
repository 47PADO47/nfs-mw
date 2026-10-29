//! How the bike feels to ride (`docs/specs/ride6-feel-round8.md`): a start that ends, a clutch that does not chatter, a fall limit
//! with margin and a stick that can be held. Run on the example bike, whose numbers are those of a 450 cm3 supermoto.

use super::{Bike, BikeInput, BikeSpec, InputRates, REAR, choices};
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
fn full_throttle_reaches_100_kmh_in_a_few_seconds_and_changes_up_before_the_limiter() {
    let (mut bike, ground) = bike_at(0.0);
    let input = BikeInput { throttle: 1.0, ..Default::default() };
    let (mut reached, mut highest) = (None, 0.0_f32);
    for step in 0..(12.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &input, &ground);
        highest = highest.max(bike.rpm());
        if reached.is_none() && bike.forward_speed() > 100.0 / 3.6 {
            reached = Some(step as f32 * FIXED_STEP);
        }
    }
    let time = reached.expect("100 km/h within 12 s");
    assert!(time < 6.0, "100 km/h took {time} s");
    assert!(bike.gear() >= 4, "gear {}", bike.gear());
    assert!(highest < bike.spec().engine.max_rpm * 1.02, "rpm {highest}");
}

#[test]
fn a_half_throttle_start_ends_and_the_bike_goes_on_accelerating() {
    let (mut bike, ground) = bike_at(0.0);
    let input = BikeInput { throttle: 0.5, ..Default::default() };
    run(&mut bike, &ground, 6.0, &input);
    let early = bike.forward_speed();
    run(&mut bike, &ground, 6.0, &input);
    // The feathering start used to hold the clutch half open until a speed the engine could not reach at half throttle: the bike
    // stalled at 10 m/s and slowed.
    assert!(bike.forward_speed() > early + 2.0, "{early} then {}", bike.forward_speed());
    assert!(bike.forward_speed() > 15.0, "speed {}", bike.forward_speed());
}

#[test]
fn a_clutch_that_is_fully_in_drives_the_wheel_steadily() {
    let (mut bike, ground) = bike_at(25.0);
    let input = BikeInput { throttle: 1.0, ..Default::default() };
    run(&mut bike, &ground, 1.0, &input);
    // The driving force of the rear tyre over two seconds: a joint that unlocked and locked again each step (the engine's small
    // inertia against the wheel's) drove for one step in three and let the bike down in between.
    let mut forces = Vec::new();
    for _ in 0..(2.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &input, &ground);
        if !bike.shifting() {
            forces.push(bike.wheel(REAR).traction);
        }
    }
    let mean = forces.iter().sum::<f32>() / forces.len() as f32;
    let worst = forces.iter().map(|f| (f - mean).abs()).fold(0.0, f32::max);
    assert!(mean > 100.0, "mean drive {mean}");
    assert!(worst < 0.35 * mean, "mean {mean} worst deviation {worst}");
}

#[test]
fn the_fall_limit_is_the_grip_angle_times_the_front_wheels_grip_limit() {
    let (mut bike, ground) = bike_at(15.0);
    run(&mut bike, &ground, 6.0, &BikeInput { throttle: 0.2, ..Default::default() });
    let spec = bike.spec();
    let rest = spec.front_share() * spec.total_mass() * 9.81;
    let grip = spec.front.tyre.lateral_peak(rest * 1e-3) / rest;
    let expected = grip.atan() * spec.front.lateral_grip_limit;
    let limit = bike.fall_lean_limit();
    assert!((limit - expected).abs() < 0.03 * expected, "{limit} against {expected}");
    // ... which leaves room above the lean limit the stick asks for.
    assert!(limit > grip.atan() * choices::LATERAL_GRIP_TOLERANCE, "{limit}");
}

#[test]
fn a_stick_held_at_three_quarters_does_not_throw_the_bike_at_any_speed() {
    for speed in [8.0_f32, 15.0, 25.0, 30.0] {
        let (mut bike, ground) = bike_at(speed);
        for step in 0..(10.0 / FIXED_STEP) as usize {
            let throttle = (0.25 + 0.2 * (speed - bike.forward_speed())).clamp(0.0, 1.0);
            bike.step(FIXED_STEP, &BikeInput { throttle, steer: 0.75, ..Default::default() }, &ground);
            assert!(!bike.fallen(), "fell at {:.1} s at {speed} m/s", step as f32 * FIXED_STEP);
        }
        assert!(bike.lean() > 0.4, "lean {} at {speed} m/s", bike.lean());
    }
}

fn pedals_like_a_keyboard() -> BikeSpec {
    let mut spec = BikeSpec::example();
    spec.input = InputRates { throttle: [3.0, 5.0], brake: [3.0, 5.0], steer: [25.0, 6.0] };
    spec
}

#[test]
fn a_pedal_held_down_in_a_corner_does_not_throw_the_bike() {
    // A lever of 1.0 for two seconds with the stick at three quarters: the rear wheel lifts or slides and the bike used to fall
    // inward as the brake took the grip the lean needed (at 22 and 30 m/s, 4.9 s and 5.9 s).
    for speed in [22.0_f32, 30.0] {
        let ground = FlatGround::new(0.0);
        let mut bike = Bike::new(pedals_like_a_keyboard());
        assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
        for step in 0..(8.0 / FIXED_STEP) as usize {
            let t = step as f32 * FIXED_STEP;
            let braking = (3.0..5.0).contains(&t);
            let throttle = if braking { 0.0 } else { (0.25 + 0.2 * (speed - bike.forward_speed())).clamp(0.0, 1.0) };
            let lever = f32::from(braking);
            bike.step(FIXED_STEP, &BikeInput { throttle, front_brake: lever, rear_brake: lever, steer: 0.75 }, &ground);
            assert!(!bike.fallen(), "fell at {t:.2} s from {speed} m/s");
        }
    }
}

#[test]
fn the_lean_relief_leaves_a_straight_stop_alone() {
    let (mut bike, ground) = bike_at(25.0);
    run(&mut bike, &ground, 1.0, &BikeInput { front_brake: 1.0, rear_brake: 1.0, ..Default::default() });
    // A bike braking upright at the limit of its tyres loses more than 8 m/s in a second.
    assert!(bike.forward_speed() < 17.0, "{}", bike.forward_speed());
}

#[test]
fn a_key_does_not_slam_the_throttle_open() {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(pedals_like_a_keyboard());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 0.0));
    // Full throttle from a standstill: after a tenth of a second the throttle the bike has is a third of the key's.
    run(&mut bike, &ground, 0.1, &BikeInput { throttle: 1.0, ..Default::default() });
    let quick = bike.rpm();
    let (mut slammed, ground) = bike_at(0.0);
    run(&mut slammed, &ground, 0.1, &BikeInput { throttle: 1.0, ..Default::default() });
    assert!(quick < slammed.rpm(), "{quick} against {}", slammed.rpm());
}
