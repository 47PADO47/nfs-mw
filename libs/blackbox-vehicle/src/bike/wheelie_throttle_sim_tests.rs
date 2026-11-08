//! The wheelie from the throttle alone (`docs/specs/ride6-wheelie-throttle.md`), ridden on the example bike (a 450 cm3 supermoto) and on a
//! superbike of its own (as in `wheelie_hold_tests.rs`), with the weight lever never touched.

use super::{Bike, BikeInput, BikeSpec, FRONT, ThrottleWheelie};
use crate::FIXED_STEP;
use crate::ground::FlatGround;

fn superbike() -> BikeSpec {
    let mut spec = BikeSpec::example();
    spec.chassis.mass = 195.0;
    spec.engine.peak_torque *= 3.0;
    spec.transmission.final_ratio = 5.0;
    spec
}

fn bike_at(spec: &BikeSpec, config: ThrottleWheelie, speed: f32) -> (Bike, FlatGround) {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
    bike.set_throttle_wheelie(config);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    (bike, ground)
}

/// A launch at full throttle for `seconds`: the highest pitch (rad), when it first passed 10 degrees, and whether the bike fell.
fn launch(spec: &BikeSpec, config: ThrottleWheelie, speed: f32, seconds: f32) -> (f32, Option<f32>, bool) {
    let (mut bike, ground) = bike_at(spec, config, speed);
    let (mut high, mut ten) = (0.0_f32, None);
    for step in 0..(seconds / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, ..Default::default() }, &ground);
        high = high.max(bike.pitch());
        if ten.is_none() && bike.pitch() > 10.0_f32.to_radians() {
            ten = Some(step as f32 * FIXED_STEP);
        }
        if bike.fallen() {
            return (high, ten, true);
        }
    }
    (high, ten, false)
}

#[test]
fn with_the_reference_values_a_full_throttle_launch_with_the_lever_released_keeps_the_front_wheel_down() {
    for spec in [BikeSpec::example(), superbike()] {
        let (high, ten, fell) = launch(&spec, ThrottleWheelie::REFERENCE, 0.0, 4.0);
        assert!(!fell && ten.is_none(), "the rider help cuts the throttle: {:.1} degrees", high.to_degrees());
    }
}

#[test]
fn the_game_values_lift_the_front_wheel_from_a_standstill_with_the_throttle_alone() {
    for (name, spec) in [("supermoto", BikeSpec::example()), ("superbike", superbike())] {
        let (high, ten, fell) = launch(&spec, ThrottleWheelie::GAME, 0.0, 4.0);
        let time = ten.unwrap_or(f32::MAX);
        assert!(!fell && time < 1.5, "{name}: 10 degrees after {time} s, fell {fell}");
        assert!(high < 50.0_f32.to_radians(), "{name}: {:.1} degrees is a wheelie, not a loop", high.to_degrees());
    }
}

#[test]
fn the_throttle_held_flat_out_never_flips_the_bike_over() {
    for (name, spec) in [("supermoto", BikeSpec::example()), ("superbike", superbike())] {
        for speed in [0.0_f32, 10.0, 20.0, 30.0] {
            let (high, _, fell) = launch(&spec, ThrottleWheelie::GAME, speed, 12.0);
            assert!(!fell, "{name} from {speed} m/s fell");
            assert!(high < 55.0_f32.to_radians(), "{name} from {speed} m/s: {:.1} degrees", high.to_degrees());
        }
    }
}

/// A bang-bang throttle (on below `target` rad of pitch and for the first 0.6 s) from `speed`, 9 s, the lever released: the lowest and highest
/// pitch of the last 5 s (rad), whether the front wheel was up in all of it, and whether the bike fell.
fn blips(spec: &BikeSpec, speed: f32, target: f32) -> (f32, f32, bool, bool) {
    let (mut bike, ground) = bike_at(spec, ThrottleWheelie::GAME, speed);
    let (mut low, mut high, mut up) = (f32::MAX, f32::MIN, true);
    for step in 0..(9.0 / FIXED_STEP) as usize {
        let time = step as f32 * FIXED_STEP;
        let throttle = f32::from(time < 0.6 || bike.pitch() < target);
        bike.step(FIXED_STEP, &BikeInput { throttle, ..Default::default() }, &ground);
        if bike.fallen() {
            return (low, high, up, true);
        }
        if time >= 4.0 {
            low = low.min(bike.pitch());
            high = high.max(bike.pitch());
            up &= !bike.wheel(FRONT).on_ground;
        }
    }
    (low, high, up, false)
}

#[test]
fn spamming_the_throttle_holds_a_wheelie_with_the_lever_released() {
    let target = 38.0_f32.to_radians();
    for (name, spec) in [("supermoto", BikeSpec::example()), ("superbike", superbike())] {
        for speed in [10.0_f32, 20.0, 30.0] {
            let (low, high, up, fell) = blips(&spec, speed, target);
            let (low, high) = (low.to_degrees(), high.to_degrees());
            assert!(!fell, "{name} at {speed} m/s fell");
            assert!(up && low > 25.0 && high < 48.0, "{name} at {speed} m/s: pitch {low:.1} to {high:.1}, up {up}");
        }
    }
}

#[test]
fn a_throttle_that_is_shut_lowers_the_nose_gently_and_the_lever_is_let_go_when_it_is_down() {
    let (mut bike, ground) = bike_at(&BikeSpec::example(), ThrottleWheelie::GAME, 20.0);
    let (mut fastest, mut touched) = (0.0_f32, false);
    let mut last = 0.0;
    for step in 0..(9.0 / FIXED_STEP) as usize {
        let time = step as f32 * FIXED_STEP;
        bike.step(FIXED_STEP, &BikeInput { throttle: f32::from(time < 4.0), ..Default::default() }, &ground);
        let rate = (last - bike.pitch()) / FIXED_STEP;
        last = bike.pitch();
        if time > 4.0 {
            fastest = fastest.max(rate);
            touched |= bike.wheel(FRONT).on_ground;
        }
        assert!(!bike.fallen());
    }
    assert!(touched, "the front wheel comes down");
    assert!(fastest < 0.8, "the nose drops at {:.0} degrees a second", fastest.to_degrees());
    assert!(!bike.throttle_lever(), "the lever is let go once the front wheel is down and the throttle shut");
}

#[test]
fn the_riders_own_lever_forward_and_a_front_brake_are_not_overridden() {
    let (mut bike, ground) = bike_at(&BikeSpec::example(), ThrottleWheelie::GAME, 0.0);
    bike.set_weight(0.8);
    for _ in 0..(2.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, ..Default::default() }, &ground);
    }
    assert!(!bike.throttle_lever() && bike.pitch() < 5.0_f32.to_radians(), "a lever forward is the stoppie side");
    let (mut bike, ground) = bike_at(&BikeSpec::example(), ThrottleWheelie::GAME, 10.0);
    for _ in 0..(2.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, front_brake: 0.5, ..Default::default() }, &ground);
    }
    assert!(!bike.throttle_lever());
}

#[test]
fn a_bike_with_no_wheelie_control_asks_for_nothing() {
    let mut spec = BikeSpec::example();
    spec.wheelie = None;
    let (_, _, fell) = launch(&spec, ThrottleWheelie::GAME, 0.0, 3.0);
    assert!(!fell);
    let (mut bike, ground) = bike_at(&spec, ThrottleWheelie::GAME, 0.0);
    bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, ..Default::default() }, &ground);
    assert!(!bike.throttle_lever());
}

#[test]
fn the_library_default_is_the_reference() {
    assert_eq!(*Bike::new(BikeSpec::example()).throttle_wheelie(), ThrottleWheelie::REFERENCE);
}
