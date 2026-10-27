use super::*;

#[test]
fn quarter_speed_keeps_the_real_tick_count_and_reduces_every_delta() {
    for rate in [60, 144] {
        for fps in [5, 30, 60, 120, 144, 240] {
            let mut clock = FixedClock::default();
            clock.set_rate_hz(rate);
            let deltas: Vec<_> = (0..fps * 2)
                .flat_map(|_| clock.advance_real(1.0 / fps as f32).into_iter().map(|period| period * 0.25))
                .collect();
            assert_eq!(deltas.len(), rate as usize * 2, "rate {rate}, FPS {fps}");
            assert!(deltas.iter().all(|&dt| dt == (0.25 / f64::from(rate)) as f32));
        }
    }
}

#[test]
fn unscaled_sixty_hz_deltas_preserve_the_legacy_step_bits() {
    let mut clock = FixedClock::default();
    let mut legacy = FixedClock::default();
    for frame in [1.0 / 120.0, 1.0 / 30.0, 0.3, 1.0 / 240.0].into_iter().cycle().take(40) {
        let deltas = clock.advance_real(frame);
        assert_eq!(deltas.len(), legacy.advance(frame) as usize);
        assert!(deltas.iter().all(|&dt| dt.to_bits() == STEP.to_bits()));
    }
}

#[test]
fn cadence_change_finishes_the_pending_tick_at_its_original_period() {
    let mut clock = FixedClock::default();
    assert!(clock.advance_real(STEP * 0.5).is_empty());
    clock.set_rate_hz(144);
    assert_eq!(clock.advance_real(STEP * 0.5), [STEP]);
    assert_eq!(clock.advance_real(1.0 / 144.0), [1.0 / 144.0]);
}

#[test]
fn loading_and_invalid_frames_preserve_real_phase_without_emitting_periods() {
    let mut clock = FixedClock::default();
    assert!(clock.advance_real(STEP * 0.5).is_empty());
    let phase = clock.alpha();
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(clock.advance_real(dt).is_empty());
        assert_eq!(clock.alpha(), phase);
    }
    assert_eq!(clock.advance_real(STEP * 0.5), [STEP]);
}

#[test]
fn network_one_per_update_ignores_the_local_cadence_and_keeps_unscaled_sixty_hz() {
    let mut clock = FixedClock::default();
    clock.set_rate_hz(144);
    clock.set_one_per_update(true);
    for frame in [STEP, STEP * 0.25, 5.0] {
        assert_eq!(clock.advance_real(frame), [STEP]);
        assert_eq!(clock.alpha(), 0.0);
    }
    assert!(clock.advance_real(0.0).is_empty());
}

#[test]
fn screenshot_slices_advance_the_shared_clock_at_the_selected_cadence() {
    for (rate, ticks) in [(60, 6), (144, 14)] {
        let mut clock = FixedClock::default();
        clock.set_rate_hz(rate);
        let deltas = clock.advance_real(STEP * 6.0);
        assert_eq!(deltas.len(), ticks);
        assert!(deltas.iter().all(|&dt| dt == (1.0 / f64::from(rate)) as f32));
    }
}

#[test]
fn zero_simulation_scale_cannot_change_the_next_canonical_real_period() {
    let mut clock = FixedClock::default();
    let paused: Vec<_> = clock.advance_real(STEP).into_iter().map(|period| period * 0.0).collect();
    assert_eq!(paused, [0.0]);
    assert_eq!(clock.advance_real(STEP), [STEP]);
}
