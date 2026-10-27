use super::*;

fn config() -> LaunchConfig {
    LaunchConfig {
        band_fraction: 0.25,
        redline_margin: 500.0,
        duration: 4.5,
        speed_cutoff: 26.8218,
        traction_gain: 0.25,
        staging_traction_scale: 0.25,
        cancel_on_shift: false,
    }
}

fn input(rpm: f32) -> LaunchInput {
    LaunchInput {
        dt: 1.0 / 144.0,
        first_gear: true,
        rpm,
        idle: 1000.0,
        redline: 8000.0,
        peak_torque_rpm: 5000.0,
        speed: 0.0,
        shifting: false,
    }
}

fn staged(rpm: f32) -> LaunchState {
    let mut state = LaunchState::default();
    state.configure(Some(config()));
    state.set_staging(true);
    state.update(&input(rpm));
    state
}

#[test]
fn original_numeric_staging_vectors_exclude_both_band_edges() {
    // Independent unchanged original-PC routine outputs, including strict edge behavior.
    for (rpm, expected) in [
        (4999.0, 0.0),
        (5000.0, 0.0),
        (5001.0, 0.500_285_7),
        (5875.0, 0.75),
        (6749.0, 0.999_714_3),
        (6750.0, 0.0),
        (6751.0, 0.0),
    ] {
        let mut state = staged(rpm);
        assert!((state.strength - expected).abs() < 1e-7, "rpm {rpm}");
        assert_eq!(state.strength(), 0.0, "staging cannot apply assistance");
        assert_eq!(state.launch(), expected > 0.0);
    }
    let mut state = staged(6625.0);
    let mut i = input(6625.0);
    i.peak_torque_rpm = 7500.0;
    state.update(&i);
    assert_eq!(state.strength, 0.75, "high peak is capped by the original upper band edge");
    i.first_gear = false;
    state.update(&i);
    assert_eq!(state.strength, 0.0);
}

#[test]
fn original_timer_vector_pauses_during_shifts_and_clears_at_expiry() {
    let mut state = staged(5875.0);
    assert!(state.launch());
    state.set_staging(false);
    state.update(&input(5875.0));
    assert!((state.remaining - 4.493_055_3).abs() < 1e-7);
    assert_eq!(state.strength(), 0.75);
    assert_eq!(state.traction_addition(), 0.1875);
    let before = state.remaining;
    let mut i = input(5875.0);
    i.shifting = true;
    state.update(&i);
    assert_eq!(state.remaining, before);
    i.shifting = false;
    i.dt = before;
    state.update(&i);
    assert_eq!(state.remaining, 0.0);
    assert_eq!(state.strength(), 0.0);
    assert!(!state.launch(), "expired strength cannot restart the launch");
}

#[test]
fn speed_cutoff_is_strict_and_reset_preserves_only_configuration() {
    let mut state = staged(5875.0);
    state.launch();
    state.set_staging(false);
    let mut i = input(5875.0);
    i.speed = config().speed_cutoff;
    state.update(&i);
    assert_eq!(state.strength(), 0.75);
    i.speed += 0.001;
    state.update(&i);
    assert_eq!(state.strength(), 0.0);
    state.reset();
    state.set_staging(true);
    state.update(&input(5875.0));
    assert!(state.launch());
    state.configure(None);
    state.set_staging(true);
    state.update(&input(5875.0));
    assert!(!state.staging());
    assert!(!state.launch());
}

#[test]
fn optional_shift_cancellation_is_distinct_from_ordinary_shift_pause() {
    let mut state = LaunchState::default();
    state.configure(Some(LaunchConfig { cancel_on_shift: true, ..config() }));
    state.set_staging(true);
    state.update(&input(5875.0));
    state.launch();
    state.set_staging(false);
    let mut i = input(5875.0);
    i.shifting = true;
    state.update(&i);
    assert_eq!(state.strength(), 0.0);
}
