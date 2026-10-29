//! Wheelies and stoppies of the whole bike on flat ground: the front wheel really lifts under power with the rider's weight
//! back, the help holds the angle the lever and the throttle ask for, the rider help cuts the power and moves the weight
//! before the bike flips, a stoppie lifts the rear under the front brake, and the bike settles again afterwards.

use super::{Bike, BikeInput, BikeSpec, FRONT, REAR};
use crate::FIXED_STEP;
use crate::ground::FlatGround;

fn bike_at(spec: BikeSpec, speed: f32) -> (Bike, FlatGround) {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
    (bike, ground)
}

/// Run for `seconds` and return the pitch (rad) at every step.
fn pitches(bike: &mut Bike, ground: &FlatGround, seconds: f32, input: &BikeInput) -> Vec<f32> {
    (0..(seconds / FIXED_STEP) as usize)
        .map(|_| {
            bike.step(FIXED_STEP, input, ground);
            bike.pitch()
        })
        .collect()
}

/// A bike at `speed` with the rider's ABS level set: the reference scales the ABS of both wheels by that level
/// (`SETTINGS_ABS`) and keeps the ABS working through a stoppie (it has no pitch term), so a stoppie asks for it low.
fn bike_with_abs(level: f32, speed: f32) -> (Bike, FlatGround) {
    let (mut bike, ground) = bike_at(BikeSpec::example(), speed);
    bike.set_abs_level(level);
    (bike, ground)
}

fn throttle() -> BikeInput {
    BikeInput { throttle: 1.0, ..Default::default() }
}

fn tail(values: &[f32], seconds: f32) -> &[f32] {
    &values[values.len() - (seconds / FIXED_STEP) as usize..]
}

#[test]
fn full_throttle_with_the_weight_back_lifts_the_front_wheel_and_the_help_holds_the_angle() {
    let (mut bike, ground) = bike_at(BikeSpec::example(), 6.0);
    bike.set_weight(-1.0);
    let target = bike.spec().wheelie.as_ref().unwrap().wheelie.max_angle;
    let pitch = pitches(&mut bike, &ground, 5.0, &throttle());
    assert!(!bike.fallen());
    assert!(!bike.wheel(FRONT).on_ground && bike.wheel(REAR).on_ground, "the front wheel is up");
    let settled = tail(&pitch, 1.5);
    let (low, high) = settled.iter().fold((f32::MAX, f32::MIN), |(l, h), &p| (l.min(p), h.max(p)));
    assert!(low > target - 0.15 && high < target + 0.15, "pitch {low} .. {high} for a target {target}");
}

#[test]
fn a_smaller_throttle_holds_a_lower_wheelie() {
    let highest = |throttle: f32| {
        let (mut bike, ground) = bike_at(BikeSpec::example(), 6.0);
        bike.set_weight(-1.0);
        let pitch = pitches(&mut bike, &ground, 2.5, &BikeInput { throttle, ..Default::default() });
        pitch.iter().cloned().fold(f32::MIN, f32::max)
    };
    let (full, part) = (highest(1.0), highest(0.7));
    assert!(part > 0.05, "a part throttle still lifts the front: {part}");
    assert!(part < full - 0.05, "{part} against {full}");
}

#[test]
fn the_wheelie_never_goes_over_and_the_bike_keeps_its_rider() {
    let (mut bike, ground) = bike_at(BikeSpec::example(), 5.0);
    bike.set_weight(-1.0);
    let fall = bike.spec().fall.pitch;
    let pitch = pitches(&mut bike, &ground, 12.0, &throttle());
    let worst = pitch.iter().cloned().fold(f32::MIN, f32::max);
    assert!(worst < fall - 0.1, "pitch {worst} against the fall limit {fall}");
    assert!(!bike.fallen());
}

#[test]
fn without_the_weight_back_a_hard_start_stays_down() {
    let (mut bike, ground) = bike_at(BikeSpec::example(), 0.0);
    let pitch = pitches(&mut bike, &ground, 6.0, &throttle());
    let worst = pitch.iter().cloned().fold(f32::MIN, f32::max);
    assert!(worst < 0.5, "pitch {worst}");
    assert!(!bike.fallen());
}

#[test]
fn a_bike_with_no_wheelie_control_has_no_help_to_hold_it() {
    let mut spec = BikeSpec::example();
    spec.wheelie = None;
    let (mut bike, ground) = bike_at(spec, 6.0);
    bike.set_weight(-1.0);
    let held = {
        let (mut with, ground) = bike_at(BikeSpec::example(), 6.0);
        with.set_weight(-1.0);
        pitches(&mut with, &ground, 4.0, &throttle())
    };
    let free = pitches(&mut bike, &ground, 4.0, &throttle());
    // Without the control the rider help does not cut the power and no torque holds the angle: it rises past the held one.
    let free_worst = free.iter().cloned().fold(f32::MIN, f32::max);
    let held_worst = held.iter().cloned().fold(f32::MIN, f32::max);
    assert!(free_worst > held_worst + 0.05 || bike.fallen(), "{free_worst} {held_worst}");
}

#[test]
fn the_front_brake_with_the_weight_forward_lifts_the_rear_wheel_and_the_help_holds_the_stoppie() {
    let (mut bike, ground) = bike_with_abs(0.0, 22.0);
    bike.set_weight(1.0);
    let input = BikeInput { front_brake: 1.0, ..Default::default() };
    let pitch = pitches(&mut bike, &ground, 1.6, &input);
    let lowest = pitch.iter().cloned().fold(f32::MAX, f32::min);
    assert!(lowest < -0.15, "the nose goes down: {lowest}");
    assert!(lowest > -bike.spec().wheelie.as_ref().unwrap().stoppie.max_angle - 0.2, "not over the bars: {lowest}");
    assert!(!bike.fallen());
}

#[test]
fn the_abs_stays_on_in_a_stoppie_and_with_it_the_rear_wheel_does_not_lift() {
    // The reference's ABS has no term for the pitch: with its level up it limits the front brake through a stoppie too.
    let lowest = |level: f32| {
        let (mut bike, ground) = bike_with_abs(level, 22.0);
        bike.set_weight(1.0);
        let pitch = pitches(&mut bike, &ground, 1.6, &BikeInput { front_brake: 1.0, ..Default::default() });
        pitch.iter().cloned().fold(f32::MAX, f32::min)
    };
    let (on, off) = (lowest(1.0), lowest(0.0));
    assert!(on > off + 0.08, "ABS on {on}, off {off}");
    assert!(on > -0.15, "{on}");
}

#[test]
fn a_stoppie_ends_with_the_bike_upright_and_both_wheels_down() {
    let (mut bike, ground) = bike_with_abs(0.0, 22.0);
    bike.set_weight(1.0);
    pitches(&mut bike, &ground, 1.2, &BikeInput { front_brake: 1.0, ..Default::default() });
    bike.set_weight(0.0);
    pitches(&mut bike, &ground, 6.0, &BikeInput { front_brake: 0.3, ..Default::default() });
    assert!(!bike.fallen());
    assert!(bike.pitch().abs() < 0.1, "{}", bike.pitch());
    assert_eq!(bike.wheels_on_ground(), 2);
}

#[test]
fn a_wheelie_comes_down_when_the_throttle_and_the_weight_are_released() {
    let (mut bike, ground) = bike_at(BikeSpec::example(), 6.0);
    bike.set_weight(-1.0);
    pitches(&mut bike, &ground, 3.0, &throttle());
    assert!(!bike.wheel(FRONT).on_ground);
    bike.set_weight(0.0);
    let after = pitches(&mut bike, &ground, 4.0, &BikeInput::default());
    assert!(!bike.fallen());
    assert!(after.iter().cloned().fold(f32::MIN, f32::max) < 0.8);
    assert!(bike.pitch().abs() < 0.1 && bike.wheels_on_ground() == 2, "{}", bike.pitch());
}

#[test]
fn the_auto_rider_pulls_the_weight_forward_while_the_front_is_up_and_the_player_leaves_it() {
    let (mut bike, ground) = bike_at(BikeSpec::example(), 6.0);
    bike.set_weight(-1.0);
    pitches(&mut bike, &ground, 3.0, &throttle());
    assert!(bike.rider_weight() < -0.8, "the player holds the weight back: {}", bike.rider_weight());
    // Inside the dead zone the same lift moves the rider forward.
    bike.set_weight(-0.4);
    pitches(&mut bike, &ground, 1.0, &throttle());
    assert!(bike.rider_weight() > 0.3, "{}", bike.rider_weight());
}

#[test]
fn the_wheelie_and_stoppie_assist_needs_the_speed_and_the_lean_room() {
    // Leaned right over, the assist lets go: the front does not hold at the angle.
    let (mut bike, ground) = bike_at(BikeSpec::example(), 6.0);
    bike.set_weight(-1.0);
    let leaned = BikeInput { throttle: 1.0, steer: 0.9, ..Default::default() };
    pitches(&mut bike, &ground, 1.0, &leaned);
    let (mut straight, ground2) = bike_at(BikeSpec::example(), 6.0);
    straight.set_weight(-1.0);
    pitches(&mut straight, &ground2, 1.0, &throttle());
    assert!(!bike.fallen());
    assert!(bike.pitch() <= straight.pitch() + 0.3);
}

#[test]
fn the_riders_abs_level_starts_full_and_snaps_to_its_steps() {
    let (mut bike, _) = bike_at(BikeSpec::example(), 0.0);
    assert_eq!(bike.abs_level(), 1.0);
    bike.set_abs_level(0.6);
    assert_eq!(bike.abs_level(), 0.5);
    bike.set_abs_level(f32::NAN);
    assert_eq!(bike.abs_level(), 1.0);
    bike.set_abs_level(-3.0);
    assert_eq!(bike.abs_level(), 0.0);
    // A rider's setting outlives a new placement.
    let ground = FlatGround::new(0.0);
    assert!(bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
    assert_eq!(bike.abs_level(), 0.0);
}

#[test]
fn with_the_abs_level_at_zero_the_brake_is_no_longer_limited_by_the_anti_lock() {
    let slowed = |level: f32| {
        let (mut bike, ground) = bike_with_abs(level, 25.0);
        pitches(&mut bike, &ground, 1.0, &BikeInput { front_brake: 1.0, rear_brake: 1.0, ..Default::default() });
        25.0 - bike.forward_speed()
    };
    let (full, half, off) = (slowed(1.0), slowed(0.5), slowed(0.0));
    assert!(off > full + 1.0, "level 0 brakes harder: {off} against {full}");
    assert!(half >= full - 0.1 && half <= off + 0.1, "{full} {half} {off}");
}

#[test]
fn the_rear_anti_lock_is_off_at_the_low_levels_and_the_front_keeps_a_tenth() {
    let spec = BikeSpec::example();
    let base = spec.brakes.abs;
    let low = spec.abs_level.apply(&base, 0.25);
    assert_eq!((low.rear_intensity, low.front_intensity), (0.0, base.front_intensity * 0.2));
    let zero = spec.abs_level.apply(&base, 0.0);
    assert!((zero.front_intensity - 0.1 * base.front_intensity).abs() < 1e-6);
}
