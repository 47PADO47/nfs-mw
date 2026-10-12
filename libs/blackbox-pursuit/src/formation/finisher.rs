//! The finisher of a formation: the in-position timer that arms it when the cops hold their slots, and the run timer
//! while it squeezes the target. Spec: `docs/specs/ai-pursuit-formations.md` section 5 (the timer reading and the arming
//! rule). The collapse and the goals the cops then take are not here.

/// The finisher's constants for a formation: from the formation table (box-in and rolling block 1.0 m tolerance and 4 s
/// to arm; pit 0.5 m and 1.2 s) and the level's finisher time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// `FinisherTolerance` (metres). The timer reads the average distance against four times this.
    pub tolerance: f32,
    /// `TimeToFinisher` (seconds the cops must hold their slots).
    pub time_to_arm: f32,
    /// `FinisherTime` (seconds the finisher runs).
    pub run_time: f32,
    /// `MinFinisherCops`: cops that must be in position to arm.
    pub min_cops: u32,
}

/// What the finisher did this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    None,
    /// The cops held their slots long enough: the finisher runs now.
    Armed,
    /// The finisher's run time is over: the cops return to the chase.
    Ended,
}

/// The finisher's timers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Finisher {
    in_position: f32,
    /// Seconds the finisher has run; `None` while idle (the spec's `-1`).
    running: Option<f32>,
}

impl Default for Finisher {
    fn default() -> Self {
        Self { in_position: 0.0, running: None }
    }
}

impl Finisher {
    /// Whether the finisher is running.
    pub fn is_running(&self) -> bool {
        self.running.is_some()
    }

    /// Seconds the cops have held their slots.
    pub fn in_position(&self) -> f32 {
        self.in_position
    }

    /// Advances by `dt` seconds. `cops_in_position` and `average` (the mean distance of the in-position cops to their
    /// slots, metres; `None` with no cop in position) drive the timer while the finisher is idle.
    pub fn tick(&mut self, dt: f32, cops_in_position: u32, average: Option<f32>, p: &Params) -> Event {
        if let Some(run) = self.running {
            let run = run + dt;
            if run >= p.run_time {
                self.running = None;
                self.in_position = 0.0;
                return Event::Ended;
            }
            self.running = Some(run);
            return Event::None;
        }
        match average {
            Some(avg) if cops_in_position > 0 => {
                let rate = p.tolerance * 4.0;
                let reading = ((2.0 * rate - avg) / rate).clamp(-1.0, 1.0);
                self.in_position = (self.in_position + reading * dt).max(0.0);
                if self.in_position >= p.time_to_arm {
                    if cops_in_position >= p.min_cops {
                        self.running = Some(0.0);
                        return Event::Armed;
                    }
                    self.in_position = p.time_to_arm - 0.01;
                }
            }
            _ => self.in_position = 0.0,
        }
        Event::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The box-in's constants: 1.0 m tolerance (4 m rate), 4 s to arm, 3 s run, 2 cops.
    const BOX_IN: Params = Params { tolerance: 1.0, time_to_arm: 4.0, run_time: 3.0, min_cops: 2 };

    #[test]
    fn holding_the_slots_closely_arms_the_finisher_after_the_time_to_arm() {
        let mut f = Finisher::default();
        // Average distance 1 m: the reading is (8 - 1) / 4 clamped to 1, so the timer rises by one second per second.
        let mut ticks = 0;
        let event = loop {
            ticks += 1;
            let event = f.tick(0.1, 2, Some(1.0), &BOX_IN);
            if event != Event::None || ticks > 100 {
                break event;
            }
        };
        assert_eq!(event, Event::Armed);
        assert!((ticks as f32 * 0.1 - 4.0).abs() < 0.11, "armed after about 4 s: {ticks} ticks");
        assert!(f.is_running());
    }

    #[test]
    fn far_from_the_slots_the_timer_falls_back_to_zero() {
        let mut f = Finisher::default();
        f.tick(1.0, 2, Some(1.0), &BOX_IN);
        assert!(f.in_position() > 0.9);
        // Average 12 m (3 rate): the reading is -1, the timer drains to zero.
        f.tick(5.0, 2, Some(12.0), &BOX_IN);
        assert_eq!(f.in_position(), 0.0);
    }

    #[test]
    fn between_the_rate_and_twice_it_the_timer_rises_by_less() {
        let mut f = Finisher::default();
        // Average 6 m: (8 - 6) / 4 = 0.5 per second.
        f.tick(2.0, 2, Some(6.0), &BOX_IN);
        assert!((f.in_position() - 1.0).abs() < 1e-5, "{}", f.in_position());
    }

    #[test]
    fn too_few_cops_in_position_does_not_arm_but_keeps_the_timer_just_below() {
        let mut f = Finisher::default();
        let event = f.tick(4.5, 1, Some(0.5), &BOX_IN);
        assert_eq!(event, Event::None);
        assert!((f.in_position() - 3.99).abs() < 1e-4, "held just below the arming time: {}", f.in_position());
        assert!(!f.is_running());
    }

    #[test]
    fn the_finisher_ends_after_its_run_time() {
        let mut f = Finisher::default();
        f.tick(4.0, 2, Some(0.0), &BOX_IN);
        assert!(f.is_running());
        assert_eq!(f.tick(2.0, 2, Some(0.0), &BOX_IN), Event::None);
        assert_eq!(f.tick(1.5, 2, Some(0.0), &BOX_IN), Event::Ended);
        assert!(!f.is_running());
        assert_eq!(f.in_position(), 0.0, "the timer starts again after a finisher");
    }

    #[test]
    fn no_cops_in_position_drains_the_timer() {
        let mut f = Finisher::default();
        f.tick(2.0, 2, Some(0.0), &BOX_IN);
        f.tick(1.0, 0, None, &BOX_IN);
        assert_eq!(f.in_position(), 0.0);
    }
}
