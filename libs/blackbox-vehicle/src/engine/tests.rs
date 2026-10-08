use super::*;

fn spec() -> EngineSpec {
    EngineSpec {
        torque: vec![100.0, 200.0, 300.0, 250.0],
        idle: 1000.0,
        red_line: 6000.0,
        max_rpm: 7000.0,
        flywheel_mass: 20.0,
        engine_braking: vec![0.2],
        speed_limiter: [0.0, 0.0],
    }
}

#[test]
fn torque_curve_is_clamped_and_interpolated() {
    let e = spec();
    assert_eq!(e.torque_ftlb(0.0), 100.0);
    assert!((e.torque_ftlb(3000.0) - 300.0 * 0.0 - 250.0).abs() > 0.0);
    // Samples sit at 1000, 3000, 5000, 7000 rpm.
    assert!((e.torque_ftlb(2000.0) - 150.0).abs() < 1e-3);
    assert!((e.torque_ftlb(5000.0) - 300.0).abs() < 1e-3);
    // Past the red line the lookup is clamped, so the tail of the curve is never reached.
    assert_eq!(e.torque_ftlb(9000.0), e.torque_ftlb(6000.0));
    assert!((e.torque_ftlb(6000.0) - 275.0).abs() < 1e-3);
}

#[test]
fn single_sample_curve_gives_no_torque() {
    let mut e = spec();
    e.torque = vec![100.0];
    assert_eq!(e.torque_ftlb(3000.0), 0.0);
}

#[test]
fn engine_braking_opposes_torque() {
    let e = spec();
    assert!((e.braking_torque(400.0, 3000.0) + 80.0).abs() < 1e-4);
    let mut curved = spec();
    curved.engine_braking = vec![0.0, 1.0];
    assert!((curved.braking_torque(100.0, 7000.0) + 100.0 * 5.0 / 6.0).abs() < 1.0);
}

#[test]
fn neutral_engine_is_lighter() {
    let e = spec();
    assert!((e.inertia(false) - 0.75).abs() < 1e-6);
    assert!((e.inertia(true) - 0.2625).abs() < 1e-6);
}
