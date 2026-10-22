//! MP-07 pursuit co-op: the players share one pursuit against the AI police (ADR 0010 decision 4: heat, cool-down,
//! helicopter and roadblocks shared, the busted meter and the bust per player). The round is that one pursuit: it
//! waits for the police to start a chase, and ends when no target is left, also when some players were never in it.
//! The team result is the pursuit's end: evaded, all busted or abandoned.
//!
//! Choices **[ours]**: a busted player is out of the round and watches the rest as a spectator (the co-op career
//! respawns them at the safe house instead; a LAN round has no safe house); a player who leaves before the chase
//! starts is out; the round ends at once when every player left. Placings are unranked, with the busted and left flags.

use blackbox_pursuit::TargetStatus;
use nfsmw_sim::{PlayerId, SimEvent, Tick};

use crate::message::PursuitEndReason;
use crate::results::{FLAG_BUSTED, FLAG_LEFT, Placing};

use super::ModeRules;
use super::event::{ModeEvent, OverReason};
use super::input::ModeInput;
use super::pursuit::PursuitTracker;

/// A pursuit co-op round.
#[derive(Debug, Clone, PartialEq)]
pub struct PursuitCoop {
    /// Every player, in id order, with their result flags.
    players: Vec<(PlayerId, u8)>,
    tracker: PursuitTracker,
    start_tick: Tick,
    over: Option<OverReason>,
}

impl PursuitCoop {
    /// A round for `players` (sorted, unique) with the shared pursuit `tracker`.
    pub fn new(players: &[PlayerId], start_tick: Tick, tracker: PursuitTracker) -> Self {
        Self { players: players.iter().map(|player| (*player, 0)).collect(), tracker, start_tick, over: None }
    }

    pub fn tracker(&self) -> &PursuitTracker {
        &self.tracker
    }

    /// How the round ended, once it has.
    pub fn outcome(&self) -> Option<OverReason> {
        self.over
    }

    fn flag(&mut self, player: PlayerId, flag: u8) {
        if let Ok(index) = self.players.binary_search_by_key(&player, |(id, _)| *id) {
            self.players[index].1 |= flag;
        }
    }

    fn end(&mut self, reason: OverReason, out: &mut Vec<ModeEvent>) {
        self.over = Some(reason);
        out.push(ModeEvent::Over { reason });
    }
}

impl ModeRules for PursuitCoop {
    fn feed(&mut self, tick: Tick, input: &ModeInput, out: &mut Vec<ModeEvent>) {
        if tick < self.start_tick || self.over.is_some() {
            return;
        }
        let busted_now = match *input {
            ModeInput::Sim(SimEvent::Busted { player }) => {
                Some(player).filter(|player| self.tracker.status(*player) == Some(TargetStatus::Chased))
            }
            _ => None,
        };
        let ended = self.tracker.feed(input, out);
        if let Some(player) = busted_now {
            self.flag(player, FLAG_BUSTED);
            out.push(ModeEvent::Spectate { player });
        }
        if let ModeInput::Left { player } = *input {
            self.flag(player, FLAG_LEFT);
        }
        if let Some(reason) = ended {
            let over = match reason {
                PursuitEndReason::Evaded => OverReason::Evaded,
                PursuitEndReason::Busted => OverReason::AllBusted,
                PursuitEndReason::Abandoned => OverReason::Abandoned,
            };
            self.end(over, out);
            return;
        }
        let everyone_left = self.players.iter().all(|(_, flags)| flags & FLAG_LEFT != 0);
        if everyone_left {
            self.end(OverReason::Empty, out);
        }
    }

    fn is_over(&self) -> bool {
        self.over.is_some()
    }

    fn placings(&self) -> Vec<Placing> {
        self.players.iter().map(|(player, flags)| Placing::unranked(*player, *flags)).collect()
    }

    fn police(&self) -> bool {
        true
    }
}
