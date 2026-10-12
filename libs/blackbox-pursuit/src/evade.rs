//! Evading the cops: the chase ends once the cops have not seen the player for long enough. The timeout is per heat
//! level (`evadetimeout`, install-verified); time spent in a hiding spot counts faster (`HiddenZoneTimeMultiplier`).
//! Spec: `docs/specs/ai-pursuit-heat.md` §2.2.

/// Unseen seconds needed to evade, per heat level 1 to 10.
pub const TIMEOUT: [f32; 10] = [20.0, 45.0, 75.0, 90.0, 120.0, 120.0, 120.0, 220.0, 60.0, 220.0];

/// Progress multiplier inside a hiding spot, per heat level 1 to 10.
pub const HIDDEN_MULTIPLIER: [f32; 10] = [4.0, 5.0, 6.0, 7.0, 8.0, 8.0, 8.0, 4.0, 2.0, 4.0];

/// The unseen timeout of a heat level (clamped to 1..=10).
pub fn timeout(level: u32) -> f32 {
    TIMEOUT[(level as usize).clamp(1, TIMEOUT.len()) - 1]
}

/// The hiding multiplier of a heat level (clamped to 1..=10).
pub fn hidden_multiplier(level: u32) -> f32 {
    HIDDEN_MULTIPLIER[(level as usize).clamp(1, HIDDEN_MULTIPLIER.len()) - 1]
}

/// How far the player has got to getting away. It fills while no cop sees the player and empties while one does.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Evade {
    unseen: f32,
}

impl Evade {
    /// Advances by `dt` seconds. `seen` resets the progress. `hidden` counts the time at the hiding multiplier.
    pub fn update(&mut self, dt: f32, seen: bool, hidden: bool, level: u32) {
        if seen {
            self.unseen = 0.0;
            return;
        }
        if !(dt > 0.0 && dt.is_finite()) {
            return;
        }
        let rate = if hidden { hidden_multiplier(level) } else { 1.0 };
        self.unseen += dt * rate;
    }

    /// Whether the player has got away at `level`.
    pub fn evaded(&self, level: u32) -> bool {
        self.unseen >= timeout(level)
    }

    /// The seconds of progress so far, for the HUD and the status line.
    pub fn progress(&self) -> f32 {
        self.unseen
    }

    /// Starts again for a new chase.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_timeouts_are_the_install_values() {
        assert_eq!(timeout(1), 20.0);
        assert_eq!(timeout(10), 220.0);
        assert_eq!(timeout(0), 20.0, "levels clamp to 1");
    }

    #[test]
    fn unseen_time_adds_up_until_the_timeout() {
        let mut evade = Evade::default();
        evade.update(19.9, false, false, 1);
        assert!(!evade.evaded(1));
        evade.update(0.2, false, false, 1);
        assert!(evade.evaded(1));
    }

    #[test]
    fn being_seen_resets_the_progress() {
        let mut evade = Evade::default();
        evade.update(15.0, false, false, 1);
        evade.update(1.0, true, false, 1);
        assert_eq!(evade.progress(), 0.0);
    }

    #[test]
    fn a_hiding_spot_speeds_the_progress_up() {
        let mut evade = Evade::default();
        evade.update(1.0, false, true, 1);
        assert_eq!(evade.progress(), 4.0, "level 1 multiplier is 4");
    }

    #[test]
    fn a_bad_step_changes_nothing() {
        let mut evade = Evade::default();
        evade.update(f32::NAN, false, false, 3);
        evade.update(-1.0, false, false, 3);
        assert_eq!(evade.progress(), 0.0);
    }

    #[test]
    fn reset_starts_a_new_chase() {
        let mut evade = Evade::default();
        evade.update(5.0, false, false, 2);
        evade.reset();
        assert_eq!(evade, Evade::default());
    }
}
