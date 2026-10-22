//! MP-35 reset to track: a client asks (`ResetRequest`), the server decides (`security-gameplay.md` section 4.2). The
//! server picks where the car appears; this rule only says whether and at what cost.
//!
//! - At most one granted reset per [`RESET_COOLDOWN_TICKS`] (10 s) per player, in every mode (the rate limit of
//!   protocol-messages.md section 7.2 applies to every mode; a refused request does not restart the wait).
//! - Refused while the player is busted or under arrest, spectating, in the countdown, or already finished.
//! - In a race the grant carries a [`RESET_PENALTY_MS`] time penalty; in free roam (and the other modes without a
//!   race clock) it is free. The caller feeds the grant to the mode as [`crate::modes::ModeInput::Reset`].
//!
//! Values **[ours]**: the penalty is 5 s (the docs give none).

use nfsmw_sim::{PlayerId, TICK_HZ, Tick};

/// Ticks between two granted resets of one player: ten seconds.
pub const RESET_COOLDOWN_TICKS: u32 = 10 * TICK_HZ;
/// The time a reset adds to a race time.
pub const RESET_PENALTY_MS: u32 = 5_000;

/// What the player is doing when the request arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResetState {
    /// Free roam, pursuit co-op, tag: no race clock.
    Roaming,
    /// A race before the start.
    Countdown,
    /// A race, on the course.
    Racing,
    /// A race, after the finish or out of it.
    Finished,
    Spectating,
    /// The busted sequence or an arrest is running.
    Busted,
}

/// Why a reset was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum ResetRefusal {
    #[error("one reset per 10 s: wait until tick {ready_at}")]
    Cooldown { ready_at: Tick },
    #[error("no reset during the countdown")]
    Countdown,
    #[error("no reset after the finish")]
    Finished,
    #[error("a spectator has no car")]
    Spectating,
    #[error("no reset while busted or under arrest")]
    Busted,
}

/// A granted reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResetGrant {
    /// Added to the race time; 0 outside a race.
    pub penalty_ms: u32,
}

/// The reset rule of one session: the last granted reset of each player.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResetRule {
    /// In player-id order.
    last: Vec<(PlayerId, Tick)>,
}

impl ResetRule {
    pub fn new() -> Self {
        Self::default()
    }

    /// The server's answer to a reset request from `player` at `tick` in `state`.
    pub fn request(&mut self, player: PlayerId, tick: Tick, state: ResetState) -> Result<ResetGrant, ResetRefusal> {
        let penalty_ms = match state {
            ResetState::Countdown => return Err(ResetRefusal::Countdown),
            ResetState::Finished => return Err(ResetRefusal::Finished),
            ResetState::Spectating => return Err(ResetRefusal::Spectating),
            ResetState::Busted => return Err(ResetRefusal::Busted),
            ResetState::Roaming => 0,
            ResetState::Racing => RESET_PENALTY_MS,
        };
        let slot = self.last.binary_search_by_key(&player, |(id, _)| *id);
        if let Ok(index) = slot {
            let ready_at = self.last[index].1.saturating_add(RESET_COOLDOWN_TICKS);
            if tick < ready_at {
                return Err(ResetRefusal::Cooldown { ready_at });
            }
        }
        match slot {
            Ok(index) => self.last[index].1 = tick,
            Err(index) => self.last.insert(index, (player, tick)),
        }
        Ok(ResetGrant { penalty_ms })
    }

    /// Forgets a player who left.
    pub fn forget(&mut self, player: PlayerId) {
        self.last.retain(|(id, _)| *id != player);
    }
}
