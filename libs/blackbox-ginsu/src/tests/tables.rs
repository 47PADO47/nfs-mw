//! The lookups on the tables.

use super::chirp::standard;
use crate::{Error, GinsuTables};

#[test]
fn frequency_lookup_hits_the_knots_and_clamps() {
    let chirp = standard();
    let t = chirp.data.tables();
    assert_eq!(t.seg_count(), 50);
    assert_eq!(t.frequency_to_sample(t.min_frequency()), t.freq_pos()[0] as i32);
    assert_eq!(t.frequency_to_sample(t.min_frequency() - 500.0), t.freq_pos()[0] as i32);
    assert_eq!(t.frequency_to_sample(t.max_frequency()), t.freq_pos()[50] as i32);
    assert_eq!(t.frequency_to_sample(t.max_frequency() + 500.0), t.freq_pos()[50] as i32);
    assert_eq!(t.frequency_to_sample(f32::NAN), t.freq_pos()[0] as i32);
    for i in [1usize, 10, 25, 49] {
        let f = t.min_frequency() + (t.max_frequency() - t.min_frequency()) * i as f32 / 50.0;
        let want = t.freq_pos()[i] as i32;
        assert!((t.frequency_to_sample(f) - want).abs() <= 1, "knot {i}");
    }
}

#[test]
fn frequency_lookup_is_monotonic_for_a_rising_recording() {
    let chirp = standard();
    let t = chirp.data.tables();
    let mut last = -1;
    for step in 0..=400 {
        let f = t.min_frequency() + (t.max_frequency() - t.min_frequency()) * step as f32 / 400.0;
        let s = t.frequency_to_sample(f);
        assert!(s >= last);
        last = s;
    }
}

#[test]
fn cycles_and_samples_convert_both_ways() {
    let chirp = standard();
    let t = chirp.data.tables();
    let n = t.cycle_count();
    assert_eq!(t.sample_to_cycle(-5), 0.0);
    assert_eq!(t.sample_to_cycle(t.cycle_pos()[n] as i32 + 9), n as f32);
    for k in [0usize, 1, 7, n / 2, n - 1, n] {
        let s = t.cycle_to_sample(k as f32);
        assert_eq!(s, t.cycle_pos()[k] as i32);
        assert_eq!(t.sample_to_cycle(s), k as f32);
    }
    let mid = (t.cycle_pos()[10] + t.cycle_pos()[11]) as f32 / 2.0;
    let c = t.sample_to_cycle(mid as i32);
    assert!((c - 10.5).abs() < 0.01, "{c}");
    assert_eq!(t.cycle_to_sample(-3.0), 0);
    assert_eq!(t.cycle_to_sample(n as f32 + 3.0), t.cycle_pos()[n] as i32);
}

#[test]
fn cycle_period_follows_the_pitch() {
    let chirp = standard();
    let t = chirp.data.tables();
    let n = t.cycle_count();
    // The first cycle starts at 10 Hz and 32 kHz (3200 samples) but the pitch rises during it.
    let first = (t.cycle_pos()[1] - t.cycle_pos()[0]) as f32;
    let last = (t.cycle_pos()[n] - t.cycle_pos()[n - 1]) as f32;
    assert_eq!(t.cycle_period(0.0), first);
    assert_eq!(t.cycle_period(n as f32), last);
    assert!(first > 2900.0 && first < 3200.0, "{first}");
    assert!(last > 450.0 && last < 540.0, "{last}");
    assert_eq!(t.cycle_period(-4.0), t.cycle_period(0.0));
    assert_eq!(t.cycle_period(n as f32 + 4.0), t.cycle_period(n as f32));
    let mut last = f32::MAX;
    for step in 0..=(n * 4) {
        let p = t.cycle_period(step as f32 / 4.0);
        assert!(p <= last + 0.01, "the period grows at {step}");
        last = p;
    }
}

#[test]
fn a_loop_of_one_or_two_cycles_does_not_panic() {
    let one = GinsuTables::new(1.0, 2.0, 24_000, 100, vec![0, 50], vec![0, 80]).unwrap();
    assert_eq!(one.cycle_period(0.5), 80.0);
    let two = GinsuTables::new(1.0, 2.0, 24_000, 100, vec![0, 50], vec![0, 40, 90]).unwrap();
    assert_eq!(two.cycle_period(0.0), 40.0);
    assert_eq!(two.cycle_period(2.0), 50.0);
    let none = GinsuTables::new(1.0, 2.0, 24_000, 100, vec![0], vec![0]).unwrap();
    assert_eq!(none.cycle_period(0.0), 0.0);
    assert_eq!(none.sample_to_cycle(50), 0.0);
    assert_eq!(none.frequency_to_sample(1.5), 0);
}

#[test]
fn clamp_frequency_splits_the_pitch_below_the_minimum() {
    let chirp = standard();
    let t = chirp.data.tables();
    let min = t.min_frequency();
    assert_eq!(t.clamp_frequency(min * 2.0), (min * 2.0, 1.0));
    assert_eq!(t.clamp_frequency(min), (min, 1.0));
    let (f, ratio) = t.clamp_frequency(min * 0.5);
    assert_eq!(f, min);
    assert!((ratio - 0.5).abs() < 1e-6);
}

#[test]
fn bad_tables_are_refused() {
    let err = |r: crate::Result<GinsuTables>| r.unwrap_err();
    assert!(matches!(err(GinsuTables::new(f32::NAN, 2.0, 1, 1, vec![0], vec![0])), Error::InvalidTables(_)));
    assert!(matches!(err(GinsuTables::new(3.0, 2.0, 1, 1, vec![0], vec![0])), Error::InvalidTables(_)));
    assert!(matches!(err(GinsuTables::new(1.0, 2.0, 1, 1, vec![], vec![0])), Error::InvalidTables(_)));
    assert!(matches!(err(GinsuTables::new(1.0, 2.0, 1, 1, vec![0], vec![])), Error::InvalidTables(_)));
    assert!(matches!(err(GinsuTables::new(1.0, 2.0, 1, 9, vec![0], vec![0, 5, 5])), Error::InvalidTables(_)));
}
