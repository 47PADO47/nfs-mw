//! Steering wheel controls: gears by number (an H-pattern shifter) and the clutch pedal (spec:
//! `docs/specs/vehicle-manual-shifting.md`, section 6).

use super::*;
use crate::drivetrain::{GEAR_FIRST, GEAR_NEUTRAL, GEAR_REVERSE};

const RED_LINE: f32 = 7000.0;

fn manual() -> Vehicle {
    let mut v = parked();
    v.config.automatic = false;
    v
}

/// One step asking for a gear by number, the gas held.
fn select(v: &mut Vehicle, gear: usize, throttle: f32) {
    let input = InputState { throttle, gear_select: Some(gear), ..Default::default() };
    v.step(FIXED_STEP, &input, &flat());
}

#[test]
fn a_gear_by_number_skips_the_gears_in_between() {
    let mut v = manual();
    select(&mut v, GEAR_FIRST + 3, 0.0);
    assert_eq!(v.gear(), GEAR_FIRST + 3);
    select(&mut v, GEAR_NEUTRAL, 0.0);
    assert_eq!(v.gear(), GEAR_NEUTRAL);
    select(&mut v, GEAR_FIRST + 1, 0.0);
    assert_eq!(v.gear(), GEAR_FIRST + 1);
}

#[test]
fn a_gear_by_number_is_ignored_by_the_automatic_box() {
    let mut v = parked();
    select(&mut v, GEAR_FIRST + 3, 0.0);
    assert_eq!(v.gear(), GEAR_FIRST, "the automatic box shifts for itself");
}

#[test]
fn reverse_by_number_waits_until_the_car_has_stopped() {
    let mut v = manual();
    run(&mut v, &InputState { throttle: 1.0, ..Default::default() }, 2.0);
    assert!(v.forward_speed() > 5.0, "set-up speed {}", v.forward_speed());
    select(&mut v, GEAR_REVERSE, 0.0);
    assert_ne!(v.gear(), GEAR_REVERSE, "no reverse at {} m/s", v.forward_speed());

    let mut stopped = manual();
    select(&mut stopped, GEAR_REVERSE, 0.0);
    assert_eq!(stopped.gear(), GEAR_REVERSE);
    select(&mut stopped, GEAR_FIRST, 0.0);
    assert_eq!(stopped.gear(), GEAR_FIRST, "a gear by number leaves reverse, which the shift buttons cannot");
}

#[test]
fn an_h_shifter_in_reverse_backs_up_on_the_gas_pedal() {
    let mut v = manual();
    v.config.h_shifter = true;
    let hold = |gear, throttle| InputState { throttle, gear_select: Some(gear), ..Default::default() };
    run(&mut v, &hold(GEAR_REVERSE, 1.0), 3.0);
    assert_eq!(v.gear(), GEAR_REVERSE);
    assert!(v.forward_speed() < -2.0, "the gas pedal reverses the car: {}", v.forward_speed());
    // The selector moves to its neutral position: the car is taken out of gear.
    run(&mut v, &hold(GEAR_NEUTRAL, 0.0), 1.0);
    assert_eq!(v.gear(), GEAR_NEUTRAL);
}

#[test]
fn an_h_shifter_keeps_a_braking_car_out_of_reverse() {
    let mut v = manual();
    v.config.h_shifter = true;
    let brake = InputState { brake: 1.0, gear_select: Some(GEAR_FIRST), ..Default::default() };
    run(&mut v, &brake, 3.0);
    assert_eq!(v.gear(), GEAR_FIRST);
    let brake = InputState { brake: 1.0, ..Default::default() };
    run(&mut v, &brake, 3.0);
    assert_eq!(v.gear(), GEAR_FIRST, "with a selector the brake pedal does not pick reverse");
}

#[test]
fn a_pressed_clutch_pedal_lets_the_engine_rev_and_the_car_stand() {
    let mut v = manual();
    v.config.manual_clutch = true;
    let pressed = InputState { throttle: 1.0, clutch: 1.0, ..Default::default() };
    run(&mut v, &pressed, 3.0);
    assert!(v.forward_speed().abs() < 0.5, "no drive with the clutch in: {}", v.forward_speed());
    assert!(v.rpm() > RED_LINE - 500.0, "the engine revs freely: {}", v.rpm());
    let released = InputState { throttle: 1.0, clutch: 0.0, ..Default::default() };
    run(&mut v, &released, 3.0);
    assert!(v.forward_speed() > 4.0, "released, the car pulls away: {}", v.forward_speed());
}

#[test]
fn the_clutch_pedal_is_ignored_unless_it_is_switched_on() {
    let mut v = manual();
    let pressed = InputState { throttle: 1.0, clutch: 1.0, ..Default::default() };
    run(&mut v, &pressed, 3.0);
    assert!(v.forward_speed() > 4.0, "the default clutch is automatic: {}", v.forward_speed());
}

#[test]
fn the_clutch_pedal_also_works_with_the_automatic_box() {
    let mut v = parked();
    v.config.manual_clutch = true;
    let pressed = InputState { throttle: 1.0, clutch: 1.0, ..Default::default() };
    run(&mut v, &pressed, 2.0);
    assert!(v.forward_speed().abs() < 0.5, "{}", v.forward_speed());
}

#[test]
fn hammering_the_gear_selector_and_the_clutch_keeps_the_state_finite() {
    let g = flat();
    let mut v = manual();
    v.config.manual_clutch = true;
    v.config.h_shifter = true;
    v.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), 20.0);
    let top = v.powertrain().top_gear();
    let mut seed = 12345_u32;
    let mut next = move || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    for step in 0..3000 {
        let input = InputState {
            throttle: next(),
            brake: if next() < 0.7 { 0.0 } else { next() },
            steer: next() * 2.0 - 1.0,
            gear_select: (next() < 0.1).then(|| (next() * (top + 3) as f32) as usize),
            clutch: next() * 1.5,
            ..Default::default()
        };
        v.step(FIXED_STEP, &input, &g);
        assert!(v.position().is_finite() && v.rpm().is_finite(), "state blew up at step {step}");
        assert!(v.gear() <= top);
    }
}
