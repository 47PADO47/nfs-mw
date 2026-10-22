//! MP-02 circuit and MP-03 sprint: the world raises laps and the finish from its course; the server keeps the best
//! lap, adds reset penalties to the finish time and ranks finishers by that time, then racers still on the course by
//! laps and progress, then racers who are out (the later out, the better).

use nfsmw_sim::{PlayerId, Tick};

use crate::results::Placing;

use super::ModeRules;
use super::core::RaceCore;
use super::event::ModeEvent;
use super::input::ModeInput;

/// Which race shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RaceKind {
    /// Laps over a checkpoint loop.
    Circuit { laps: u8 },
    /// Point to point.
    Sprint,
}

/// A circuit or sprint race.
#[derive(Debug, Clone, PartialEq)]
pub struct Race {
    kind: RaceKind,
    core: RaceCore,
    police: bool,
}

impl Race {
    /// A race of `players` (sorted, unique) from `start_tick`; `police` when pursuits are on.
    pub fn new(kind: RaceKind, players: &[PlayerId], start_tick: Tick, police: bool) -> Self {
        Self { kind, core: RaceCore::new(players, start_tick), police }
    }

    pub fn kind(&self) -> RaceKind {
        self.kind
    }
}

impl ModeRules for Race {
    fn feed(&mut self, tick: Tick, input: &ModeInput, out: &mut Vec<ModeEvent>) {
        self.core.feed(tick, input, out);
        self.core.check_over(out);
    }

    fn is_over(&self) -> bool {
        self.core.over
    }

    fn placings(&self) -> Vec<Placing> {
        self.core.placings(false)
    }

    fn police(&self) -> bool {
        self.police
    }
}
