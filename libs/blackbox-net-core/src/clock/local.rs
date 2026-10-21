//! The local tick clock: a fractional tick advanced by the caller's time and a rate factor.

use super::Adjust;

/// The client's local tick clock. It never moves backward except by a negative [`Adjust::Step`].
#[derive(Debug, Clone, PartialEq)]
pub struct LocalClock {
    tick_hz: u32,
    tick: f64,
    last_ms: Option<u64>,
    rate: f64,
}

impl LocalClock {
    /// A clock at tick 0, running at `tick_hz` ticks per second with rate factor 1.
    pub fn new(tick_hz: u32) -> Self {
        Self { tick_hz, tick: 0.0, last_ms: None, rate: 1.0 }
    }

    /// Moves the tick forward by the time since the previous call, scaled by the rate factor.
    ///
    /// The first call only sets the origin. A time before the previous call adds nothing, and the later call measures
    /// from the latest time seen.
    pub fn advance(&mut self, now_ms: u64) {
        let Some(last) = self.last_ms else {
            self.last_ms = Some(now_ms);
            return;
        };
        let dt_ms = now_ms.saturating_sub(last);
        self.last_ms = Some(last.max(now_ms));
        self.tick += dt_ms as f64 * f64::from(self.tick_hz) / 1000.0 * self.rate;
    }

    /// The current fractional tick.
    pub fn tick(&self) -> f64 {
        self.tick
    }

    /// Applies a decision from [`super::adjust`]. A slew sets the rate factor (never below zero). A step adds to the
    /// tick at once and resets the rate factor to 1.
    pub fn apply(&mut self, adjust: Adjust) {
        match adjust {
            Adjust::Slew(rate) => self.rate = rate.max(0.0),
            Adjust::Step(error) => {
                self.tick += error;
                self.rate = 1.0;
            }
        }
    }
}
