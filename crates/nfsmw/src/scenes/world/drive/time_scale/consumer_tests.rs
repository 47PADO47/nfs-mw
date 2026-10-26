use super::super::clock::FixedClock;
use super::super::speedbreaker::SpeedBreaker;
use crate::settings::speedbreaker::vanilla::{RECHARGE_ABOVE_MPS, RECHARGE_SECONDS};

#[test]
fn passive_charge_uses_each_completed_speed_across_thresholds_and_render_groupings() {
    let speeds = [RECHARGE_ABOVE_MPS - 1.0, RECHARGE_ABOVE_MPS + 1.0, RECHARGE_ABOVE_MPS, RECHARGE_ABOVE_MPS + 2.0];
    let mut reference = None;
    for fps in [5, 30, 60, 144, 240] {
        let mut meter = SpeedBreaker::default();
        meter.meter = 0.0;
        let mut clock = FixedClock::default();
        let mut tick = 0;
        for _ in 0..fps {
            // Frame updates must not pre-credit future tick time using the previous speed.
            meter.step(false, RECHARGE_ABOVE_MPS - 1.0, 0.0, 1.0 / fps as f32);
            for period in clock.advance_real(1.0 / fps as f32) {
                meter.charge_passive(speeds[tick % speeds.len()], period);
                tick += 1;
            }
        }
        assert_eq!(tick, 60);
        assert!((meter.meter - 0.5 / RECHARGE_SECONDS).abs() < 1e-7, "FPS {fps}: {}", meter.meter);
        let bits = meter.meter.to_bits();
        assert_eq!(*reference.get_or_insert(bits), bits, "render grouping must not change passive charge");
    }
}

#[test]
fn passive_charge_rejects_non_completed_intervals_and_active_meter() {
    let mut meter = SpeedBreaker::default();
    meter.meter = 0.25;
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        meter.charge_passive(50.0, dt);
        assert_eq!(meter.meter, 0.25);
    }
    assert!(meter.step(true, 50.0, 0.0, 0.0));
    meter.charge_passive(50.0, 1.0);
    assert_eq!(meter.meter, 0.25);
}
