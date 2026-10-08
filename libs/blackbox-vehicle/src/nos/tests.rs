use super::*;
use crate::drivetrain::GEAR_FIRST;

fn spec() -> NosSpec {
    NosSpec {
        nos_capacity: 5.0,
        torque_boost: 0.5,
        nos_disengage: 0.5,
        recharge_min: 20.0,
        recharge_max: 5.0,
        recharge_min_speed: 20.0,
        recharge_max_speed: 100.0,
    }
}

fn input(held: bool, speed_mph: f32) -> NosInput {
    NosInput { dt: 1.0 / 60.0, held, gear: GEAR_FIRST + 1, throttle: 1.0, speed_mph, blown: false, tuning: 0.0 }
}

#[test]
fn no_nos_without_capacity() {
    let none = NosSpec::default();
    let mut n = Nos::new(&none);
    n.update(&none, &input(true, 50.0));
    assert_eq!(n.boost, 1.0);
    assert_eq!(n.capacity, 0.0);
}

#[test]
fn burns_and_boosts_above_minimum_speed() {
    let s = spec();
    let mut n = Nos::new(&s);
    for _ in 0..60 {
        n.update(&s, &input(true, 50.0));
    }
    assert!((n.boost - 1.5).abs() < 1e-6);
    assert!((n.capacity - 0.8).abs() < 0.01, "tank = {}", n.capacity);
    assert!(n.is_engaged());
}

#[test]
fn does_not_start_below_ten_mph() {
    let s = spec();
    let mut n = Nos::new(&s);
    n.update(&s, &input(true, 8.0));
    assert_eq!(n.boost, 1.0);
}

#[test]
fn empties_then_recharges_after_the_disengage_delay() {
    let s = spec();
    let mut n = Nos::new(&s);
    for _ in 0..(6 * 60) {
        n.update(&s, &input(true, 60.0));
    }
    assert_eq!(n.capacity, 0.0);
    assert_eq!(n.boost, 1.0);
    for _ in 0..30 {
        n.update(&s, &input(false, 60.0));
    }
    assert_eq!(n.engaged, 0.0);
    let before = n.capacity;
    for _ in 0..60 {
        n.update(&s, &input(false, 60.0));
    }
    assert!(n.capacity > before);
}
