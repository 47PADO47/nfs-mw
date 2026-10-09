//! A missing limiter replacement preserves the surviving engine and the tachometer's original behavior.

use super::common::{hold, input, tuning};
use crate::{EngineMixer, TICK_SECONDS};

#[test]
fn unavailable_sample_layer_keeps_the_loops_and_tachometer_running() {
    let mut healthy = EngineMixer::new(&tuning());
    let mut missing = healthy.clone();
    missing.set_redline_sample_available(false);
    let mut last = None;
    for _ in 0..240 {
        let a = healthy.update(TICK_SECONDS, &input(1.0, 1.0, 3));
        let b = missing.update(TICK_SECONDS, &input(1.0, 1.0, 3));
        assert_eq!(a.eng_rpm, b.eng_rpm);
        assert_eq!(a.redlining, b.redlining);
        assert_eq!(a.visual_rpm_fraction, b.visual_rpm_fraction);
        assert_eq!(b.redline_sample_volume, 0.0);
        last = Some((a, b));
    }
    let (healthy, missing) = last.unwrap();
    assert!(missing.redlining);
    assert!(healthy.redline_sample_volume > 0.5);
    assert!((healthy.accel_volume / missing.accel_volume - 0.15).abs() < 1e-4);
    assert!(missing.accel_volume > 0.5, "surviving loop at its normal level");
}

#[test]
fn losing_and_restoring_the_sample_layer_uses_the_existing_fades() {
    let mut mixer = EngineMixer::new(&tuning());
    let limiter = input(1.0, 1.0, 3);
    let ducked = *hold(&mut mixer, 4.0, limiter).last().unwrap();
    mixer.set_redline_sample_available(false);
    let first = mixer.update(TICK_SECONDS, &limiter);
    assert!(first.redlining);
    assert!(first.accel_volume > ducked.accel_volume);
    assert!(first.redline_sample_volume < ducked.redline_sample_volume);
    let restored = *hold(&mut mixer, 0.1, limiter).last().unwrap();
    assert!(restored.redlining);
    assert!(restored.accel_volume > ducked.accel_volume * 6.0);
    assert_eq!(restored.redline_sample_volume, 0.0);

    mixer.set_redline_sample_available(true);
    let first = mixer.update(TICK_SECONDS, &limiter);
    assert!(first.redlining);
    assert!(first.accel_volume < restored.accel_volume);
    assert!(first.accel_volume > ducked.accel_volume, "gear-dependent attack remains gradual");
    let ducked_again = *hold(&mut mixer, 4.0, limiter).last().unwrap();
    assert_eq!(ducked_again.accel_volume, ducked.accel_volume);
    assert_eq!(ducked_again.redline_sample_volume, ducked.redline_sample_volume);
    let below = *hold(&mut mixer, 0.2, input(0.8, 1.0, 3)).last().unwrap();
    assert!(!below.redlining);
    assert_eq!(below.redline_sample_volume, 0.0);
    assert!(below.accel_volume > ducked.accel_volume * 6.0);
}

#[test]
fn reporting_a_healthy_layer_keeps_default_output_identical() {
    let mut normal = EngineMixer::new(&tuning());
    let mut reported = normal.clone();
    for tick in 0..1200 {
        reported.set_redline_sample_available(true);
        let rpm = match tick {
            0..=59 => 0.0,
            60..=239 => (tick - 60) as f32 / 180.0,
            240..=719 => 1.0,
            _ => 0.7,
        };
        let telemetry = input(rpm, 1.0, 3);
        assert_eq!(normal.update(TICK_SECONDS, &telemetry), reported.update(TICK_SECONDS, &telemetry));
    }
}
