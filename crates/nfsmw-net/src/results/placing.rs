//! One player's line of a result, as a mode's rules give it.

use nfsmw_sim::PlayerId;

use crate::message::{RESULT_BUSTED, RESULT_DNF};

/// The player did not finish (the wire's bit 0).
pub const FLAG_DNF: u8 = RESULT_DNF;
/// The player was busted (the wire's bit 1).
pub const FLAG_BUSTED: u8 = RESULT_BUSTED;
/// The player was knocked out at the end of a lap (record only).
pub const FLAG_KNOCKED_OUT: u8 = 1 << 2;
/// The player left or timed out before the end (record only).
pub const FLAG_LEFT: u8 = 1 << 3;
/// The flags that mean the player did not finish: they set the wire's DNF bit.
pub const NOT_FINISHED: u8 = FLAG_DNF | FLAG_KNOCKED_OUT | FLAG_LEFT;

/// One player's place and race facts.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Placing {
    pub player: PlayerId,
    /// From 1, unique in a result; 0 in a mode without places (free roam, pursuit co-op).
    pub place: u8,
    /// The race time with penalties, or the time as "it" in tag; `None` when there is none.
    pub time_ms: Option<u32>,
    pub best_lap_ms: Option<u32>,
    /// Speed-trap points (the sum of the trap speeds in m/s); 0 in other modes.
    pub points: f32,
    /// [`FLAG_DNF`], [`FLAG_BUSTED`], [`FLAG_KNOCKED_OUT`], [`FLAG_LEFT`].
    pub flags: u8,
}

impl Placing {
    /// An unranked line for `player` with `flags`.
    pub fn unranked(player: PlayerId, flags: u8) -> Self {
        Self { player, flags, ..Self::default() }
    }
}
