//! How long a traffic car has been held up by a car in its lane, and whether it is stuck.
//! Rewrite extension: the original has no lane change, no honk and no stuck timer for traffic.

/// Seconds blocked before a car looks at the lane next to it.
pub const CHANGE_AFTER: f32 = 3.0;
/// Seconds blocked before the car sounds its horn, once per blocked episode.
pub const HONK_AFTER: f32 = 3.0;
/// Seconds blocked and standing still before the car counts as stuck (for removal once it is out of view).
pub const STUCK_AFTER: f32 = 60.0;
/// Metres per second under which a car may change lane (it is slow enough to move over safely).
pub const SLOW: f32 = 5.0;
/// Metres per second under which a car counts as standing still.
pub const STANDING: f32 = 0.5;

/// The blocked clock of one car. An episode is the time from the first think that finds the car blocked until
/// the first think that does not: the clock and the honk flag start again with each episode.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Blocked {
    /// Seconds of the episode so far.
    pub for_secs: f32,
    /// Seconds of the episode the car has also been standing still.
    pub stuck_secs: f32,
    honked: bool,
}

impl Blocked {
    /// Advances the clock by `dt` seconds. `blocked` is what the last think found; `speed` is the car's speed in
    /// metres per second (either sign).
    pub fn tick(&mut self, dt: f32, blocked: bool, speed: f32) {
        if !blocked {
            *self = Self::default();
            return;
        }
        self.for_secs += dt;
        self.stuck_secs = match speed.abs() < STANDING {
            true => self.stuck_secs + dt,
            false => 0.0,
        };
    }

    /// Whether the car has waited long enough, and is slow enough, to look for a way round.
    pub fn may_change_lane(&self, speed: f32) -> bool {
        self.for_secs >= CHANGE_AFTER && speed.abs() < SLOW
    }

    /// Whether the horn sounds now: the episode has lasted [`HONK_AFTER`] and has not honked yet. Taking it marks
    /// the episode as honked.
    pub fn take_honk(&mut self) -> bool {
        if self.honked || self.for_secs < HONK_AFTER {
            return false;
        }
        self.honked = true;
        true
    }

    /// Whether the car has stood blocked for [`STUCK_AFTER`]. The caller may then remove it once it is out of view.
    pub fn stuck(&self) -> bool {
        self.stuck_secs >= STUCK_AFTER
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_runs_only_while_blocked_and_resets_when_free() {
        let mut clock = Blocked::default();
        for _ in 0..10 {
            clock.tick(0.5, true, 2.0);
        }
        assert!((clock.for_secs - 5.0).abs() < 1e-5);
        clock.tick(0.5, false, 2.0);
        assert_eq!(clock, Blocked::default());
    }

    #[test]
    fn a_lane_change_needs_three_seconds_blocked_and_slow_speed() {
        let mut clock = Blocked::default();
        clock.tick(2.9, true, 1.0);
        assert!(!clock.may_change_lane(1.0), "too soon");
        clock.tick(0.2, true, 1.0);
        assert!(clock.may_change_lane(1.0));
        assert!(!clock.may_change_lane(SLOW + 0.1), "too fast to move over");
        assert!(clock.may_change_lane(-1.0), "reverse counts by its size");
    }

    #[test]
    fn the_honk_comes_once_per_blocked_episode() {
        let mut clock = Blocked::default();
        clock.tick(HONK_AFTER - 0.1, true, 0.0);
        assert!(!clock.take_honk(), "not yet");
        clock.tick(0.2, true, 0.0);
        assert!(clock.take_honk());
        assert!(!clock.take_honk(), "already honked this episode");
        clock.tick(1.0, true, 0.0);
        assert!(!clock.take_honk(), "still the same episode");
        clock.tick(0.5, false, 0.0);
        clock.tick(HONK_AFTER, true, 0.0);
        assert!(clock.take_honk(), "a new episode may honk again");
    }

    #[test]
    fn stuck_means_standing_and_blocked_for_sixty_seconds() {
        let mut clock = Blocked::default();
        for _ in 0..119 {
            clock.tick(0.5, true, 0.0);
        }
        assert!(!clock.stuck(), "59.5 s");
        clock.tick(0.5, true, 0.0);
        assert!(clock.stuck());
        clock.tick(0.5, true, 4.0);
        assert!(!clock.stuck(), "moving again breaks the stuck run");
    }
}
