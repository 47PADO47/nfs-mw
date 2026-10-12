//! The speed breaker's meter (docs/specs/speedbreaker.md). It drains while the action is held and recharges while the car
//! is slow. The constants below are our own placeholders: the original's values are not read yet. The slowdown the
//! breaker is for is not applied; `step` only reports whether the breaker is in use.

/// Seconds to empty a full meter while the action is held (placeholder).
pub const DRAIN_SECONDS: f32 = 5.0;
/// Seconds to fill an empty meter while the car is below `RECHARGE_BELOW_MPS` (placeholder).
pub const RECHARGE_SECONDS: f32 = 10.0;
/// The speed under which the meter recharges: 100 km/h (placeholder).
pub const RECHARGE_BELOW_MPS: f32 = 100.0 / 3.6;

/// The meter, from 0 (empty) to 1 (full). A new car starts full.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpeedBreaker {
    pub meter: f32,
}

impl Default for SpeedBreaker {
    fn default() -> Self {
        Self { meter: 1.0 }
    }
}

impl SpeedBreaker {
    /// One step of `dt` seconds. Returns whether the breaker is in use: held, with charge left.
    pub fn step(&mut self, held: bool, speed_mps: f32, dt: f32) -> bool {
        let using = held && self.meter > 0.0;
        if using {
            self.meter = (self.meter - dt / DRAIN_SECONDS).max(0.0);
        } else if speed_mps < RECHARGE_BELOW_MPS {
            self.meter = (self.meter + dt / RECHARGE_SECONDS).min(1.0);
        }
        using
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEP: f32 = 1.0 / 60.0;

    #[test]
    fn a_held_breaker_drains_to_empty_in_the_drain_time() {
        let mut sb = SpeedBreaker::default();
        let mut steps = 0;
        while sb.meter > 0.0 {
            assert!(sb.step(true, 50.0, STEP), "in use until it is empty");
            steps += 1;
            assert!(steps < 10_000, "it must empty");
        }
        let seconds = steps as f32 * STEP;
        assert!((seconds - DRAIN_SECONDS).abs() < 2.0 * STEP, "drains in {seconds} s");
        assert!(!sb.step(true, 50.0, STEP), "empty, it is not in use");
    }

    #[test]
    fn a_released_breaker_recharges_only_below_the_speed_limit() {
        let mut sb = SpeedBreaker { meter: 0.0 };
        sb.step(false, RECHARGE_BELOW_MPS + 5.0, RECHARGE_SECONDS);
        assert_eq!(sb.meter, 0.0, "too fast to recharge");
        sb.step(false, RECHARGE_BELOW_MPS - 5.0, RECHARGE_SECONDS);
        assert_eq!(sb.meter, 1.0);
    }

    #[test]
    fn the_meter_stays_within_zero_and_one() {
        let mut sb = SpeedBreaker { meter: 0.99 };
        sb.step(false, 0.0, RECHARGE_SECONDS);
        assert_eq!(sb.meter, 1.0);
    }
}
