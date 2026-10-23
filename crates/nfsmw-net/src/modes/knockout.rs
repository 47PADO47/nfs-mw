//! MP-06 knockout: laps of a circuit; when every racer but one still in has completed a lap, that one is knocked out
//! and becomes a spectator (`blackbox_race::knockout_due` with one knockout per lap over the racers still in).
//!
//! Choices the docs leave open **[ours]**:
//!
//! - Every player is human, so the rule knocks out humans (the original skips human racers, race-mode.md section 7).
//! - A racer who leaves or is busted before the lap's knockout counts as that lap's knockout, so a disconnect does
//!   not cost the last racer a place as well.
//! - When one racer is left, that racer wins at once: the finish is the race time of that tick.
//! - The rule also runs on the last lap, where it knocks out the last racer once all others have finished.

use blackbox_race::knockout_due;
use nfsmw_sim::{PlayerId, SimEvent, Tick};

use crate::results::Placing;

use super::ModeRules;
use super::core::{OutCause, RaceCore};
use super::event::{ModeEvent, OverReason};
use super::input::ModeInput;

/// A knockout race.
#[derive(Debug, Clone, PartialEq)]
pub struct Knockout {
    core: RaceCore,
    laps: u8,
    /// The lap whose knockout comes next, from 1.
    next_lap: u32,
    police: bool,
}

impl Knockout {
    /// A knockout of `laps` laps for `players` (sorted, unique, at least two).
    pub fn new(laps: u8, players: &[PlayerId], start_tick: Tick, police: bool) -> Self {
        Self { core: RaceCore::new(players, start_tick), laps, next_lap: 1, police }
    }

    pub fn laps(&self) -> u8 {
        self.laps
    }

    /// A departure or a bust before this lap's knockout is the lap's knockout.
    fn credit_departure(&mut self, tick: Tick, player: PlayerId) {
        let racer = match self.core.racer(player) {
            Some(racer) => *racer,
            None => return,
        };
        if !self.core.live(tick) || !racer.racing() || racer.laps >= self.next_lap {
            return;
        }
        self.next_lap += 1;
    }

    /// Knocks out the last racer of every lap whose knockout is due.
    fn knock_due(&mut self, tick: Tick, out: &mut Vec<ModeEvent>) {
        loop {
            let alive = self.core.alive();
            let lap = self.next_lap;
            let completed = self.core.racers.iter().filter(|racer| racer.out.is_none() && racer.laps >= lap).count();
            if !knockout_due(alive, 1, 1, completed) {
                return;
            }
            let last: Vec<PlayerId> = self
                .core
                .racers
                .iter()
                .filter(|racer| racer.racing() && racer.laps < lap)
                .map(|racer| racer.player)
                .collect();
            for player in last {
                self.core.knock(player, OutCause::KnockedOut, tick);
                let lap = u8::try_from(lap).unwrap_or(u8::MAX);
                out.push(ModeEvent::KnockedOut { player, lap });
                out.push(ModeEvent::Spectate { player });
            }
            self.next_lap += 1;
        }
    }

    /// One racer left who has not finished: the winner.
    fn crown_survivor(&mut self, tick: Tick, out: &mut Vec<ModeEvent>) {
        if self.core.alive() != 1 {
            return;
        }
        let survivor = match self.core.racers.iter().find(|racer| racer.racing()) {
            Some(racer) => racer.player,
            None => return,
        };
        let time_ms = self.core.race_ms(tick);
        self.core.finish(survivor, time_ms, out);
        self.core.over = true;
        out.push(ModeEvent::Over { reason: OverReason::LastStanding });
    }
}

impl ModeRules for Knockout {
    fn feed(&mut self, tick: Tick, input: &ModeInput, out: &mut Vec<ModeEvent>) {
        if let ModeInput::Left { player } | ModeInput::Sim(SimEvent::Busted { player }) = *input {
            self.credit_departure(tick, player);
        }
        let live = self.core.live(tick);
        self.core.feed(tick, input, out);
        let changes_field = matches!(
            input,
            ModeInput::Left { .. }
                | ModeInput::Sim(SimEvent::LapCompleted { .. })
                | ModeInput::Sim(SimEvent::RaceFinished { .. })
                | ModeInput::Sim(SimEvent::Busted { .. })
        );
        if !live || !changes_field {
            return;
        }
        self.knock_due(tick, out);
        self.crown_survivor(tick, out);
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
