//! Steering with the front wheel up (`docs/specs/ride6-wheelie-steer.md`): the mid-air steer's gates, the wheelie's lean help, and a
//! wheelie that turns with the stick without falling.

use super::*;
use crate::FIXED_STEP;
use crate::bike::{BikeInput, BikeSpec};
use crate::ground::FlatGround;

#[test]
fn the_mid_air_steer_needs_the_front_wheel_up_a_small_lean_and_a_nose_that_is_not_dropping() {
    assert_eq!(mid_air_intensity(0.0, 0.0, 0.0), 0.0, "the front wheel on the road");
    assert_eq!(mid_air_intensity(0.03, 0.0, 0.0), 0.0, "under 4 cm");
    assert_eq!(mid_air_intensity(0.05, 0.0, 0.0), 1.0);
    assert_eq!(mid_air_intensity(0.05, 0.3, 0.0), 1.0);
    assert_eq!(mid_air_intensity(0.05, -0.5, 0.0), 0.0, "the lean is over 40 percent of its limit, either side");
    assert_eq!(mid_air_intensity(0.30, 0.0, -0.5), 0.0, "the nose drops faster than 0.4 rad/s");
    assert!((mid_air_intensity(0.30, 0.0, -0.35) - 0.5).abs() < 1e-5);
    assert_eq!(mid_air_intensity(0.30, 0.0, 2.0), 1.0, "a nose that rises is no reason to let go");
}

#[test]
fn the_mid_air_handlebar_is_eight_tenths_of_the_lock_against_the_stick() {
    assert!((mid_air_angle(1.0, 0.5) + 0.4).abs() < 1e-6);
    assert!((mid_air_angle(-1.0, 0.5) - 0.4).abs() < 1e-6);
    assert_eq!(mid_air_angle(0.0, 0.5), 0.0);
    assert!((mid_air_angle(3.0, 0.5) + 0.4).abs() < 1e-6, "the stick is held to -1..1");
}

#[test]
fn a_hovering_front_wheel_gets_no_yaw_torque_and_a_wheelie_gets_all_of_it() {
    let range = [10.0 / 3.6, 15.0 / 3.6];
    // The hover a throttle opened with the stick flicked makes: a lift of 0.02 to 0.05 rad (round 11, section 1.2).
    assert_eq!(yaw_weight(12.0, 0.03, range), 0.0);
    assert_eq!(yaw_weight(12.0, 0.05, range), 0.0);
    assert_eq!(yaw_weight(12.0, 0.6, range), 1.0, "a wheelie of 35 degrees");
    assert!((yaw_weight(12.0, 0.25, range) - 0.5).abs() < 1e-5);
    assert_eq!(yaw_weight(1.0, 0.6, range), 0.0, "and none at a walking pace");
    assert_eq!(yaw_weight(-12.0, 0.6, range), 1.0, "the speed counts by its size");
}

#[test]
fn the_coordinated_yaw_rate_is_that_of_a_steady_turn() {
    let rate = coordinated_yaw_rate(0.3, 12.0, 1.3, 4.0);
    assert!((rate - 9.81 * 0.3_f32.tan() / 12.0).abs() < 1e-6);
    assert!(coordinated_yaw_rate(-0.3, 12.0, 1.3, 4.0) < 0.0);
    assert_eq!(
        coordinated_yaw_rate(0.3, 1.0, 1.3, 4.0),
        coordinated_yaw_rate(0.3, 4.0, 1.3, 4.0),
        "slower than the floor it stays at the floor"
    );
    assert_eq!(
        coordinated_yaw_rate(2.0, 10.0, 0.5, 4.0),
        coordinated_yaw_rate(0.5, 10.0, 0.5, 4.0),
        "the lean is held to the limit"
    );
}

/// A bike that has been held in a wheelie at 12 m/s and then has the stick held at `stick` for 3 s with the lever back; the heading it
/// turned through (rad, positive to the right), whether the front wheel stayed up, the greatest lean, and whether it fell.
fn turned(stick: f32) -> (f32, bool, f32, bool) {
    let ground = FlatGround::new(0.0);
    let mut spec = BikeSpec::example();
    spec.engine.peak_torque *= 2.2;
    let mut bike = Bike::new(spec);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 12.0));
    bike.set_weight(-1.0);
    let yaw = |bike: &Bike| (bike.orientation() * Vec3::Z).x.atan2((bike.orientation() * Vec3::Z).z);
    let mut start = None;
    let (mut up, mut lean, mut counted) = (0.0_f32, 0.0_f32, 0.0_f32);
    for step in 0..(8.0 / FIXED_STEP) as usize {
        let t = step as f32 * FIXED_STEP;
        let held = start.is_some();
        let input = BikeInput { throttle: 1.0, steer: if held { stick } else { 0.0 }, ..Default::default() };
        bike.step(FIXED_STEP, &input, &ground);
        if bike.fallen() {
            return (0.0, false, lean, true);
        }
        if start.is_none() && t > 2.0 && bike.pitch() > 0.4 {
            start = Some((t, yaw(&bike)));
        }
        let Some((begin, _)) = start else { continue };
        counted += FIXED_STEP;
        up += f32::from(u8::from(!bike.wheel(FRONT).on_ground)) * FIXED_STEP;
        lean = lean.max(bike.lean().abs());
        if t - begin >= 3.0 {
            break;
        }
    }
    let Some((_, from)) = start else { return (0.0, false, lean, false) };
    (yaw(&bike) - from, up > 0.9 * counted, lean, false)
}

#[test]
fn a_wheelie_turns_with_the_stick_to_either_side_and_stays_up() {
    let (idle, idle_up, _, idle_fell) = turned(0.0);
    assert!(idle_up && !idle_fell, "the wheelie is held without a stick");
    assert!(idle.abs() < 0.05, "no stick, no turn: {idle}");
    for stick in [1.0_f32, -1.0, 0.5, -0.5] {
        let (heading, up, lean, fell) = turned(stick);
        assert!(!fell, "stick {stick}");
        assert!(up, "the front wheel stays up with the stick at {stick}");
        // The turn is the sign of the stick and as large as a leaned bike on its rear wheel turns: 15 degrees in 3 s at a half stick.
        assert!(heading * stick > 0.0, "stick {stick} turned {heading}");
        let least = 0.26 * stick.abs();
        assert!(heading.abs() > least, "stick {stick} turned only {heading} rad ({least} wanted, lean {lean})");
    }
}

#[test]
fn the_lean_help_has_the_whole_factor_while_the_front_wheel_is_up() {
    let (mut bike, ground) = {
        let ground = FlatGround::new(0.0);
        let mut spec = BikeSpec::example();
        spec.engine.peak_torque *= 2.2;
        let mut bike = Bike::new(spec);
        assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 12.0));
        (bike, ground)
    };
    bike.set_weight(-1.0);
    for _ in 0..(3.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, steer: 0.5, ..Default::default() }, &ground);
    }
    assert!(!bike.wheel(FRONT).on_ground && bike.wheel(REAR).on_ground);
    // The couple the help applies in the wheelie is the whole of what the controller wants of the front tyre, not a tenth: the lean
    // follows its target to within a few hundredths of a radian.
    assert!((bike.lean() - bike.target_lean()).abs() < 0.05, "lean {} target {}", bike.lean(), bike.target_lean());
}

#[test]
fn the_front_wheel_of_a_wheelie_reports_how_high_it_is() {
    let ground = FlatGround::new(0.0);
    let mut spec = BikeSpec::example();
    spec.engine.peak_torque *= 2.2;
    let mut bike = Bike::new(spec);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 12.0));
    assert_eq!(bike.wheel(FRONT).gap, 0.0);
    bike.set_weight(-1.0);
    for _ in 0..(3.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, ..Default::default() }, &ground);
    }
    let gap = bike.wheel(FRONT).gap;
    assert!(!bike.wheel(FRONT).on_ground && gap > 0.2 && gap < 1.5, "gap {gap}");
    assert_eq!(bike.wheel(REAR).gap, 0.0);
}

#[test]
fn the_front_wheel_of_a_wheelie_is_steered_by_the_mid_air_steer_from_the_stick() {
    let ground = FlatGround::new(0.0);
    let mut spec = BikeSpec::example();
    spec.engine.peak_torque *= 2.2;
    let lock = spec.steering.max_angle;
    let mut bike = Bike::new(spec);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 12.0));
    bike.set_weight(-1.0);
    for _ in 0..(3.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, ..Default::default() }, &ground);
    }
    assert!(!bike.wheel(FRONT).on_ground);
    // A small stick: the lean stays under 40 percent of its limit, and the bar goes to its share of the lock against the stick.
    for _ in 0..(0.6 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, steer: 0.1, ..Default::default() }, &ground);
    }
    let wanted = mid_air_angle(0.1, lock);
    assert!(wanted < 0.0 && bike.steer_angle() < 0.0, "the bar is turned against the stick: {}", bike.steer_angle());
}

/// A wheelie held at 12 m/s with the lever back (the pitch settles in 1 s once it is past 23 degrees), then the stick held at `stick` for 3 s from 1.3 s on,
/// the arcade steering's `wheelie_steer_drop` as given: the pitch before the stick (rad, the mean of 0.3 s), the lowest pitch with the stick
/// held, whether the front wheel stayed up the whole time, and whether the bike fell.
/// [`steered_wheelie`] with the lever (`manual`) or with the throttle alone, the game's throttle wheelie (the lever never touched).
fn steered_wheelie_with(stick: f32, drop: Option<f32>, manual: bool) -> (f32, f32, bool, bool) {
    let ground = FlatGround::new(0.0);
    let mut spec = BikeSpec::example();
    spec.engine.peak_torque *= 2.2;
    spec.steering.arcade =
        crate::bike::ArcadeSteering { wheelie_steer_drop: drop, ..crate::bike::ArcadeSteering::REFERENCE };
    let mut bike = Bike::new(spec);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, if manual { 12.0 } else { 0.0 }));
    bike.set_weight(if manual { -1.0 } else { 0.0 });
    if !manual {
        bike.set_throttle_wheelie(crate::bike::ThrottleWheelie::GAME);
    }
    let (mut settled, mut samples, mut lowest, mut up) = (None, 0.0_f32, f32::MAX, true);
    let mut begin = None;
    for step in 0..(9.0 / FIXED_STEP) as usize {
        let t = step as f32 * FIXED_STEP;
        let input = BikeInput {
            throttle: 1.0,
            steer: if begin.is_some_and(|start| t - start >= 1.3) { stick } else { 0.0 },
            ..Default::default()
        };
        bike.step(FIXED_STEP, &input, &ground);
        if bike.fallen() {
            return (settled.unwrap_or(0.0), lowest, false, true);
        }
        let Some(start) = begin else {
            if t > 2.0 && bike.pitch() > if manual { 0.4 } else { 0.3 } {
                begin = Some(t);
            }
            continue;
        };
        // The first second the wheelie settles; the mean of the next 0.3 s is where the stick finds it.
        if t - start >= 1.0 && t - start < 1.3 {
            settled = Some(settled.unwrap_or(0.0) + bike.pitch());
            samples += 1.0;
        }
        if t - start >= 1.3 {
            lowest = lowest.min(bike.pitch());
            up &= !bike.wheel(FRONT).on_ground;
        }
        if t - start >= 4.3 {
            break;
        }
    }
    (settled.map_or(0.0, |sum| sum / samples.max(1.0)), lowest, up, false)
}

#[test]
fn the_stick_lowers_the_front_of_a_wheelie_only_a_little_and_the_wheelie_stays_up() {
    // The lever's wheelie and the throttle's own, the stick at 1, -1 and 0.5 for 3 s: at most about 10 degrees lost (the measured loss is printed).
    for manual in [true, false] {
        for stick in [1.0_f32, -1.0, 0.5] {
            let (before, lowest, up, fell) = steered_wheelie_with(stick, Some(7.0_f32.to_radians()), manual);
            println!("manual {manual} stick {stick}: {:.1} to {:.1} degrees", before.to_degrees(), lowest.to_degrees());
            assert!(!fell, "stick {stick}");
            assert!(before > 0.3, "the wheelie was up and settled: {:.1} degrees", before.to_degrees());
            let lost = (before - lowest).to_degrees();
            assert!(
                lost < 12.0,
                "manual {manual} stick {stick}: {lost:.1} degrees lost, from {:.1}",
                before.to_degrees()
            );
            assert!(up, "the front wheel stays up with the stick at {stick} (manual {manual})");
        }
    }
}

#[test]
fn the_steered_wheelie_with_the_floor_is_never_lower_than_with_the_data_assist_and_the_reference_has_none() {
    for manual in [true, false] {
        let (_, with, with_up, _) = steered_wheelie_with(1.0, Some(7.0_f32.to_radians()), manual);
        let (_, without, _, _) = steered_wheelie_with(1.0, None, manual);
        assert!(
            with >= without - 1.5_f32.to_radians(),
            "manual {manual}: with the floor {:.1} degrees, the data's assist {:.1}",
            with.to_degrees(),
            without.to_degrees()
        );
        assert!(with_up);
    }
    assert_eq!(crate::bike::ArcadeSteering::REFERENCE.wheelie_steer_drop, None);
}
