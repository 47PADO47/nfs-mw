//! The lane change of a traffic car that is blocked: when it moves over, how it aborts, and when it returns home.
//! Rewrite extension (spec `docs/specs/ai-traffic-obstacles.md` §10.1). The navigator and the car's position are the
//! caller's: this machine only says which lane the car should be in and when the blend is done.

use super::blend::{Blend, blend_length};

/// Metres a car must drive clear of the blocker in the new lane before it may move back home.
pub const RETURN_AFTER: f32 = 15.0;

/// Where the car is in its lane change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// In its own lane, looking for a block.
    Keep,
    /// Blending from lane `from` to lane `to`. `back` is set for a move home and for an abort, which is a move back
    /// to where the car came from.
    Moving { from: usize, to: usize, back: bool },
    /// In the other lane, waiting until the home lane is clear.
    Settled,
}

/// What the caller should do after a think.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Stay,
    /// Put the navigator on this traffic lane (of the same segment and direction) at the car's position.
    Move(usize),
}

/// What the car knows at a think.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Situation {
    /// The last think found the car blocked.
    pub blocked: bool,
    /// The blocked clock allows a lane change now (`Blocked::may_change_lane`).
    pub may_change: bool,
    /// Speed, m/s (either sign).
    pub speed: f32,
    /// Metres driven since the last think.
    pub metres: f32,
}

/// The lane change of one car. The default is in lane 0, keeping its lane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneChange {
    pub phase: Phase,
    /// The lane the car is in now (the navigator's lane).
    current: usize,
    /// The lane the car left for its first move, while it is away from it.
    home: Option<usize>,
    /// Metres driven since the car was last blocked, in the new lane.
    clear_run: f32,
    blend: Blend,
}

impl Default for LaneChange {
    fn default() -> Self {
        Self::new(0)
    }
}

impl LaneChange {
    /// A car in traffic lane `lane`, keeping it.
    pub fn new(lane: usize) -> Self {
        Self { phase: Phase::Keep, current: lane, home: None, clear_run: 0.0, blend: Blend::default() }
    }

    /// The lane the car is in.
    pub fn lane(&self) -> usize {
        self.current
    }

    /// The share of the lateral gap still to close in the blend (see [`Blend::remaining`]).
    pub fn remaining(&self) -> f32 {
        self.blend.remaining()
    }

    /// Adds the metres the car has driven: the blend moves on with them.
    pub fn advance(&mut self, metres: f32) {
        self.blend.advance(metres);
    }

    /// Decides this think. `neighbours` are the lanes next to the car's lane, `clear(lane)` says whether that lane's
    /// gap is clear now, measured from the navigator's current lane. A returned [`Command::Move`] has already been
    /// taken into account: the caller must move the navigator onto that lane.
    pub fn update(
        &mut self,
        situation: Situation,
        neighbours: [Option<usize>; 2],
        clear: impl Fn(usize) -> bool,
    ) -> Command {
        match self.phase {
            Phase::Keep => self.keep(situation, neighbours, clear),
            Phase::Moving { from, to, back } => self.moving(from, to, back, situation, clear),
            Phase::Settled => self.settled(situation, clear),
        }
    }

    fn keep(&mut self, situation: Situation, neighbours: [Option<usize>; 2], clear: impl Fn(usize) -> bool) -> Command {
        if !situation.may_change {
            return Command::Stay;
        }
        let Some(to) = neighbours.into_iter().flatten().find(|&lane| clear(lane)) else {
            return Command::Stay;
        };
        self.home.get_or_insert(self.current);
        self.start(self.current, to, false, situation.speed)
    }

    fn moving(
        &mut self,
        from: usize,
        to: usize,
        back: bool,
        situation: Situation,
        clear: impl Fn(usize) -> bool,
    ) -> Command {
        // An avoidable in the new lane during the blend: go back to the old lane and wait there.
        if !back && !clear(to) {
            self.current = from;
            return self.start(to, from, true, situation.speed);
        }
        if self.blend.done() {
            self.finish(back);
        }
        Command::Stay
    }

    fn settled(&mut self, situation: Situation, clear: impl Fn(usize) -> bool) -> Command {
        self.clear_run = match situation.blocked {
            true => 0.0,
            false => self.clear_run + situation.metres.max(0.0),
        };
        let Some(home) = self.home else {
            self.phase = Phase::Keep;
            return Command::Stay;
        };
        if self.clear_run < RETURN_AFTER || !clear(home) {
            return Command::Stay;
        }
        let from = self.current;
        self.start(from, home, true, situation.speed)
    }

    fn start(&mut self, from: usize, to: usize, back: bool, speed: f32) -> Command {
        self.phase = Phase::Moving { from, to, back };
        self.blend = Blend::start(blend_length(speed));
        self.clear_run = 0.0;
        self.current = to;
        Command::Move(to)
    }

    fn finish(&mut self, back: bool) {
        match back {
            true => {
                self.phase = Phase::Keep;
                if self.home == Some(self.current) {
                    self.home = None;
                }
            }
            false => self.phase = Phase::Settled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEFT: usize = 1;
    const OWN: usize = 2;
    const RIGHT: usize = 3;

    fn situation(blocked: bool, may_change: bool) -> Situation {
        Situation { blocked, may_change, speed: 2.0, metres: 0.0 }
    }

    fn open(_: usize) -> bool {
        true
    }

    fn shut(_: usize) -> bool {
        false
    }

    #[test]
    fn a_car_keeps_its_lane_until_the_clock_allows_a_change() {
        let mut change = LaneChange::new(OWN);
        let command = change.update(situation(true, false), [Some(LEFT), Some(RIGHT)], open);
        assert_eq!(command, Command::Stay);
        assert_eq!(change.phase, Phase::Keep);
    }

    #[test]
    fn a_blocked_car_moves_to_the_first_clear_neighbour() {
        let mut change = LaneChange::new(OWN);
        let command = change.update(situation(true, true), [Some(LEFT), Some(RIGHT)], |lane| lane == RIGHT);
        assert_eq!(command, Command::Move(RIGHT));
        assert_eq!(change.lane(), RIGHT);
        assert!(matches!(change.phase, Phase::Moving { from: OWN, to: RIGHT, back: false }));
        assert_eq!(change.remaining(), 1.0, "the blend starts at the car's own place");
    }

    #[test]
    fn a_car_with_no_clear_neighbour_waits() {
        let mut change = LaneChange::new(OWN);
        assert_eq!(change.update(situation(true, true), [Some(LEFT), Some(RIGHT)], shut), Command::Stay);
        assert_eq!(change.lane(), OWN);
        assert_eq!(change.update(situation(true, true), [None, None], open), Command::Stay, "no lane there");
    }

    #[test]
    fn once_the_blend_is_done_the_car_is_settled_in_the_new_lane() {
        let mut change = LaneChange::new(OWN);
        change.update(situation(true, true), [Some(LEFT), None], open);
        let length = blend_length(2.0);
        change.advance(length / 2.0);
        change.update(situation(false, false), [None, None], open);
        assert!(matches!(change.phase, Phase::Moving { .. }), "half way");
        change.advance(length);
        change.update(situation(false, false), [None, None], open);
        assert_eq!(change.phase, Phase::Settled);
    }

    #[test]
    fn an_avoidable_in_the_new_lane_during_the_blend_sends_the_car_back() {
        let mut change = LaneChange::new(OWN);
        change.update(situation(true, true), [Some(LEFT), None], open);
        change.advance(1.0);
        let command = change.update(situation(true, false), [None, None], shut);
        assert_eq!(command, Command::Move(OWN), "abort: back to the lane it left");
        assert_eq!(change.lane(), OWN);
        assert!(matches!(change.phase, Phase::Moving { from: LEFT, to: OWN, back: true }));
    }

    #[test]
    fn after_the_blend_home_the_car_keeps_its_lane_again() {
        let mut change = LaneChange::new(OWN);
        change.update(situation(true, true), [Some(LEFT), None], open);
        change.advance(100.0);
        change.update(situation(false, false), [None, None], open);
        let driving = Situation { blocked: false, may_change: false, speed: 10.0, metres: 20.0 };
        assert_eq!(change.update(driving, [None, None], open), Command::Move(OWN));
        change.advance(100.0);
        change.update(situation(false, false), [None, None], open);
        assert_eq!(change.phase, Phase::Keep);
        assert_eq!(change.lane(), OWN);
        assert_eq!(change.home, None, "the home lane is forgotten once the car is in it");
    }

    #[test]
    fn a_settled_car_goes_home_after_fifteen_metres_clear_when_home_is_clear() {
        let mut change = LaneChange::new(OWN);
        change.update(situation(true, true), [Some(LEFT), None], open);
        change.advance(100.0);
        change.update(situation(false, false), [None, None], open);
        assert_eq!(change.phase, Phase::Settled);
        let driving = |metres: f32| Situation { blocked: false, may_change: false, speed: 10.0, metres };
        assert_eq!(change.update(driving(10.0), [None, None], open), Command::Stay, "10 m clear");
        assert_eq!(change.update(driving(6.0), [None, None], shut), Command::Stay, "16 m but home is shut");
        assert_eq!(change.update(driving(0.0), [None, None], open), Command::Move(OWN), "home is clear now");
        assert!(matches!(change.phase, Phase::Moving { from: LEFT, to: OWN, back: true }));
    }

    #[test]
    fn a_blocker_in_the_new_lane_resets_the_run_home() {
        let mut change = LaneChange::new(OWN);
        change.update(situation(true, true), [Some(LEFT), None], open);
        change.advance(100.0);
        change.update(situation(false, false), [None, None], open);
        let driving = |blocked: bool, metres: f32| Situation { blocked, may_change: false, speed: 10.0, metres };
        change.update(driving(false, 14.0), [None, None], open);
        change.update(driving(true, 0.0), [None, None], open);
        assert_eq!(change.update(driving(false, 14.0), [None, None], open), Command::Stay, "the run started again");
    }
}
