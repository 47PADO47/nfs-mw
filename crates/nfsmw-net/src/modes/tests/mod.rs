//! Tests of the mode rules, driven the way the server drives them: inputs of a tick, then the end of the tick.

mod coop;
mod knockout;
mod race;
mod setup;
mod speedtrap;
mod tag;
mod wire;

use nfsmw_sim::{PlayerId, SimEvent, Tick};

use super::*;
use crate::message::Mode;

/// The start tick of every test round.
const START: Tick = 100;

fn p(id: u8) -> PlayerId {
    PlayerId(id)
}

fn rules(laps: u8) -> CustomRules {
    CustomRules { laps, traffic_density: 50, cop_level_cap: 5, time_of_day: 600, nitrous: true, pursuits: true }
}

/// A mode and the tick the server is at.
struct Run {
    mode: AnyMode,
    tick: Tick,
}

impl Run {
    fn with(setup: ModeSetup) -> Self {
        let mut mode = AnyMode::build(&setup).expect("a valid setup");
        let mut out = Vec::new();
        mode.begin(&mut out);
        Self { mode, tick: setup.start_tick }
    }

    fn new(kind: Mode, laps: u8, players: &[u8]) -> Self {
        let players = players.iter().copied().map(PlayerId).collect();
        Self::with(ModeSetup::new(kind, rules(laps), players, START))
    }

    fn feed(&mut self, input: ModeInput) -> Vec<ModeEvent> {
        let mut out = Vec::new();
        self.mode.feed(self.tick, &input, &mut out);
        out
    }

    fn sim(&mut self, event: SimEvent) -> Vec<ModeEvent> {
        self.feed(ModeInput::Sim(event))
    }

    fn lap(&mut self, player: u8, lap: u8, time_ms: u32) -> Vec<ModeEvent> {
        self.sim(SimEvent::LapCompleted { player: p(player), lap, time_ms })
    }

    fn finish(&mut self, player: u8, time_ms: u32) -> Vec<ModeEvent> {
        self.sim(SimEvent::RaceFinished { player: p(player), place: 0, time_ms })
    }

    fn leave(&mut self, player: u8) -> Vec<ModeEvent> {
        self.feed(ModeInput::Left { player: p(player) })
    }

    /// Ends the current tick and moves to the next.
    fn step(&mut self) -> Vec<ModeEvent> {
        let mut out = Vec::new();
        self.mode.end_tick(self.tick, &mut out);
        self.tick += 1;
        out
    }

    /// Ends `n` ticks.
    fn steps(&mut self, n: u32) -> Vec<ModeEvent> {
        (0..n).flat_map(|_| self.step()).collect()
    }

    fn order(&self) -> Vec<u8> {
        self.mode.placings().iter().map(|placing| placing.player.0).collect()
    }
}

fn over(events: &[ModeEvent]) -> Option<OverReason> {
    events.iter().find_map(|event| match event {
        ModeEvent::Over { reason } => Some(*reason),
        _ => None,
    })
}
