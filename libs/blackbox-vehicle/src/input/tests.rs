use super::*;
use crate::drivetrain::{GEAR_FIRST, GEAR_REVERSE};

fn ctx(speed: f32, gear: usize) -> InputContext {
    InputContext { forward_speed: speed, gear, disabled: false }
}

fn cfg() -> ControlConfig {
    ControlConfig { dead_zone: 0.1, ..ControlConfig::default() }
}

#[test]
fn dead_zone_snaps_pedals() {
    let i = InputState { throttle: 0.95, brake: 0.05, ..Default::default() };
    let c = shape(&i, &cfg(), &ctx(20.0, GEAR_FIRST));
    assert_eq!((c.gas, c.brake), (1.0, 0.0));
    let mid = shape(&InputState { throttle: 0.5, ..Default::default() }, &cfg(), &ctx(20.0, GEAR_FIRST));
    assert_eq!(mid.gas, 0.5);
}

#[test]
fn brake_pedal_selects_reverse_when_stopped_and_swaps_the_pedals() {
    let i = InputState { brake: 1.0, ..Default::default() };
    let c = shape(&i, &cfg(), &ctx(0.5, GEAR_FIRST));
    assert_eq!(c.gear_request, Some(GearRequest::Reverse));
    assert_eq!((c.gas, c.brake), (1.0, 0.0));
}

#[test]
fn braking_at_speed_does_not_select_reverse() {
    let i = InputState { brake: 1.0, ..Default::default() };
    let c = shape(&i, &cfg(), &ctx(20.0, GEAR_FIRST + 2));
    assert_eq!(c.gear_request, None);
    assert_eq!((c.gas, c.brake), (0.0, 1.0));
}

#[test]
fn gas_in_reverse_returns_to_first() {
    let i = InputState { throttle: 1.0, ..Default::default() };
    let c = shape(&i, &cfg(), &ctx(-1.0, GEAR_REVERSE));
    assert_eq!(c.gear_request, Some(GearRequest::First));
    assert_eq!((c.gas, c.brake), (1.0, 0.0));
}

#[test]
fn idle_auto_brake_fades_with_speed() {
    let none = InputState::default();
    let still = shape(&none, &cfg(), &ctx(0.0, GEAR_FIRST));
    assert_eq!(still.brake, 1.0);
    let rolling = shape(&none, &cfg(), &ctx(4.0, GEAR_FIRST));
    assert!((rolling.brake - 0.625).abs() < 1e-6);
    let fast = shape(&none, &cfg(), &ctx(20.0, GEAR_FIRST + 3));
    assert_eq!(fast.brake, 0.0);
}

#[test]
fn handbrake_clears_the_pedal_brake() {
    let i = InputState { brake: 1.0, handbrake: 0.8, ..Default::default() };
    let c = shape(&i, &cfg(), &ctx(20.0, GEAR_FIRST + 2));
    assert_eq!((c.brake, c.handbrake), (0.0, 0.8));
}

#[test]
fn disabled_cars_stop_dead() {
    let i = InputState { throttle: 1.0, ..Default::default() };
    let mut c = ctx(10.0, GEAR_FIRST);
    c.disabled = true;
    let out = shape(&i, &cfg(), &c);
    assert_eq!((out.gas, out.brake, out.handbrake), (0.0, 1.0, 1.0));
}

#[test]
fn shift_buttons_become_requests_except_in_reverse() {
    let up = InputState { shift_up: true, throttle: 1.0, ..Default::default() };
    assert_eq!(shape(&up, &cfg(), &ctx(10.0, GEAR_FIRST)).gear_request, Some(GearRequest::Shift(1)));
    let down = InputState { shift_down: true, throttle: 1.0, ..Default::default() };
    assert_eq!(shape(&down, &cfg(), &ctx(10.0, GEAR_FIRST + 1)).gear_request, Some(GearRequest::Shift(-1)));
    let in_reverse = InputState { shift_down: true, brake: 1.0, ..Default::default() };
    assert_eq!(shape(&in_reverse, &cfg(), &ctx(-3.0, GEAR_REVERSE)).gear_request, None);
}

#[test]
fn without_auto_reverse_the_pedals_stay_put() {
    let config = ControlConfig { auto_reverse: false, ..cfg() };
    let i = InputState { brake: 1.0, ..Default::default() };
    let c = shape(&i, &config, &ctx(0.0, GEAR_FIRST));
    assert_eq!((c.gear_request, c.gas, c.brake), (None, 0.0, 1.0));
}

#[test]
fn steering_is_passed_through_clamped() {
    let c = shape(&InputState { steer: 3.0, ..Default::default() }, &cfg(), &ctx(0.0, GEAR_FIRST));
    assert_eq!(c.steering, 1.0);
}
