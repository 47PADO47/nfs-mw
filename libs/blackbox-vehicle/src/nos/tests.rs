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
    NosInput {
        dt: 1.0 / 60.0,
        held,
        gear: GEAR_FIRST + 1,
        throttle: 1.0,
        speed_mph,
        blown: false,
        tuning: 0.0,
        can_recharge: true,
    }
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

#[test]
fn recharge_uses_speed_curve_without_a_stunt_bonus() {
    let s = spec();
    for (speed, seconds) in [(20.0, 20.0), (60.0, 12.5), (100.0, 5.0)] {
        let mut n = Nos { capacity: 0.5, engaged: 0.0, boost: 1.0 };
        n.update(&s, &NosInput { dt: 1.0, ..input(false, speed) });
        assert!((n.capacity - (0.5 + 1.0 / seconds)).abs() < 1e-6);
    }
}

#[test]
fn recharge_permission_does_not_prevent_burning_or_cooldown() {
    let s = spec();
    let mut n = Nos { capacity: 0.5, engaged: 0.0, boost: 1.0 };
    let denied = NosInput { can_recharge: false, ..input(false, 60.0) };
    n.update(&s, &denied);
    assert_eq!(n.capacity, 0.5);
    n.update(&s, &NosInput { held: true, ..denied });
    assert!(n.capacity < 0.5);
    assert_eq!(n.boost, 1.5);
    let before = n.capacity;
    n.update(&s, &denied);
    assert_eq!(n.capacity, before);
    assert!(n.engaged > 0.0 && n.engaged < 1.0);
    assert_eq!(n.boost, 1.0);
}

#[test]
fn restarting_during_cooldown_requires_ten_mph() {
    let s = spec();
    let mut n = Nos::new(&s);
    n.update(&s, &input(true, 20.0));
    // Fully engaged nitrous can continue at eight mph.
    n.update(&s, &input(true, 8.0));
    assert_eq!(n.boost, 1.5);
    n.update(&s, &input(false, 8.0));
    let before = n.capacity;
    assert!(n.engaged > 0.0 && n.engaged < 1.0);
    // Its residual disengage timer is not fully engaged nitrous.
    n.update(&s, &input(true, 8.0));
    assert_eq!(n.capacity, before);
    assert_eq!(n.boost, 1.0);
    n.update(&s, &input(true, 10.0));
    assert!(n.capacity < before);
    assert_eq!(n.boost, 1.5);
}

#[test]
fn matches_original_pc_nitrous_lifecycle_vectors() {
    // Synthetic vectors replayed through the original PC nitrous routine.
    // docs/provenance/vehicle-engine-drivetrain.md records the comparison scope.
    let s = spec();
    let base = NosInput { dt: 0.1, ..input(false, 60.0) };
    let cases = [
        ("recharge min", NosInput { speed_mph: 20.0, ..base }, 0.0, [0.505, 0.0, 1.0]),
        ("recharge middle", base, 0.0, [0.508, 0.0, 1.0]),
        ("recharge max", NosInput { speed_mph: 100.0, ..base }, 0.0, [0.52, 0.0, 1.0]),
        ("recharge denied", NosInput { can_recharge: false, ..base }, 0.0, [0.5, 0.0, 1.0]),
        ("burn", NosInput { held: true, can_recharge: false, ..base }, 0.0, [0.48, 1.0, 1.5]),
        ("continue", NosInput { held: true, speed_mph: 8.0, ..base }, 1.0, [0.48, 1.0, 1.5]),
        ("restart cooldown", NosInput { held: true, speed_mph: 8.0, ..base }, 0.9, [0.5, 0.7, 1.0]),
        ("restart ten", NosInput { held: true, speed_mph: 10.0, ..base }, 0.9, [0.48, 1.0, 1.5]),
        ("cooldown", base, 1.0, [0.5, 0.8, 1.0]),
        ("tuning power", NosInput { held: true, tuning: 1.0, ..base }, 0.0, [0.5 - 0.1 / 3.75, 1.0, 1.625]),
        ("tuning duration", NosInput { held: true, tuning: -1.0, ..base }, 0.0, [0.5 - 0.1 / 6.25, 1.0, 1.375]),
        ("no throttle", NosInput { held: true, throttle: 0.0, ..base }, 0.0, [0.508, 0.0, 1.0]),
        ("blown", NosInput { held: true, blown: true, ..base }, 0.0, [0.508, 0.0, 1.0]),
        ("reverse", NosInput { held: true, gear: 0, ..base }, 0.0, [0.5, 0.0, 1.0]),
    ];
    for (name, input, engaged, expected) in cases {
        let mut n = Nos { capacity: 0.5, engaged, boost: 1.0 };
        n.update(&s, &input);
        for (actual, wanted) in [n.capacity, n.engaged, n.boost].into_iter().zip(expected) {
            assert!((actual - wanted).abs() < 1e-6, "{name}: {actual} != {wanted}");
        }
    }
}
