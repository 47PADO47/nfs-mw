//! The building blocks on their own.

use crate::avg::RunningAverage;
use crate::engine::{MAX_TICKS_PER_UPDATE, Ticker};
use crate::interp::{Curve, Interp};
use crate::math::{bezier_y, equal_power_sq, ramp, slew, slew_asym};
use crate::rng::Rng;
use crate::{DecelWindow, MixLevels, TICK_SECONDS};

#[test]
fn slew_moves_by_at_most_the_step() {
    assert_eq!(slew(0.0, 10.0, 3.0), 3.0);
    assert_eq!(slew(0.0, -10.0, 3.0), -3.0);
    assert_eq!(slew(0.0, 2.0, 3.0), 2.0);
    assert_eq!(slew_asym(100.0, 5000.0, 999.0, 60.0), 1099.0);
    assert_eq!(slew_asym(100.0, -5000.0, 999.0, 60.0), 40.0);
}

#[test]
fn ramp_clamps_at_both_ends() {
    assert_eq!(ramp(0.0, 4000.0, 7000.0, 26000.0, 32767.0), 26000.0);
    assert_eq!(ramp(9000.0, 4000.0, 7000.0, 26000.0, 32767.0), 32767.0);
    assert!((ramp(5500.0, 4000.0, 7000.0, 0.0, 1.0) - 0.5).abs() < 1e-6);
    assert_eq!(ramp(1.0, 2.0, 2.0, 0.0, 1.0), 0.0, "a zero-width ramp is a step");
    assert_eq!(ramp(3.0, 2.0, 2.0, 0.0, 1.0), 1.0);
}

#[test]
fn bezier_and_equal_power_hit_their_end_points() {
    assert_eq!(bezier_y([0.0, 0.2, 0.6, 1.0], 0.0), 0.0);
    assert!((bezier_y([0.0, 0.2, 0.6, 1.0], 1.0) - 1.0).abs() < 1e-6);
    assert!((bezier_y([0.0, 0.2, 0.6, 1.0], 0.5) - (0.375 * 0.2 + 0.375 * 0.6 + 0.125)).abs() < 1e-6);
    assert_eq!(equal_power_sq(0.0), 0.0);
    assert!((equal_power_sq(1.0) - 1.0).abs() < 1e-6);
    assert!((equal_power_sq(0.5) - 0.5).abs() < 1e-6);
}

#[test]
fn interp_runs_over_its_length_and_lands_on_the_finish() {
    let mut i = Interp::default();
    assert!(i.finished());
    i.begin(0.0, 10.0, 100.0, Curve::Linear);
    i.update(0.05);
    assert!((i.value() - 5.0).abs() < 1e-4);
    assert!(!i.finished());
    i.update(0.06);
    assert!(i.finished());
    assert_eq!(i.value(), 10.0);
    i.update(1.0);
    assert_eq!(i.value(), 10.0);
}

#[test]
fn interp_with_no_length_takes_ten_milliseconds_and_a_live_target_wins() {
    let mut i = Interp::default();
    i.begin(1.0, 1.0, 0.0, Curve::Linear);
    assert!(!i.finished());
    i.update(0.005);
    assert!(!i.finished());
    i.update(0.006);
    assert!(i.finished());
    i.begin(0.0, 1.0, 100.0, Curve::EqPowerSq);
    i.update_live(0.2, 7.0);
    assert!(i.finished());
    assert_eq!(i.value(), 7.0);
}

#[test]
fn running_average_replaces_the_oldest_slot() {
    let mut a = RunningAverage::<4>::flushed(0.0);
    a.record(4.0);
    assert_eq!(a.value(), 1.0);
    for _ in 0..4 {
        a.record(8.0);
    }
    assert_eq!(a.value(), 8.0);
    a.flush(2.0);
    assert_eq!(a.value(), 2.0);
}

#[test]
fn the_ticker_runs_whole_ticks_and_carries_the_remainder() {
    let mut t = Ticker::default();
    assert_eq!(t.advance(TICK_SECONDS), 1);
    assert_eq!(t.advance(TICK_SECONDS * 0.4), 0);
    assert_eq!(t.advance(TICK_SECONDS * 0.4), 0);
    assert_eq!(t.advance(TICK_SECONDS * 0.4), 1, "1.2 ticks accumulated");
    assert_eq!(t.advance(TICK_SECONDS * 3.0), 3);
    assert_eq!(t.advance(10.0), MAX_TICKS_PER_UPDATE, "a stall is capped");
    assert_eq!(t.advance(0.0), 0, "and its excess is dropped");
    assert_eq!(t.advance(f32::NAN), 0);
    assert_eq!(t.advance(-1.0), 0);
}

#[test]
fn the_generator_is_seeded_and_bounded() {
    let mut a = Rng::new(7);
    let mut b = Rng::new(7);
    assert!((0..100).all(|_| a.next_u32() == b.next_u32()));
    let mut z = Rng::new(0);
    assert_ne!(z.next_u32(), 0, "a zero seed still runs");
    assert!((0..1000).all(|_| a.below(5) < 5));
    assert_eq!(a.below(0), 0);
}

#[test]
fn the_decel_window_is_a_five_point_polyline() {
    let w = DecelWindow { min_rpm: 2000.0, max_rpm: 7000.0, fade_in_fraction: 0.25, fade_out_fraction: 0.02 };
    assert_eq!(w.gain(1000.0), 0.0);
    assert_eq!(w.gain(2000.0), 0.0);
    assert!((w.gain(2625.0) - 0.5).abs() < 1e-6, "halfway up the fade-in (625 of 1250)");
    assert_eq!(w.gain(4000.0), 1.0);
    assert!((w.gain(6950.0) - 0.5).abs() < 1e-3, "50 of 100 down the fade-out");
    assert_eq!(w.gain(7000.0), 0.0);
    assert_eq!(w.gain(f32::NAN), 0.0);
}

#[test]
fn mix_levels_interpolate_between_steady_and_large() {
    let l = MixLevels { steady: 0.2, large: 0.8 };
    assert_eq!(l.at(0.0), 0.2);
    assert!((l.at(0.5) - 0.5).abs() < 1e-6);
    assert!((l.at(1.0) - 0.8).abs() < 1e-6);
}
