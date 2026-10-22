//! MP-01 free roam: everybody drives the open city. It never ends on its own (the lobby ends the session). With
//! pursuits on, the police chase players through one shared pursuit ([`PursuitTracker`]); a busted player respawns
//! (the world's job) and keeps playing. With pursuits off the world runs no police ([`ModeRules::police`] is false)
//! and pursuit events are ignored. Placings are unranked: free roam has session stats, not places.

use nfsmw_sim::{PlayerId, Tick};

use crate::results::{FLAG_LEFT, Placing};

use super::ModeRules;
use super::event::ModeEvent;
use super::input::ModeInput;
use super::pursuit::PursuitTracker;

/// A free-roam session.
#[derive(Debug, Clone, PartialEq)]
pub struct FreeRoam {
    /// Every player seen, in id order, and whether they left.
    players: Vec<(PlayerId, bool)>,
    pursuits: Option<PursuitTracker>,
    start_tick: Tick,
}

impl FreeRoam {
    /// Free roam for `players` (sorted, unique); `pursuits` holds the tracker when pursuits are on.
    pub fn new(players: &[PlayerId], start_tick: Tick, pursuits: Option<PursuitTracker>) -> Self {
        Self { players: players.iter().map(|player| (*player, false)).collect(), pursuits, start_tick }
    }

    /// The pursuit tracker, when pursuits are on.
    pub fn pursuits(&self) -> Option<&PursuitTracker> {
        self.pursuits.as_ref()
    }

    /// Adds a player who joined the running session (free roam is open to joiners, MP-23).
    pub fn join(&mut self, player: PlayerId) {
        match self.players.binary_search_by_key(&player, |(id, _)| *id) {
            Ok(index) => self.players[index].1 = false,
            Err(index) => self.players.insert(index, (player, false)),
        }
    }
}

impl ModeRules for FreeRoam {
    fn feed(&mut self, tick: Tick, input: &ModeInput, out: &mut Vec<ModeEvent>) {
        if tick < self.start_tick {
            return;
        }
        if let ModeInput::Left { player } = *input
            && let Ok(index) = self.players.binary_search_by_key(&player, |(id, _)| *id)
        {
            self.players[index].1 = true;
        }
        if let Some(tracker) = self.pursuits.as_mut() {
            tracker.feed(input, out);
        }
    }

    fn is_over(&self) -> bool {
        false
    }

    fn placings(&self) -> Vec<Placing> {
        self.players
            .iter()
            .map(|(player, left)| {
                let flags = match left {
                    true => FLAG_LEFT,
                    false => 0,
                };
                Placing::unranked(*player, flags)
            })
            .collect()
    }

    fn police(&self) -> bool {
        self.pursuits.is_some()
    }
}
