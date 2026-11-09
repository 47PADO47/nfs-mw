//! The small pops of a bike's exhaust during a burnout (`docs/specs/ride6-burnout.md`, section 10): while the rear wheel spins on the spot
//! the engine sits on its limiter, and the pipe flicks the same small flame a gear change makes ([`ExhaustFlames::pop`]: a bike's pop is the
//! shift flame, a quarter larger, `trigger::BIKE_BACKFIRE_SCALE_OVER_SHIFT`) every [`BURNOUT_POP_INTERVAL`] seconds. It rides on the
//! existing trigger, so the `exhaust_flames` setting switches it off with the rest of the flames.

use super::ExhaustFlames;

/// Seconds between two pops of a burnout. A choice of this project (the player asked for "every 3 s").
pub const BURNOUT_POP_INTERVAL: f32 = 3.0;

/// Seconds after the burnout begins when the first pop comes: the engine has been on its limiter for a moment, and a burnout that lasts a
/// few seconds still gets one. A choice of this project.
pub const BURNOUT_FIRST_POP: f32 = 1.0;

/// When the next pop of the burnout in progress is due.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct BurnoutPops {
    /// Seconds since the burnout began, or since its last pop.
    since: f32,
    /// Whether the burnout in progress has had its first pop.
    popped: bool,
}

impl BurnoutPops {
    /// One physics step of `dt` seconds with the bike in a burnout or not; whether a pop is due now. Out of a burnout the clock stands at zero
    /// and the next burnout starts again with its first pop.
    pub(super) fn step(&mut self, dt: f32, burnout: bool) -> bool {
        if !burnout {
            *self = Self::default();
            return false;
        }
        self.since += dt;
        let due = match self.popped {
            true => BURNOUT_POP_INTERVAL,
            false => BURNOUT_FIRST_POP,
        };
        if self.since < due {
            return false;
        }
        self.since -= due;
        self.popped = true;
        true
    }
}

impl ExhaustFlames {
    /// One physics step of the bike: while it is in a burnout a pop is made every [`BURNOUT_POP_INTERVAL`] seconds, at the next step like the
    /// console's `exhaust-flames pop` (and no flame when the `exhaust_flames` setting is off). Returns whether a pop is due now, flames
    /// or not: the caller voices it (the sound of a pop does not depend on the flames setting), see `audio/car/own_sfx.rs`.
    pub fn note_burnout(&mut self, dt: f32, burnout: bool) -> bool {
        let due = self.burnout.step(dt, burnout);
        if due {
            self.pop();
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    /// The times (s) of the pops of a burnout that lasts `seconds`.
    fn pops(seconds: f32) -> Vec<f32> {
        let mut timer = BurnoutPops::default();
        let steps = (seconds / DT).round() as usize;
        (1..=steps).filter(|_| timer.step(DT, true)).map(|step| step as f32 * DT).collect()
    }

    #[test]
    fn a_burnout_pops_once_early_and_then_every_three_seconds() {
        let times = pops(11.0);
        assert_eq!(times.len(), 4, "{times:?}");
        assert!((times[0] - BURNOUT_FIRST_POP).abs() < 2.0 * DT);
        for pair in times.windows(2) {
            assert!((pair[1] - pair[0] - BURNOUT_POP_INTERVAL).abs() < 2.0 * DT, "{times:?}");
        }
        assert_eq!(BURNOUT_POP_INTERVAL, 3.0);
    }

    #[test]
    fn no_burnout_no_pop_and_the_next_burnout_starts_over() {
        let mut timer = BurnoutPops::default();
        for _ in 0..600 {
            assert!(!timer.step(DT, false));
        }
        assert!(pops(0.9).is_empty(), "shorter than the first pop");
        // Half a burnout, a pause, a burnout again: the first pop comes a second into the new one.
        for _ in 0..30 {
            timer.step(DT, true);
        }
        timer.step(DT, false);
        let mut first = None;
        for step in 1..=120 {
            if timer.step(DT, true) {
                first.get_or_insert(step as f32 * DT);
            }
        }
        assert!((first.expect("a pop") - BURNOUT_FIRST_POP).abs() < 2.0 * DT);
    }

    #[test]
    fn the_pop_is_the_consoles_pop_and_the_flames_setting_switches_it_off() {
        let mut flames = ExhaustFlames::default();
        // The first pop is due after a second, with the setting off: nothing is asked.
        assert!(flames.note_burnout(1.0, true), "the pop is due: its sound plays whatever the flames setting");
        assert!(!flames.forced, "the exhaust_flames setting is off: no flame");
        flames.set_enabled(true);
        assert!(!flames.note_burnout(2.9, true) && !flames.forced, "not due yet");
        assert!(
            flames.note_burnout(0.2, true) && flames.forced,
            "three seconds later: the same backfire as `exhaust-flames pop`"
        );
        assert!(!flames.note_burnout(0.1, false), "no burnout, no pop");
    }
}
