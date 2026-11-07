//! A numeric harness for turning (`docs/specs/ride6-feel-round12.md`, section 2): how fast the lean comes in after a step of the stick, and
//! how tight the steady turn is, on the example bike (a 450 cm3 supermoto with the data of the KTM 450 SMR) with the data's input rates.
//!
//! The bounds around the lean-in times come from the controller's own filters, worked out outside the model (the stick filter, the
//! target filter stepped by `dt x time scale` and the lean spring, with the lean following its target exactly), so the open-loop test
//! checks the port of the filters and the closed-loop one that the bike does not add much to it. The numbers of the closed-loop tests
//! are wide on purpose: they stand for "about", not for a measurement. These tests were written without running them.

use super::lean::{Lean, LeanInput};
use super::{Bike, BikeInput, BikeSpec, FRONT, InputRates, choices};
use crate::FIXED_STEP;
use crate::ground::FlatGround;

/// The tyre grip of the example bike's front wheel the lean limit is taken from here: `atan(1.3)`, 52 degrees.
const GRIP: f32 = 1.3;
/// The sub-step the open-loop chain runs at (s), the game's.
const SUBSTEP: f32 = choices::MAX_SUBSTEP;

/// The data's rates of the throttle, the brake and the stick (a key takes a third of a second to open the throttle, the stick moves at
/// 25 per second away from the middle and 6 towards it).
fn with_data_rates() -> BikeSpec {
    let mut spec = BikeSpec::example();
    spec.input = InputRates { throttle: [3.0, 5.0], brake: [3.0, 5.0], steer: [25.0, 6.0] };
    spec
}

/// The seconds a held `stick` takes to bring the lean to each of `angles` (rad) when the lean follows the controller's prescription
/// exactly (the roll acceleration of the plan integrated as it asks), at `speed` along the bike on flat ground. `None` for an angle not reached
/// in 5 s.
fn open_loop_times(speed: f32, stick: f32, angles: &[f32]) -> Vec<Option<f32>> {
    let steering = BikeSpec::example().steering;
    let mut lean = Lean::new();
    let (mut angle, mut rate) = (0.0_f32, 0.0_f32);
    let mut times = vec![None; angles.len()];
    for step in 0..(5.0 / SUBSTEP) as usize {
        let input = LeanInput {
            stick,
            speed,
            lean: angle,
            lean_rate: rate,
            lateral_grip: GRIP,
            long_g: 0.0,
            heading_error: 0.0,
            pitch_cut: (0.0, 0.0),
            terrain_slope: 0.0,
            roll_inertia: 47.8,
            cg_height: 0.8,
            front_arm: 1.0,
            target_cap: f32::MAX,
            burnout_limit: None,
            dt: SUBSTEP,
        };
        let plan = lean.plan(&steering, &input);
        rate += plan.acceleration * SUBSTEP;
        angle += rate * SUBSTEP;
        let now = (step + 1) as f32 * SUBSTEP;
        for (time, wanted) in times.iter_mut().zip(angles) {
            if time.is_none() && angle.abs() >= *wanted {
                *time = Some(now);
            }
        }
    }
    times
}

#[test]
fn the_lean_comes_in_over_about_a_second_after_a_step_of_the_stick() {
    let (twenty, forty) = (20.0_f32.to_radians(), 40.0_f32.to_radians());
    // `(speed, stick, bounds of the time to 20 degrees, bounds of the time to 40 degrees)`, round about the worked-out times (right: 0.46 and
    // 0.82 s at 20 m/s, left: 0.56 and 1.01 s; at 40 m/s 0.49 and 0.87 s, left 0.60 and 1.10 s).
    let cases = [
        (20.0_f32, 1.0_f32, (0.30, 0.65), (0.60, 1.10)),
        (20.0, -1.0, (0.40, 0.80), (0.75, 1.35)),
        (40.0, 1.0, (0.33, 0.70), (0.65, 1.20)),
        (40.0, -1.0, (0.42, 0.85), (0.80, 1.50)),
    ];
    for (speed, stick, to_twenty, to_forty) in cases {
        let times = open_loop_times(speed, stick, &[twenty, forty]);
        let (a, b) = (times[0].expect("20 degrees"), times[1].expect("40 degrees"));
        assert!((to_twenty.0..=to_twenty.1).contains(&a), "{speed} m/s, stick {stick}: 20 degrees in {a} s");
        assert!((to_forty.0..=to_forty.1).contains(&b), "{speed} m/s, stick {stick}: 40 degrees in {b} s");
    }
}

#[test]
fn a_left_turn_comes_in_slower_than_a_right_one_because_the_data_says_so() {
    // The time scale is cut by a curve of the signed stick (0.425 at -1, 0.7 at +1 here): ride6-feel-round12.md, section 3.
    let forty = 40.0_f32.to_radians();
    let right = open_loop_times(30.0, 1.0, &[forty])[0].expect("right");
    let left = open_loop_times(30.0, -1.0, &[forty])[0].expect("left");
    assert!(left > right * 1.05, "left {left} s, right {right} s");
}

/// A bike at `speed` m/s on flat ground, ridden with a stick of `stick` and a throttle that holds the speed; the time its lean first
/// reaches each of `angles` (rad), whether it fell, and the radius of the turn over the last two of `seconds` (m, 0 if it does not turn)
/// with the mean speed (m/s) and the mean lean (rad) it was ridden at there.
fn ride_turn(speed: f32, stick: f32, seconds: f32, angles: &[f32]) -> (Vec<Option<f32>>, bool, f32, [f32; 2]) {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(with_data_rates());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    let mut times = vec![None; angles.len()];
    let (mut speeds, mut yaw_rates, mut leans, mut taken) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
    for step in 0..(seconds / FIXED_STEP) as usize {
        let throttle = (0.25 + 0.2 * (speed - bike.forward_speed())).clamp(0.0, 1.0);
        bike.step(FIXED_STEP, &BikeInput { throttle, steer: stick, ..Default::default() }, &ground);
        if bike.fallen() {
            return (times, true, 0.0, [0.0; 2]);
        }
        let now = (step + 1) as f32 * FIXED_STEP;
        for (time, wanted) in times.iter_mut().zip(angles) {
            if time.is_none() && bike.lean().abs() >= *wanted {
                *time = Some(now);
            }
        }
        if now > seconds - 2.0 {
            speeds += bike.forward_speed();
            yaw_rates += bike.body().angular_velocity.y.abs();
            leans += bike.lean().abs();
            taken += 1.0;
        }
    }
    let radius = match yaw_rates > 1e-3 {
        true => speeds / yaw_rates,
        false => 0.0,
    };
    (times, false, radius, [speeds / taken.max(1.0), leans / taken.max(1.0)])
}

#[test]
fn the_bike_leans_in_over_about_a_second_and_a_half_at_most_in_the_game_loop() {
    for speed in [10.0_f32, 20.0, 30.0] {
        for stick in [0.9_f32, -0.9] {
            let (times, fell, _, _) = ride_turn(speed, stick, 6.0, &[20.0_f32.to_radians(), 40.0_f32.to_radians()]);
            assert!(!fell, "{speed} m/s, stick {stick}: fell");
            let (a, b) = (times[0].expect("20 degrees"), times[1].expect("40 degrees"));
            assert!(a < 1.2 && b < 2.2, "{speed} m/s, stick {stick}: 20 degrees in {a} s, 40 degrees in {b} s");
        }
    }
}

#[test]
fn a_held_stick_turns_within_the_radius_of_its_lean_and_a_wide_tyres_geometry() {
    for speed in [10.0_f32, 20.0, 30.0] {
        let (_, fell, radius, [mean_speed, mean_lean]) = ride_turn(speed, 0.9, 8.0, &[]);
        assert!(!fell, "{speed} m/s: fell");
        // The radius of the lean alone is `v^2 / (g tan(lean))`; the geometry of a wide tyre (the patch moves in with the lean) and the speed
        // a corner costs make the real one wider (round 11: 1.0 to 1.2 g where `tan` of the lean says 1.3).
        let lean_radius = mean_speed * mean_speed / (9.81 * mean_lean.tan());
        assert!(
            radius > lean_radius * 0.7,
            "{speed} m/s: radius {radius} m against {lean_radius} m: tighter than the lean allows"
        );
        assert!(
            radius < lean_radius * 1.6,
            "{speed} m/s: radius {radius} m against {lean_radius} m: wider than the lean needs"
        );
    }
}

#[test]
fn a_stick_pulled_to_its_reach_gives_the_whole_lean_limit_and_one_short_of_it_a_little_less() {
    // The reach is the input layer's (`sim/bike.rs::command`): 0.9 of the travel reads as the whole stick. Here the library takes the stick.
    assert!(choices::STEER_REACH > 0.5 && choices::STEER_REACH <= 1.0);
    let ground = FlatGround::new(0.0);
    let lean_at = |stick: f32| {
        let mut bike = Bike::new(with_data_rates());
        assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 20.0));
        for _ in 0..(5.0 / FIXED_STEP) as usize {
            let throttle = (0.25 + 0.2 * (20.0 - bike.forward_speed())).clamp(0.0, 1.0);
            bike.step(FIXED_STEP, &BikeInput { throttle, steer: stick, ..Default::default() }, &ground);
        }
        (bike.target_lean().abs(), bike.lean_limit(), bike.wheel(FRONT).on_ground)
    };
    let (full, limit, down) = lean_at(1.0);
    let (part, _, _) = lean_at(0.85);
    assert!(down && (full - limit).abs() < 0.1 * limit, "a full stick asks for the lean limit: {full} of {limit}");
    assert!(part < full * 0.95 && part > full * 0.75, "0.85 of the stick asks for 0.85 of it: {part} against {full}");
}
