use super::*;
use crate::rng::SplitMix64;

const HZ: u32 = 60;
const OFFSET_MS: f64 = 12_345.0;

/// The server's tick at its own time `t_ms`, when its clock runs `offset_ms` ahead of the client's.
fn server_tick(t_ms: f64, offset_ms: f64) -> f64 {
    (t_ms + offset_ms) * f64::from(HZ) / 1000.0
}

fn rate_of(adjust: Adjust) -> f64 {
    match adjust {
        Adjust::Slew(rate) => rate,
        Adjust::Step(_) => panic!("expected a slew, got {adjust:?}"),
    }
}

#[test]
fn ten_jittered_samples_converge_within_half_a_tick() {
    let mut rng = SplitMix64(7);
    let mut sync = ClockSync::new(HZ);
    for i in 0..10u64 {
        let sent = 1_000 + i * 500;
        let up = 10 + rng.below(31);
        let down = 10 + rng.below(31);
        let answered = server_tick((sent + up) as f64, OFFSET_MS);
        sync.add_sample(sent, answered, sent + up + down);
    }
    let now = 6_000;
    let estimate = sync.server_tick_at(now).unwrap();
    let truth = server_tick(now as f64, OFFSET_MS);
    assert!((estimate - truth).abs() < 0.5, "estimate {estimate} is more than half a tick from {truth}");
}

#[test]
fn a_symmetric_round_trip_places_the_answer_at_the_midpoint() {
    let mut sync = ClockSync::new(HZ);
    // Sent at 0, answered at server time 10 (tick 0.6 at offset 0), received at 20.
    sync.add_sample(0, 0.6, 20);
    assert_eq!(sync.rtt_ms(), Some(20.0));
    let at_midpoint = sync.server_tick_at(10).unwrap();
    assert!((at_midpoint - 0.6).abs() < 1e-12);
    let ten_ms_later = sync.server_tick_at(20).unwrap();
    assert!((ten_ms_later - 1.2).abs() < 1e-12);
}

#[test]
fn an_outlier_with_a_long_round_trip_does_not_move_the_estimate() {
    let mut sync = ClockSync::new(HZ);
    // The best sample: round trip 20 ms, symmetric.
    sync.add_sample(1_000, server_tick(1_010.0, OFFSET_MS), 1_020);
    for i in 1..9u64 {
        let sent = 2_000 + i * 100;
        sync.add_sample(sent, server_tick(sent as f64 + 30.0, OFFSET_MS), sent + 60);
    }
    let before = sync.server_tick_at(5_000);
    // A 500 ms round trip answered 250 ms late, with a wrong tick. Still within the ten kept samples.
    sync.add_sample(4_000, 999.0, 4_500);
    assert_eq!(sync.server_tick_at(5_000), before);
    assert_eq!(sync.rtt_ms(), Some(20.0));
}

#[test]
fn only_the_last_ten_samples_are_kept() {
    let mut sync = ClockSync::new(HZ);
    sync.add_sample(0, 0.0, 10);
    for i in 1..=9u64 {
        sync.add_sample(i * 100, 0.0, i * 100 + 30);
    }
    assert_eq!(sync.rtt_ms(), Some(10.0));
    // The eleventh sample pushes out the first, which was the best.
    sync.add_sample(2_000, 0.0, 2_030);
    assert_eq!(sync.rtt_ms(), Some(30.0));
}

#[test]
fn samples_received_before_they_were_sent_are_ignored() {
    let mut sync = ClockSync::new(HZ);
    sync.add_sample(100, 5.0, 90);
    assert_eq!(sync.rtt_ms(), None);
    assert_eq!(sync.server_tick_at(100), None);
}

#[test]
fn a_non_finite_tick_is_ignored() {
    let mut sync = ClockSync::new(HZ);
    sync.add_sample(0, f64::NAN, 10);
    sync.add_sample(0, f64::INFINITY, 10);
    assert_eq!(sync.rtt_ms(), None);
}

#[test]
fn a_three_tick_error_slews_to_within_five_percent_in_about_300_ticks() {
    let mut clock = LocalClock::new(HZ);
    clock.advance(0);
    // The estimate is 3 ticks ahead of the local clock at time 0 and both run at 60 Hz.
    let truth = |now: u64| server_tick(now as f64, 0.0) + 3.0;
    let mut elapsed_ticks = None;
    let mut now = 0u64;
    while now < 20_000 {
        now += 1;
        clock.advance(now);
        let error = truth(now) - clock.tick();
        if error.abs() <= 0.15 {
            elapsed_ticks = Some(now as f64 * f64::from(HZ) / 1000.0);
            break;
        }
        clock.apply(adjust(error));
    }
    let ticks = elapsed_ticks.expect("the error never converged");
    assert!((250.0..=350.0).contains(&ticks), "took {ticks} ticks");
}

#[test]
fn the_slew_rate_is_clamped_at_the_limit() {
    assert!((rate_of(adjust(3.0)) - (1.0 + MAX_SLEW)).abs() < 1e-12);
    assert!((rate_of(adjust(-3.0)) - (1.0 - MAX_SLEW)).abs() < 1e-12);
    assert!((rate_of(adjust(1.0)) - 1.01).abs() < 1e-12);
}

#[test]
fn a_twenty_tick_error_steps_by_the_whole_error() {
    assert_eq!(adjust(20.0), Adjust::Step(20.0));
    assert_eq!(adjust(-20.0), Adjust::Step(-20.0));
    let mut clock = LocalClock::new(HZ);
    clock.advance(0);
    clock.apply(adjust(20.0));
    assert_eq!(clock.tick(), 20.0);
}

#[test]
fn the_step_threshold_is_exclusive() {
    assert!(matches!(adjust(STEP_TICKS), Adjust::Slew(_)));
    assert!(matches!(adjust(STEP_TICKS + 0.5), Adjust::Step(_)));
    assert!(matches!(adjust(-STEP_TICKS - 0.5), Adjust::Step(_)));
}

#[test]
fn a_nan_error_leaves_the_rate_alone() {
    assert_eq!(adjust(f64::NAN), Adjust::Slew(1.0));
}

#[test]
fn a_negative_step_moves_the_tick_back() {
    let mut clock = LocalClock::new(HZ);
    clock.advance(0);
    clock.advance(1_000);
    assert_eq!(clock.tick(), 60.0);
    clock.apply(Adjust::Step(-10.0));
    assert_eq!(clock.tick(), 50.0);
}

#[test]
fn time_going_backward_adds_nothing() {
    let mut clock = LocalClock::new(HZ);
    clock.advance(100);
    clock.advance(50);
    assert_eq!(clock.tick(), 0.0);
    // Measured from the latest time seen (100), not from 50.
    clock.advance(150);
    assert!((clock.tick() - 3.0).abs() < 1e-12);
}

#[test]
fn slews_never_move_the_tick_backward() {
    let mut rng = SplitMix64(99);
    let mut clock = LocalClock::new(HZ);
    let mut now = 0u64;
    clock.advance(now);
    let mut last_tick = clock.tick();
    for _ in 0..10_000 {
        now += rng.below(50);
        // Errors inside the slew band, both signs.
        let error = rng.next_f64() * 2.0 * STEP_TICKS - STEP_TICKS;
        clock.advance(now);
        clock.apply(adjust(error));
        assert!(clock.tick() >= last_tick, "tick moved backward at {now} ms");
        last_tick = clock.tick();
    }
}
