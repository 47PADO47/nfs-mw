use super::*;

#[test]
fn the_rolling_sum_forgets_old_buckets() {
    let mut rolling = Rolling::new(100, 10);
    rolling.add(0, 5);
    rolling.add(950, 7);
    assert_eq!(rolling.sum(999), 12);
    assert_eq!(rolling.sum(1000), 7, "the first bucket left the window");
    assert_eq!(rolling.sum(1950), 0);
    rolling.add(100_000, 3);
    assert_eq!(rolling.sum(100_000), 3);
    // An add stamped before the head counts into the head.
    rolling.add(50, 1);
    assert_eq!(rolling.sum(100_000), 4);
    assert!((rolling.per_second(100_000) - 4.0).abs() < 1.0e-9);
}

#[test]
fn bandwidth_is_bytes_per_second_over_two_seconds() {
    let mut stats = NetStats::new(3);
    for ms in (0..4000).step_by(50) {
        stats.on_bytes_in(400, ms);
        stats.on_bytes_out(60, ms);
    }
    let line = stats.line(3999);
    assert!((line.bytes_in_per_s - 8000.0).abs() < 1.0, "{}", line.bytes_in_per_s);
    assert!((line.bytes_out_per_s - 1200.0).abs() < 1.0, "{}", line.bytes_out_per_s);
}

#[test]
fn skipped_snapshots_count_as_lost() {
    let mut stats = NetStats::new(3);
    let mut now = 0;
    for tick in (0..600u32).step_by(3) {
        now = u64::from(tick) * 50 / 3;
        // Every tenth snapshot is lost.
        if tick % 30 == 27 {
            continue;
        }
        stats.on_snapshot(tick, now);
    }
    let loss = stats.loss(now);
    assert!((loss - 0.1).abs() < 0.01, "{loss}");
    assert!(stats.jitter_ms() < 1.0, "steady arrivals have no jitter: {}", stats.jitter_ms());
    // The newest snapshot that arrived is tick 594, at 9,900 ms.
    assert_eq!(stats.snapshot_age_ms(now + 40), Some(90));
}

#[test]
fn uneven_arrivals_show_as_jitter() {
    let mut stats = NetStats::new(3);
    for i in 0..400u32 {
        let late = u64::from(i % 2) * 20;
        stats.on_snapshot(i * 3, u64::from(i) * 50 + late);
    }
    // Spacing alternates between +20 and -20 ms of the nominal 50.
    assert!((stats.jitter_ms() - 20.0).abs() < 1.0, "{}", stats.jitter_ms());
    // A reordered old snapshot is delivered but changes nothing else.
    let before = stats.jitter_ms();
    stats.on_snapshot(3, 30_000);
    assert_eq!(stats.jitter_ms(), before);
}

#[test]
fn corrections_and_snaps_per_minute() {
    let mut stats = NetStats::new(3);
    for s in 0..90u64 {
        stats.on_correction(s * 1000);
    }
    stats.on_snap(80_000);
    let line = stats.line(89_999);
    assert_eq!(line.corrections_per_min, 60);
    assert_eq!(line.snaps_per_min, 1);
}

#[test]
fn the_line_prints_every_field() {
    let line = NetLine {
        rtt_ms: Some(12.4),
        jitter_ms: 1.54,
        loss: 0.004,
        snapshot_age_ms: Some(33),
        corrections_per_min: 3,
        snaps_per_min: 0,
        bytes_in_per_s: 14_230.0,
        bytes_out_per_s: 2_100.0,
    };
    assert_eq!(
        line.to_string(),
        "net rtt 12 ms  jit 1.5 ms  loss 0.4%  snap 33 ms  corr 3/min  snaps 0/min  in 14.2 KB/s  out 2.1 KB/s"
    );
    let empty = NetStats::new(3).line(0).to_string();
    assert!(empty.starts_with("net rtt -  jit 0.0 ms  loss 0.0%  snap -"), "{empty}");
}
