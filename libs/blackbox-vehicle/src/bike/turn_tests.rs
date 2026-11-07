//! A numeric harness for turning (`docs/specs/ride6-feel-round12.md`, section 2; `ride6-feel-round13.md`): how fast the lean comes in after a
//! step of the stick, to the left and to the right and at 20 to 80 m/s, and how tight the steady turn is, on the example bike (a 450 cm3
//! supermoto with the data of the KTM 450 SMR) with the data's input rates, and over the shapes the 288 bikes' tables take.
//!
//! The bounds around the lean-in times come from the controller's own filters, worked out outside the model (the stick filter, the
//! target filter stepped by `dt x time scale` and the lean spring, with the lean following its target exactly), so the open-loop tests
//! check the port of the filters and the closed-loop ones that the bike does not add much to it. The numbers of the closed-loop tests
//! are wide on purpose: they stand for "about", not for a measurement. These tests were written without running them.

use super::lean::{Lean, LeanInput};
use super::{Bike, BikeInput, BikeSpec, FRONT, InputRates, SteeringSpec, choices};
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

/// The example bike with the power and gearing of a superbike (its top speed is over 85 m/s; the example's own is 52 m/s, which makes a
/// ride at 60 or 80 m/s a rev limiter and an engine brake and not a turn): the high-speed turns of the closed-loop tests ride this one.
fn superbike() -> BikeSpec {
    let mut spec = with_data_rates();
    spec.engine.peak_torque *= 3.5;
    spec.transmission.final_ratio = 4.0;
    spec
}

/// What the controller is given at `speed` with the lean at `lean` and rolling at `rate`, on flat ground, a stick of `stick`.
fn plan_input(stick: f32, speed: f32, lean: f32, rate: f32) -> LeanInput {
    LeanInput {
        stick,
        speed,
        lean,
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
    }
}

/// The shapes of the stick's time-scale curve in the data of the 288 bikes, `(at -1, at +1)` (207, 61 and 20 bikes), the same two ways
/// round, and an even one, which no bike has but a mod's could.
const STICK_ENDS: [(f32, f32); 6] = [(0.4, 0.5), (0.28, 0.45), (0.425, 0.7), (0.7, 0.425), (0.5, 0.4), (0.6, 0.6)];

/// The speed curves of the time scale in the data: 1 at 40 km/h falling to 0.333 at 330 km/h (270 bikes), and 1 at 10 km/h falling to
/// 0.65 at 200 km/h (18 bikes).
fn speed_tables() -> [Vec<(f32, f32)>; 2] {
    [vec![(11.111, 1.0), (91.667, 0.333)], vec![(2.778, 1.0), (55.556, 0.65)]]
}

/// The example bike's steering with a stick curve of the given ends (1 at the middle) and a speed curve.
fn steering_with(ends: (f32, f32), speed_table: &[(f32, f32)]) -> SteeringSpec {
    let mut steering = BikeSpec::example().steering;
    steering.lean.time_scale_input = vec![(-1.0, ends.0), (0.0, 1.0), (1.0, ends.1)];
    steering.lean.time_scale_speed = speed_table.to_vec();
    steering
}

/// The seconds a held `stick` takes to bring the lean to each of `angles` (rad) when the lean follows the controller's prescription
/// exactly (the roll acceleration of the plan integrated as it asks), at `speed` along the bike on flat ground. `None` for an angle not reached
/// in 5 s.
fn open_loop_times(steering: &SteeringSpec, speed: f32, stick: f32, angles: &[f32]) -> Vec<Option<f32>> {
    let mut lean = Lean::new();
    let (mut angle, mut rate) = (0.0_f32, 0.0_f32);
    let mut times = vec![None; angles.len()];
    for step in 0..(5.0 / SUBSTEP) as usize {
        let plan = lean.plan(steering, &plan_input(stick, speed, angle, rate));
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
    let steering = BikeSpec::example().steering;
    // `(speed, stick, bounds of the time to 20 degrees, bounds of the time to 40 degrees)`, round about the worked-out times (0.46 and
    // 0.82 s at 20 m/s, 0.49 and 0.87 s at 40 m/s); a left stick is the same as a right one since round 13.
    let cases = [
        (20.0_f32, 1.0_f32, (0.30, 0.65), (0.60, 1.10)),
        (20.0, -1.0, (0.30, 0.65), (0.60, 1.10)),
        (40.0, 1.0, (0.33, 0.70), (0.65, 1.20)),
        (40.0, -1.0, (0.33, 0.70), (0.65, 1.20)),
    ];
    for (speed, stick, to_twenty, to_forty) in cases {
        let times = open_loop_times(&steering, speed, stick, &[twenty, forty]);
        let (a, b) = (times[0].expect("20 degrees"), times[1].expect("40 degrees"));
        assert!((to_twenty.0..=to_twenty.1).contains(&a), "{speed} m/s, stick {stick}: 20 degrees in {a} s");
        assert!((to_forty.0..=to_forty.1).contains(&b), "{speed} m/s, stick {stick}: 40 degrees in {b} s");
    }
}

#[test]
fn a_left_stick_asks_for_the_mirror_of_a_right_one_whatever_the_shape_of_the_tables() {
    for ends in STICK_ENDS {
        for speed_table in speed_tables() {
            let steering = steering_with(ends, &speed_table);
            for speed in [10.0_f32, 40.0, 80.0] {
                for stick in [0.25_f32, 0.5, 1.0] {
                    let (mut right, mut left) = (Lean::new(), Lean::new());
                    for step in 0..240 {
                        let a = right.plan(&steering, &plan_input(stick, speed, 0.0, 0.0));
                        let b = left.plan(&steering, &plan_input(-stick, speed, 0.0, 0.0));
                        let what = format!("ends {ends:?}, {speed} m/s, stick {stick}, step {step}");
                        assert!((a.target + b.target).abs() < 1e-5, "{what}: target {} {}", a.target, b.target);
                        assert!((a.limit - b.limit).abs() < 1e-6, "{what}: limit");
                        let tolerance = 1e-3 * (1.0 + a.acceleration.abs());
                        assert!((a.acceleration + b.acceleration).abs() < tolerance, "{what}: acceleration");
                    }
                }
            }
        }
    }
}

#[test]
fn a_left_turn_comes_in_as_fast_as_a_right_one_whatever_the_data_tables() {
    let wanted = 0.8 * GRIP.atan();
    for ends in STICK_ENDS {
        for speed_table in speed_tables() {
            let steering = steering_with(ends, &speed_table);
            for speed in [10.0_f32, 20.0, 40.0, 60.0, 80.0] {
                let right = open_loop_times(&steering, speed, 1.0, &[wanted])[0].expect("right");
                let left = open_loop_times(&steering, speed, -1.0, &[wanted])[0].expect("left");
                assert!((left - right).abs() < 1e-3, "ends {ends:?}, {speed} m/s: left {left} s, right {right} s");
            }
        }
    }
}

#[test]
fn the_quicker_side_of_the_data_is_never_made_slower() {
    // The data's 0.28 / 0.45 curve: the right stick's 0.45 is the time scale of both sides, so the left comes in as the right did.
    let wanted = 0.8 * GRIP.atan();
    let steering = steering_with((0.28, 0.45), &speed_tables()[0]);
    let mut right_only = steering.clone();
    right_only.lean.time_scale_input = vec![(-1.0, 0.45), (0.0, 1.0), (1.0, 0.45)];
    let left = open_loop_times(&steering, 30.0, -1.0, &[wanted])[0].expect("left");
    let right = open_loop_times(&right_only, 30.0, 1.0, &[wanted])[0].expect("right");
    assert!((left - right).abs() < 1e-3, "{left} s against {right} s");
}

#[test]
fn the_lean_in_time_grows_little_with_speed_whatever_the_data_tables() {
    // 80 percent of the lean limit after a full stick: about a second at 20 m/s; the data's speed curve alone would make it 1.6 s (right) to
    // 2.3 s (left) at 80 m/s; with the floor it is within 30 percent of the time at 20 m/s and under 1.5 s.
    let wanted = 0.8 * GRIP.atan();
    for ends in STICK_ENDS {
        for speed_table in speed_tables() {
            let steering = steering_with(ends, &speed_table);
            let time = |speed: f32| open_loop_times(&steering, speed, 1.0, &[wanted])[0].expect("reached");
            let base = time(20.0);
            for speed in [40.0_f32, 60.0, 80.0] {
                let t = time(speed);
                assert!(t < base * 1.3 && t < 1.5, "ends {ends:?}, {speed} m/s: {t} s against {base} s at 20 m/s");
            }
        }
    }
}

/// A bike of `spec` at `speed` m/s on flat ground, ridden with a stick of `stick` and a throttle that holds the speed; the time its lean first
/// reaches each of `angles` (rad), whether it fell, and the radius of the turn over the last two of `seconds` (m, 0 if it does not turn)
/// with the mean speed (m/s) and the mean lean (rad) it was ridden at there.
fn ride_turn(
    spec: &BikeSpec,
    speed: f32,
    stick: f32,
    seconds: f32,
    angles: &[f32],
) -> (Vec<Option<f32>>, bool, f32, [f32; 2]) {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec.clone());
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
            let (times, fell, _, _) =
                ride_turn(&with_data_rates(), speed, stick, 6.0, &[20.0_f32.to_radians(), 40.0_f32.to_radians()]);
            assert!(!fell, "{speed} m/s, stick {stick}: fell");
            let (a, b) = (times[0].expect("20 degrees"), times[1].expect("40 degrees"));
            assert!(a < 1.2 && b < 2.2, "{speed} m/s, stick {stick}: 20 degrees in {a} s, 40 degrees in {b} s");
        }
    }
}

#[test]
fn a_full_stick_leans_the_bike_in_about_the_same_time_at_20_and_at_80_ms_in_the_game_loop() {
    let angles = [20.0_f32.to_radians(), 40.0_f32.to_radians()];
    let at = |speed: f32, stick: f32| {
        let (times, fell, _, _) = ride_turn(&superbike(), speed, stick, 5.0, &angles);
        assert!(!fell, "{speed} m/s, stick {stick}: fell");
        (times[0].expect("20 degrees"), times[1].expect("40 degrees"))
    };
    for stick in [1.0_f32, -1.0] {
        let (base_a, base_b) = at(20.0, stick);
        for speed in [40.0_f32, 60.0, 80.0] {
            let (a, b) = at(speed, stick);
            let what = format!("{speed} m/s, stick {stick}: 20 degrees in {a} s, 40 degrees in {b} s");
            // The open loop grows by at most 30 percent over this range (above); the bike's own lag is the same at any speed, so the
            // closed loop has room for 40 percent and a tenth of a second.
            assert!(a < base_a * 1.4 + 0.1 && b < base_b * 1.4 + 0.1, "{what} (20 m/s: {base_a} s, {base_b} s)");
            assert!(b < 2.2, "{what}");
        }
    }
}

#[test]
fn a_held_stick_turns_within_the_radius_of_its_lean_and_a_wide_tyres_geometry() {
    for speed in [10.0_f32, 20.0, 30.0] {
        let (_, fell, radius, [mean_speed, mean_lean]) = ride_turn(&with_data_rates(), speed, 0.9, 8.0, &[]);
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
fn a_full_stick_turns_at_the_radius_of_its_lean_and_leans_as_far_at_20_to_80_ms() {
    let spec = superbike();
    let mut base_lean = None;
    for speed in [20.0_f32, 40.0, 60.0, 80.0] {
        let (_, fell, radius, [mean_speed, mean_lean]) = ride_turn(&spec, speed, 1.0, 8.0, &[]);
        assert!(!fell, "{speed} m/s: fell");
        let lean_radius = mean_speed * mean_speed / (9.81 * mean_lean.tan());
        assert!(
            radius > lean_radius * 0.7 && radius < lean_radius * 1.6,
            "{speed} m/s: radius {radius} m against {lean_radius} m of its lean ({mean_lean} rad)"
        );
        // The lean a full stick reaches is the limit's, the same at any speed (the limit is `atan` of the tyre's grip, which has no speed
        // term); allow the cut of the acceleration along the bike (drag at speed) and the geometry.
        let base = *base_lean.get_or_insert(mean_lean);
        assert!(mean_lean > 0.7 && mean_lean > base * 0.85, "{speed} m/s: lean {mean_lean} rad (20 m/s: {base} rad)");
    }
}

#[test]
fn a_stick_pulled_to_its_reach_gives_the_whole_lean_limit_and_one_short_of_it_a_little_less() {
    // The reach is the input layer's (`sim/bike.rs::command`): 0.9 of the travel reads as the whole stick. Here the library takes the stick.
    const { assert!(choices::STEER_REACH > 0.5 && choices::STEER_REACH <= 1.0) };
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
