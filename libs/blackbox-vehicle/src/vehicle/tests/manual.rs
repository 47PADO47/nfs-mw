//! Manual shifting on flat ground (spec: `docs/specs/vehicle-manual-shifting.md`).

use super::*;
use crate::drivetrain::{GEAR_FIRST, GEAR_NEUTRAL, GEAR_REVERSE};

const RED_LINE: f32 = 7000.0;

fn manual() -> Vehicle {
    let mut v = parked();
    v.config.automatic = false;
    v
}

fn gas(throttle: f32) -> InputState {
    InputState { throttle, ..Default::default() }
}

/// One step with a shift request, the gas held.
fn shift(v: &mut Vehicle, up: bool, throttle: f32) {
    let input = InputState { throttle, shift_up: up, shift_down: !up, ..Default::default() };
    v.step(FIXED_STEP, &input, &flat());
}

/// Climbs by hand: full throttle, an upshift at 6500 rpm, until `gear`.
fn climb_to(v: &mut Vehicle, gear: usize) {
    for _ in 0..(60.0 / FIXED_STEP) as usize {
        if v.gear() >= gear {
            return;
        }
        let up = v.rpm() > 6500.0 && !v.powertrain().shifting();
        let input = InputState { throttle: 1.0, shift_up: up, ..Default::default() };
        v.step(FIXED_STEP, &input, &flat());
    }
    panic!("never reached gear {gear}, in {}", v.gear());
}

#[test]
fn nothing_shifts_by_itself() {
    let mut v = manual();
    let g = flat();
    let mut top_speed = 0.0_f32;
    for _ in 0..(8.0 / FIXED_STEP) as usize {
        v.step(FIXED_STEP, &gas(1.0), &g);
        top_speed = top_speed.max(v.forward_speed());
        assert_eq!(v.gear(), GEAR_FIRST, "the box shifted on its own");
        assert!(v.rpm() <= RED_LINE + 1.0, "rpm {}", v.rpm());
    }
    // First gear stops pulling at the red line: speed saturates near the red-line speed of the gear. (Held
    // there for a minute the car still creeps up by a few percent per ten seconds: the tire model's
    // wheel-spin reaction pushes while the limiter pins the wheels; see the spec's open questions.)
    let limit = v.powertrain().speed_at(RED_LINE, GEAR_FIRST);
    assert!(top_speed > 0.8 * limit && top_speed < 1.1 * limit, "{top_speed} m/s against {limit}");
    assert!(v.rpm() > RED_LINE - 300.0, "the tachometer sits at the limiter: {}", v.rpm());
}

#[test]
fn the_limiter_cuts_drive_in_every_gear() {
    let mut v = manual();
    climb_to(&mut v, GEAR_FIRST + 2);
    run(&mut v, &gas(1.0), 20.0);
    assert_eq!(v.gear(), GEAR_FIRST + 2);
    assert!(v.rpm() <= RED_LINE + 1.0 && v.rpm() > RED_LINE - 300.0, "rpm {}", v.rpm());
    let before = v.forward_speed();
    run(&mut v, &gas(1.0), 2.0);
    assert!((v.forward_speed() - before).abs() < 1.0, "still accelerating: {before} to {}", v.forward_speed());
}

#[test]
fn shift_requests_step_one_gear_and_stop_at_the_ends() {
    let mut v = manual();
    shift(&mut v, true, 0.0);
    assert_eq!(v.gear(), GEAR_FIRST + 1);
    shift(&mut v, false, 0.0);
    assert_eq!(v.gear(), GEAR_FIRST);
    // Down from first is neutral, and neutral is as far down as the buttons go.
    shift(&mut v, false, 0.0);
    assert_eq!(v.gear(), GEAR_NEUTRAL);
    shift(&mut v, false, 0.0);
    assert_eq!(v.gear(), GEAR_NEUTRAL);
    let top = v.powertrain().top_gear();
    for _ in 0..top + 2 {
        shift(&mut v, true, 0.0);
    }
    assert_eq!(v.gear(), top, "no gear above the top one");
}

#[test]
fn neutral_stays_neutral_and_free_revs_to_the_limiter() {
    let mut v = manual();
    shift(&mut v, false, 0.0);
    assert_eq!(v.gear(), GEAR_NEUTRAL);
    run(&mut v, &gas(0.0), 3.0);
    assert_eq!(v.gear(), GEAR_NEUTRAL, "a manual box does not leave neutral by itself");
    run(&mut v, &gas(1.0), 3.0);
    assert!(v.rpm() > RED_LINE - 300.0 && v.rpm() <= RED_LINE + 1.0, "rpm {}", v.rpm());
    assert!(v.forward_speed().abs() < 0.5, "no drive in neutral: {}", v.forward_speed());
    shift(&mut v, true, 1.0);
    assert_eq!(v.gear(), GEAR_FIRST);
}

#[test]
fn a_second_request_during_a_shift_is_accepted() {
    let mut v = manual();
    shift(&mut v, true, 1.0);
    assert!(v.powertrain().shifting());
    shift(&mut v, true, 1.0);
    assert_eq!(v.gear(), GEAR_FIRST + 2, "manual mode has no lock-out while shifting");
}

#[test]
fn the_shift_buttons_do_nothing_in_reverse() {
    let mut v = manual();
    run(&mut v, &InputState { brake: 1.0, ..Default::default() }, 2.0);
    assert_eq!(v.gear(), GEAR_REVERSE);
    shift(&mut v, true, 0.0);
    assert_eq!(v.gear(), GEAR_REVERSE);
    shift(&mut v, false, 0.0);
    assert_eq!(v.gear(), GEAR_REVERSE);
}

#[test]
fn braking_to_a_stop_in_a_high_gear_still_engages_reverse() {
    let mut v = manual();
    climb_to(&mut v, GEAR_FIRST + 3);
    run(&mut v, &InputState { brake: 1.0, ..Default::default() }, 8.0);
    assert_eq!(v.gear(), GEAR_REVERSE, "the original puts a stopped car in reverse in manual mode too");
}

#[test]
fn an_over_rev_downshift_is_not_refused_and_slows_the_car() {
    let mut v = manual();
    climb_to(&mut v, GEAR_FIRST + 3);
    run(&mut v, &gas(0.0), 0.5);
    let g = flat();
    let start = v.forward_speed();
    assert!(start > 25.0, "set-up speed {start}");
    // Fourth to first in one go: the new gear's red line is far below the road speed.
    for _ in 0..3 {
        shift(&mut v, false, 0.0);
    }
    assert_eq!(v.gear(), GEAR_FIRST, "the downshifts are not refused");
    let mut last = v.forward_speed();
    for _ in 0..(3.0 / FIXED_STEP) as usize {
        v.step(FIXED_STEP, &gas(0.0), &g);
        let speed = v.forward_speed();
        assert!(speed.is_finite() && v.rpm().is_finite(), "state blew up");
        assert!(v.rpm() <= RED_LINE + 1.0, "rpm {}", v.rpm());
        assert!(speed <= last + 0.05, "sped up from {last} to {speed} under engine braking");
        last = speed;
    }
    let slower = start - v.forward_speed();
    assert!(slower > 5.0, "an over-revved first gear should drag the car down: only {slower} m/s lost");
    assert!(up(&v).y > 0.95, "the car must stay level: {}", up(&v).y);
    assert!(v.position().x.abs() < 1.0, "veered {} m", v.position().x);
}

#[test]
fn a_matched_downshift_is_gentle() {
    let mut v = manual();
    climb_to(&mut v, GEAR_FIRST + 3);
    run(&mut v, &gas(0.0), 3.0);
    let before = v.forward_speed();
    let rpm = v.rpm();
    shift(&mut v, false, 0.0);
    run(&mut v, &gas(0.0), 0.5);
    assert!(v.rpm() < RED_LINE, "rpm {rpm} then {}", v.rpm());
    assert!(before - v.forward_speed() < 3.0, "lost {} m/s", before - v.forward_speed());
}

#[test]
fn held_on_the_limiter_in_the_top_gear_the_car_stays_at_its_top_speed() {
    // A car with little drag, so the red line and not the air is what stops it.
    let mut spec = VehicleSpec::example();
    spec.aero.drag_coefficient = 0.05;
    spec.aero.aero_coefficient = 0.0;
    let mut v = Vehicle::new(spec);
    assert!(v.place_on_ground(&flat(), 0.0, 0.0, 5.0, 0.0));
    v.config.automatic = false;
    let top = v.powertrain().top_gear();
    climb_to(&mut v, top);
    run(&mut v, &gas(1.0), 60.0);
    let max = v.powertrain().max_speed();
    assert_eq!(v.gear(), top);
    assert!(v.rpm() > RED_LINE - 100.0 && v.rpm() <= RED_LINE + 1.0, "rpm {}", v.rpm());
    assert!((v.forward_speed() - max).abs() < 0.03 * max, "{} m/s against the limit {max}", v.forward_speed());
}

#[test]
fn automatic_mode_still_shifts_by_itself() {
    let mut v = parked();
    assert!(v.config.automatic);
    run(&mut v, &gas(1.0), 6.0);
    assert!(v.gear() > GEAR_FIRST, "the automatic box shifts up by itself");
}
