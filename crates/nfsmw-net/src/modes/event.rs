//! What the mode rules tell the session: events for clients and actions for the server.

use nfsmw_sim::PlayerId;

use crate::message::PursuitEndReason;

/// Why a mode's round is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverReason {
    /// Every racer finished or is out, and at least one finished.
    Finished,
    /// One player is left (knockout, tag).
    LastStanding,
    /// Every player left.
    Empty,
    /// The round's time ran out (tag).
    TimeUp,
    /// Every target or racer was busted (or left after another was busted).
    AllBusted,
    /// The pursuit co-op team got away.
    Evaded,
    /// The pursuit ended without a bust or an escape (its targets left the session).
    Abandoned,
}

/// One thing a mode decided.
#[derive(Debug, Clone, PartialEq)]
pub enum ModeEvent {
    /// A lap ended; `best` when it is the player's best so far.
    LapCompleted { player: PlayerId, lap: u8, time_ms: u32, best: bool },
    /// The player finished. `place` is provisional while a later finisher can still win on penalties; the result
    /// record is final.
    Finished { player: PlayerId, place: u8, time_ms: u32 },
    /// Last at the end of `lap` in a knockout.
    KnockedOut { player: PlayerId, lap: u8 },
    /// An action: the server makes the player a spectator (knockout, busted in pursuit co-op).
    Spectate { player: PlayerId },
    /// A speed trap paid: the stored speed in m/s and the new total.
    TrapScored { player: PlayerId, trap: u8, speed: f32, total: f32 },
    /// A pursuit began.
    PursuitStarted { pursuit: u16, targets: Vec<PlayerId>, heat: u8 },
    /// Another car joined a running pursuit; `targets` are the cars chased now.
    TargetAdded { pursuit: u16, player: PlayerId, targets: Vec<PlayerId>, heat: u8 },
    /// The shared heat changed; `targets` are the cars chased now.
    HeatChanged { pursuit: u16, level: u8, targets: Vec<PlayerId> },
    /// One target's chase ended.
    TargetEnded { pursuit: u16, player: PlayerId, reason: PursuitEndReason },
    /// No target is left: the pursuit is over.
    PursuitEnded { pursuit: u16, reason: PursuitEndReason },
    /// The tag moved to `to`; `from` is `None` for the first holder.
    TagPassed { from: Option<PlayerId>, to: PlayerId },
    /// A reset added this penalty to the player's race time.
    Penalty { player: PlayerId, ms: u32 },
    /// The round is over; read the placings now.
    Over { reason: OverReason },
}
