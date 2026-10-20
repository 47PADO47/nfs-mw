//! The race phases and the race clock (`docs/specs/race-mode.md` sections 2 and 5).

/// Where the race is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    /// The racers are on the grid, held.
    Staging,
    /// The board counts down; `left` seconds remain.
    Countdown { left: f32 },
    /// The race is running.
    Racing,
    /// The race is over.
    Over,
}

/// What a tick of the clock raised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockEvent {
    /// The countdown shows this number (3, 2, 1).
    Count(u32),
    /// The countdown ended: the cars are released and the race timer starts.
    Go,
    /// The time limit ran out.
    TimeUp,
}

/// The phase and the timers of one race.
#[derive(Debug, Clone, PartialEq)]
pub struct Clock {
    phase: Phase,
    elapsed: f32,
    limit: f32,
    bonus: f32,
    countdown: f32,
    shown: Option<u32>,
}

impl Clock {
    /// A race on the grid. `limit` is the time limit in seconds (0: none); `countdown` is the length of the board.
    pub fn new(limit: f32, countdown: f32) -> Self {
        Self { phase: Phase::Staging, elapsed: 0.0, limit, bonus: 0.0, countdown, shown: None }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Seconds since the race started (0 before GO).
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// Starts the countdown from the grid. A zero-length countdown starts the race at once.
    pub fn begin_countdown(&mut self, out: &mut Vec<ClockEvent>) {
        if self.phase != Phase::Staging {
            return;
        }
        if self.countdown <= 0.0 {
            self.go(out);
            return;
        }
        self.phase = Phase::Countdown { left: self.countdown };
    }

    /// Starts the race now (a rolling start).
    pub fn start_now(&mut self, out: &mut Vec<ClockEvent>) {
        if matches!(self.phase, Phase::Staging | Phase::Countdown { .. }) {
            self.go(out);
        }
    }

    fn go(&mut self, out: &mut Vec<ClockEvent>) {
        self.phase = Phase::Racing;
        self.elapsed = 0.0;
        out.push(ClockEvent::Go);
    }

    /// Ends the race.
    pub fn finish(&mut self) {
        self.phase = Phase::Over;
    }

    /// Adds bonus seconds to the limit (the tollbooth time bonus).
    pub fn award_bonus(&mut self, seconds: f32) {
        self.bonus += seconds;
    }

    /// Seconds left before the limit; 0 when there is no limit or it ran out. `max(0, limit + bonus - elapsed)`, and
    /// 0 when `limit + bonus` is not above 0 (`docs/specs/race-mode.md` section 5).
    pub fn time_left(&self) -> f32 {
        let total = self.limit + self.bonus;
        if total <= 0.0 {
            return 0.0;
        }
        (total - self.elapsed).max(0.0)
    }

    /// Whether the race has a time limit at all.
    pub fn has_limit(&self) -> bool {
        self.limit + self.bonus > 0.0
    }

    /// Advances the clock by `dt` seconds.
    pub fn tick(&mut self, dt: f32, out: &mut Vec<ClockEvent>) {
        match self.phase {
            Phase::Countdown { left } => {
                let left = left - dt;
                if left <= 0.0 {
                    self.go(out);
                    return;
                }
                self.phase = Phase::Countdown { left };
                let number = left.ceil() as u32;
                if self.shown != Some(number) {
                    self.shown = Some(number);
                    out.push(ClockEvent::Count(number));
                }
            }
            Phase::Racing => {
                self.elapsed += dt;
                if self.has_limit() && self.time_left() <= 0.0 {
                    self.phase = Phase::Over;
                    out.push(ClockEvent::TimeUp);
                }
            }
            Phase::Staging | Phase::Over => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(clock: &mut Clock, seconds: f32) -> Vec<ClockEvent> {
        let mut events = Vec::new();
        let steps = (seconds * 60.0).round() as u32;
        for _ in 0..steps {
            clock.tick(1.0 / 60.0, &mut events);
        }
        events
    }

    #[test]
    fn the_countdown_counts_3_2_1_then_goes() {
        let mut clock = Clock::new(0.0, 3.0);
        let mut events = Vec::new();
        clock.begin_countdown(&mut events);
        assert_eq!(clock.phase(), Phase::Countdown { left: 3.0 });
        let events = run(&mut clock, 3.1);
        assert_eq!(events, [ClockEvent::Count(3), ClockEvent::Count(2), ClockEvent::Count(1), ClockEvent::Go]);
        assert_eq!(clock.phase(), Phase::Racing);
    }

    #[test]
    fn staging_does_not_tick() {
        let mut clock = Clock::new(0.0, 3.0);
        assert!(run(&mut clock, 5.0).is_empty());
        assert_eq!(clock.elapsed(), 0.0);
    }

    #[test]
    fn a_rolling_start_goes_at_once() {
        let mut clock = Clock::new(0.0, 3.0);
        let mut events = Vec::new();
        clock.start_now(&mut events);
        assert_eq!(events, [ClockEvent::Go]);
        assert_eq!(clock.phase(), Phase::Racing);
    }

    #[test]
    fn time_left_follows_the_limit_and_the_bonus() {
        let mut clock = Clock::new(60.0, 0.0);
        let mut events = Vec::new();
        clock.begin_countdown(&mut events);
        run(&mut clock, 10.0);
        assert!((clock.time_left() - 50.0).abs() < 0.05);
        clock.award_bonus(15.0);
        assert!((clock.time_left() - 65.0).abs() < 0.05);
    }

    #[test]
    fn no_limit_means_no_time_left_and_no_time_up() {
        let mut clock = Clock::new(0.0, 0.0);
        let mut events = Vec::new();
        clock.begin_countdown(&mut events);
        assert!(!clock.has_limit());
        let events = run(&mut clock, 100.0);
        assert!(events.is_empty());
        assert_eq!(clock.time_left(), 0.0);
        assert_eq!(clock.phase(), Phase::Racing);
    }

    #[test]
    fn the_limit_running_out_ends_the_race_once() {
        let mut clock = Clock::new(5.0, 0.0);
        let mut events = Vec::new();
        clock.begin_countdown(&mut events);
        let events = run(&mut clock, 8.0);
        assert_eq!(events, [ClockEvent::TimeUp]);
        assert_eq!(clock.phase(), Phase::Over);
    }
}
