//! The lagging health: the pale segment that holds at the old level after a hit, then drains. Spec section 6.

use super::params::BarParams;

/// Below this, a hold counts as over (float slack for the countdown).
const HOLD_EPS: f32 = 1e-6;

/// The lag of one cop, stepped once per simulated step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LagTracker {
    lag: f32,
    last_health: f32,
    hold: f32,
}

impl LagTracker {
    /// Starts with the lag at `health`, so a cop seen for the first time shows no pale segment.
    pub fn new(health: f32) -> Self {
        Self { lag: health, last_health: health, hold: 0.0 }
    }

    /// The lagging value, 0..=1, never below the health it last followed.
    pub fn lag(&self) -> f32 {
        self.lag
    }

    /// One step of `dt` simulated seconds with the current `health`. Returns the new lag.
    pub fn step(&mut self, health: f32, dt: f32, params: &BarParams) -> f32 {
        if health > self.lag {
            self.lag = health;
            self.hold = 0.0;
        }
        if health < self.last_health {
            self.hold = params.lag_hold_secs;
        }
        self.last_health = health;
        if self.hold > HOLD_EPS {
            self.hold = (self.hold - dt).max(0.0);
            return self.lag;
        }
        if self.lag > health {
            self.lag = (self.lag - params.lag_drain_rate * dt).max(health);
        }
        self.lag
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health_bar::approx;

    /// Steps `tracker` through `(health, dt)` pairs and returns the lag after each.
    fn run(tracker: &mut LagTracker, steps: &[(f32, f32)]) -> Vec<f32> {
        let params = BarParams::default();
        steps.iter().map(|&(health, dt)| tracker.step(health, dt, &params)).collect()
    }

    #[test]
    fn lag_equals_health_when_never_hit() {
        let mut tracker = LagTracker::new(1.0);
        let lags = run(&mut tracker, &[(1.0, 0.25), (1.0, 0.25), (1.0, 0.25)]);
        assert!(lags.iter().all(|&lag| approx(lag, 1.0)));
    }

    #[test]
    fn one_drop_holds_half_a_second_then_drains_to_health() {
        let mut tracker = LagTracker::new(1.0);
        // The drop at t = 0; the hold runs to t = 0.5, then the lag falls 0.75 per second to 0.6.
        let lags = run(&mut tracker, &[(0.6, 0.25), (0.6, 0.25), (0.6, 0.25), (0.6, 0.25), (0.6, 0.25)]);
        assert!(approx(lags[0], 1.0), "held at t = 0");
        assert!(approx(lags[1], 1.0), "held at t = 0.25");
        assert!(approx(lags[2], 0.8125), "drain starts at t = 0.5");
        assert!(approx(lags[3], 0.625), "t = 0.75");
        assert!(approx(lags[4], 0.6), "stops at health");
    }

    #[test]
    fn second_drop_during_the_hold_restarts_the_countdown() {
        let mut tracker = LagTracker::new(1.0);
        // First drop at t = 0.1 (to 0.8), second at t = 0.2 (to 0.6). Without the restart the hold would end at
        // t = 0.6; with it, the hold runs to t = 0.7, so the lag is still 1.0 at t = 0.6.
        let lags =
            run(&mut tracker, &[(0.8, 0.1), (0.6, 0.1), (0.6, 0.1), (0.6, 0.1), (0.6, 0.1), (0.6, 0.1), (0.6, 0.1)]);
        assert!(approx(lags[0], 1.0), "first drop at t = 0.1");
        assert!(approx(lags[1], 1.0), "second drop at t = 0.2");
        assert!(approx(lags[4], 1.0), "t = 0.5, still held");
        assert!(approx(lags[5], 1.0), "t = 0.6, held by the restart");
        assert!(lags[6] < 1.0, "t = 0.7, draining");
    }

    #[test]
    fn a_new_cop_shows_no_pale_segment() {
        let mut tracker = LagTracker::new(0.6);
        let lags = run(&mut tracker, &[(0.6, 0.5)]);
        assert!(approx(lags[0], 0.6));
    }
}
