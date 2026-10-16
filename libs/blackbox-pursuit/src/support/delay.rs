//! The shared timer of the ram delay (spec `ai-pursuit-support.md` section 4). The binary keeps one support timer on the
//! manager: each ram car taken sets it to 3.0 s, and a request is placed only once it has run below zero. The timer is
//! kept by the caller (the world), because the original shares it between requests.

/// Seconds the timer is set to for each ram car taken (the acquisition's constant).
pub const RAM_DELAY: f32 = 3.0;

/// The shared support timer. It starts below zero, so a request that takes no ram car is placed at once.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RamDelay {
    remaining: f32,
}

impl Default for RamDelay {
    fn default() -> Self {
        Self { remaining: -1.0 }
    }
}

impl RamDelay {
    /// A ram car was taken: the timer restarts at [`RAM_DELAY`].
    pub fn arm(&mut self) {
        self.remaining = RAM_DELAY;
    }

    /// Counts the timer down by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        self.remaining -= dt;
    }

    /// The timer is below zero: a request may be placed.
    pub fn is_clear(&self) -> bool {
        self.remaining < 0.0
    }

    /// Seconds left (negative once clear).
    pub fn remaining(&self) -> f32 {
        self.remaining
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_timer_is_clear() {
        assert!(RamDelay::default().is_clear());
    }

    #[test]
    fn a_ram_car_holds_the_placement_for_three_seconds() {
        let mut delay = RamDelay::default();
        delay.arm();
        delay.tick(1.5);
        assert!(!delay.is_clear(), "1.5 s of 3.0 s");
        delay.tick(1.6);
        assert!(delay.is_clear(), "past 3.0 s");
    }

    #[test]
    fn taking_a_second_ram_car_restarts_the_delay() {
        let mut delay = RamDelay::default();
        delay.arm();
        delay.tick(2.0);
        delay.arm();
        assert_eq!(delay.remaining(), RAM_DELAY);
    }
}
