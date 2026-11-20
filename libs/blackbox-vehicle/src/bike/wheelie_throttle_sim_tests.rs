//! The wheelie from the throttle alone (`docs/specs/ride6-wheelie-throttle.md`), ridden on the example bike (a 450 cm3 supermoto) and on a
//! superbike of its own (as in `wheelie_hold_tests.rs`), with the weight lever never touched: the lift, the cap of each gear, the slow rise,
//! the dip at a gear change and the latch after the front wheel was put down.

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

/// One step of a ride as the tests read it.
#[derive(Clone, Copy, Debug)]
struct Sample {
    time: f32,
    pitch: f32,
    gear: usize,
    shifting: bool,
    lever: bool,
    front_down: bool,
}

/// A ride with `throttle(time)` for `seconds`: a sample per step, and whether the bike fell.
fn ride(
    spec: &BikeSpec,
    config: ThrottleWheelie,
    speed: f32,
    seconds: f32,
    throttle: &dyn Fn(f32) -> f32,
) -> (Vec<Sample>, bool) {
    let (mut bike, ground) = bike_at(spec, config, speed);
    let mut samples = Vec::new();
    for step in 0..(seconds / FIXED_STEP) as usize {
        let time = step as f32 * FIXED_STEP;
        bike.step(FIXED_STEP, &BikeInput { throttle: throttle(time), ..Default::default() }, &ground);
        if bike.fallen() {
            return (samples, true);
        }
        samples.push(Sample {
            time,
            pitch: bike.pitch(),
            gear: bike.gear(),
            shifting: bike.shifting(),
            lever: bike.throttle_lever(),
            front_down: bike.wheel(FRONT).on_ground,
        });
    }
    (samples, false)
}

/// A launch at full throttle for `seconds`: the highest pitch (rad), when it first passed `at` rad, and whether the bike fell.
fn launch(spec: &BikeSpec, config: ThrottleWheelie, speed: f32, seconds: f32, at: f32) -> (f32, Option<f32>, bool) {
    let (samples, fell) = ride(spec, config, speed, seconds, &|_| 1.0);
    let high = samples.iter().map(|s| s.pitch).fold(0.0, f32::max);
    (high, samples.iter().find(|s| s.pitch > at).map(|s| s.time), fell)
}

/// The throttle-only wheelie without its new limits: an allowed angle that is not held back by the rate, the nose, the gear's cap or the
/// dip, and no engine cut: what the assist did before the playtest asked for it gentler.
fn unlimited() -> ThrottleWheelie {
    ThrottleWheelie {
        gear_caps: [1.2; 3],
        rise_rate: 100.0,
        fall_rate: 100.0,
        lead: 10.0,
        shift_dip: 0.0,
        cut_most: 0.0,
        shift_cut: 0.0,
        ..ThrottleWheelie::GAME
    }
}

/// The first speed (m/s) at which the bike is placed in `gear`.
fn speed_in_gear(spec: &BikeSpec, gear: usize) -> f32 {
    (0..60)
        .map(|v| v as f32)
        .find(|&speed| bike_at(spec, ThrottleWheelie::REFERENCE, speed).0.gear() == gear)
        .expect("a speed in that gear")
}

#[test]
fn with_the_reference_values_a_full_throttle_launch_with_the_lever_released_keeps_the_front_wheel_down() {
    for spec in [BikeSpec::example(), superbike()] {
        let (high, ten, fell) = launch(&spec, ThrottleWheelie::REFERENCE, 0.0, 4.0, 10.0_f32.to_radians());
        assert!(!fell && ten.is_none(), "the rider help cuts the throttle: {:.1} degrees", high.to_degrees());
    }
}

#[test]
fn the_game_values_lift_the_front_wheel_from_a_standstill_with_the_throttle_alone() {
    for (name, spec) in [("supermoto", BikeSpec::example()), ("superbike", superbike())] {
        let (high, ten, fell) = launch(&spec, ThrottleWheelie::GAME, 0.0, 6.0, 10.0_f32.to_radians());
        let time = ten.unwrap_or(f32::MAX);
        assert!(!fell && time < 3.5, "{name}: 10 degrees after {time} s, fell {fell}");
        assert!(high < 46.0_f32.to_radians(), "{name}: {:.1} degrees is a wheelie, not a loop", high.to_degrees());
    }
}

#[test]
fn the_lift_is_slower_than_it_was() {
    let (at, game) = (25.0_f32.to_radians(), ThrottleWheelie::GAME);
    for (name, spec) in [("supermoto", BikeSpec::example()), ("superbike", superbike())] {
        let slow = launch(&spec, game, 0.0, 8.0, at).1.expect("the game's assist reaches 25 degrees");
        let quick = launch(&spec, unlimited(), 0.0, 8.0, at).1.expect("the old assist reaches 25 degrees");
        println!("{name}: 25 degrees after {slow:.2} s, before {quick:.2} s");
        assert!(slow >= quick * 1.3 + 0.2, "{name}: {slow} s against {quick} s before");
    }
}

#[test]
fn the_assist_never_lifts_beyond_the_cap_of_the_gear_it_is_in() {
    let game = ThrottleWheelie::GAME;
    for (name, spec) in [("supermoto", BikeSpec::example()), ("superbike", superbike())] {
        for speed in [0.0_f32, 8.0, 14.0] {
            let (samples, fell) = ride(&spec, game, speed, 14.0, &|_| 1.0);
            assert!(!fell, "{name} from {speed} m/s fell");
            let mut changed = f32::MIN;
            let mut gear = 0;
            let mut lifted = 0.0_f32;
            for s in &samples {
                if s.gear != gear {
                    (gear, changed) = (s.gear, s.time);
                }
                lifted = lifted.max(s.pitch);
                let Some(cap) = game.cap(s.gear) else { continue };
                // The nose may stand a little over its cap while it comes down to a lower gear's cap.
                if !s.lever || s.time - changed < 1.5 {
                    continue;
                }
                assert!(
                    s.pitch < cap + 6.0_f32.to_radians(),
                    "{name} from {speed} m/s: {:.1} degrees in gear {}",
                    s.pitch.to_degrees(),
                    s.gear
                );
            }
            assert!(lifted < 50.0_f32.to_radians(), "{name} from {speed} m/s: {:.1} degrees", lifted.to_degrees());
        }
    }
}

#[test]
fn no_wheelie_is_asked_in_a_gear_above_the_third() {
    let spec = superbike();
    let speed = speed_in_gear(&spec, 4);
    let (samples, fell) = ride(&spec, ThrottleWheelie::GAME, speed, 6.0, &|_| 1.0);
    assert!(!fell);
    for s in samples.iter().filter(|s| s.gear >= 4) {
        assert!(!s.lever, "the lever is asked in gear {} at {:.2} s", s.gear, s.time);
    }
    assert!(samples.iter().any(|s| s.gear >= 4), "the ride never was in fourth gear");
}

#[test]
fn the_rider_who_wheelies_with_the_lever_is_not_held_to_the_cap() {
    let spec = superbike();
    let speed = speed_in_gear(&spec, 3);
    let cap = ThrottleWheelie::GAME.cap(3).unwrap();
    let high = |manual: bool| {
        let (mut bike, ground) = bike_at(&spec, ThrottleWheelie::GAME, speed);
        bike.set_weight(if manual { -1.0 } else { 0.0 });
        let mut high = 0.0_f32;
        for _ in 0..(4.0 / FIXED_STEP) as usize {
            bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, ..Default::default() }, &ground);
            if bike.gear() == 3 {
                high = high.max(bike.pitch());
            }
        }
        high
    };
    let (assisted, manual) = (high(false), high(true));
    println!(
        "third gear: the assist {:.1} degrees, the lever {:.1}, the cap {:.1}",
        assisted.to_degrees(),
        manual.to_degrees(),
        cap.to_degrees()
    );
    assert!(assisted < cap + 6.0_f32.to_radians(), "{:.1} degrees", assisted.to_degrees());
    assert!(
        manual > assisted + 1.0_f32.to_radians(),
        "the lever's wheelie {:.1} degrees, the assist's {:.1}",
        manual.to_degrees(),
        assisted.to_degrees()
    );
}

#[test]
fn a_gear_change_takes_the_nose_down_a_little_before_it_goes_up_again() {
    for (name, spec) in [("supermoto", BikeSpec::example()), ("superbike", superbike())] {
        let (samples, fell) = ride(&spec, ThrottleWheelie::GAME, 0.0, 12.0, &|_| 1.0);
        assert!(!fell);
        // The first change made with the front wheel well up by the throttle's lever.
        let start = samples
            .windows(2)
            .position(|w| !w[0].shifting && w[1].shifting && w[1].lever && w[1].pitch > 8.0_f32.to_radians())
            .unwrap_or_else(|| panic!("{name}: no gear change in a wheelie"));
        let before = samples[start].pitch;
        let window = (0.8 / FIXED_STEP) as usize;
        let lowest = samples[start..start + window].iter().map(|s| s.pitch).fold(f32::MAX, f32::min);
        println!(
            "{name}: {:.1} degrees at the change, {:.1} at the lowest of 0.8 s",
            before.to_degrees(),
            lowest.to_degrees()
        );
        assert!(
            lowest < before - 1.0_f32.to_radians(),
            "{name}: the nose went from {:.1} to {:.1} degrees",
            before.to_degrees(),
            lowest.to_degrees()
        );
        let later = samples[start..].iter().take((3.0 / FIXED_STEP) as usize).map(|s| s.pitch).fold(0.0, f32::max);
        assert!(later > lowest + 1.0_f32.to_radians(), "{name}: it goes up again, {:.1} degrees", later.to_degrees());
    }
}

#[test]
fn a_front_wheel_put_down_is_not_lifted_again_by_the_gas_until_first_gear_or_a_stop() {
    let spec = superbike();
    // The gas for 3 s, shut for 3.5 s (the front wheel comes down), then flat out again for 3 s.
    let (samples, fell) = ride(&spec, ThrottleWheelie::GAME, 0.0, 9.5, &|t| f32::from(!(3.0..6.5).contains(&t)));
    assert!(!fell);
    let up = samples.iter().filter(|s| s.time < 3.0).map(|s| s.pitch).fold(0.0, f32::max);
    assert!(up > 10.0_f32.to_radians(), "the first wheelie: {:.1} degrees", up.to_degrees());
    let touched = samples.iter().filter(|s| (3.0..6.5).contains(&s.time)).any(|s| s.front_down);
    assert!(touched, "the front wheel comes down while the throttle is shut");
    // Back in a higher gear than the first the gas does not lift it again.
    let again: Vec<&Sample> = samples.iter().filter(|s| s.time >= 6.5 && s.gear != 1).collect();
    assert!(!again.is_empty());
    assert!(again.iter().all(|s| !s.lever), "the lever is pulled again by the gas");
    let high = again.iter().map(|s| s.pitch).fold(0.0, f32::max);
    assert!(high < 8.0_f32.to_radians(), "{:.1} degrees", high.to_degrees());
}

#[test]
fn the_throttle_held_flat_out_never_flips_the_bike_over() {
    for (name, spec) in [("supermoto", BikeSpec::example()), ("superbike", superbike())] {
        for speed in [0.0_f32, 10.0, 20.0, 30.0] {
            let (high, _, fell) = launch(&spec, ThrottleWheelie::GAME, speed, 12.0, 1.0);
            assert!(!fell, "{name} from {speed} m/s fell");
            assert!(high < 50.0_f32.to_radians(), "{name} from {speed} m/s: {:.1} degrees", high.to_degrees());
        }
    }
}

#[test]
fn a_throttle_that_is_shut_lowers_the_nose_gently_and_the_lever_is_let_go_when_it_is_down() {
    let (mut bike, ground) = bike_at(&BikeSpec::example(), ThrottleWheelie::GAME, 6.0);
    let (mut fastest, mut touched) = (0.0_f32, false);
    let mut last = 0.0;
    for step in 0..(10.0 / FIXED_STEP) as usize {
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
    let (_, _, fell) = launch(&spec, ThrottleWheelie::GAME, 0.0, 3.0, 1.0);
    assert!(!fell);
    let (mut bike, ground) = bike_at(&spec, ThrottleWheelie::GAME, 0.0);
    bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, ..Default::default() }, &ground);
    assert!(!bike.throttle_lever());
}

#[test]
fn the_library_default_is_the_reference() {
    assert_eq!(*Bike::new(BikeSpec::example()).throttle_wheelie(), ThrottleWheelie::REFERENCE);
}
