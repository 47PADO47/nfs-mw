//! `nfsmw.*` event kinds and their payloads (ADR 0005 section 4; catalogue: docs/modding/api-nfsmw.md, "Events").
//!
//! The kinds live in `MW_EVENT_RANGE_START` and up. Each kind names one payload struct that starts with its `size`, and a
//! reader uses it only when `size` covers the fields it knows. Kinds are never reused. Enumerations inside a payload are
//! `u32` numbers from [`codes`](super::codes); the cash and bounty values are `u64`, as the career will hold them.

use super::ids::MW_EVENT_RANGE_START;

/// `BbMwPursuitStarted`: a chase began.
pub const MW_EVENT_PURSUIT_STARTED: u32 = MW_EVENT_RANGE_START;
/// `BbMwPursuitEnded`: a chase ended; `reason` is an `EndReason`.
pub const MW_EVENT_PURSUIT_ENDED: u32 = MW_EVENT_RANGE_START + 1;
/// `BbMwHeatChanged`: the heat moved to a new value.
pub const MW_EVENT_HEAT_CHANGED: u32 = MW_EVENT_RANGE_START + 2;
/// `BbMwBusted`: the player was busted.
pub const MW_EVENT_BUSTED: u32 = MW_EVENT_RANGE_START + 3;
/// `BbMwEscaped`: the player escaped the chase.
pub const MW_EVENT_ESCAPED: u32 = MW_EVENT_RANGE_START + 4;
/// `BbMwInfraction`: an infraction was recorded.
pub const MW_EVENT_INFRACTION_RECORDED: u32 = MW_EVENT_RANGE_START + 5;
/// `BbMwRaceStarted`: a race began (its countdown ended and the racers are off).
pub const MW_EVENT_RACE_STARTED: u32 = MW_EVENT_RANGE_START + 6;
/// `BbMwRaceFinished`: the player finished a race, or lost it.
pub const MW_EVENT_RACE_FINISHED: u32 = MW_EVENT_RANGE_START + 7;

/// Payload of `MW_EVENT_PURSUIT_STARTED`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbMwPursuitStarted {
    /// Bytes the host filled in.
    pub size: u32,
    /// The heat level the chase started at, 1 to 10.
    pub level: u32,
    /// Cops in the first wave.
    pub cops: u32,
    /// The heat when the chase began.
    pub heat: f32,
}

/// Payload of `MW_EVENT_PURSUIT_ENDED`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbMwPursuitEnded {
    /// Bytes the host filled in.
    pub size: u32,
    /// `EndReason` number.
    pub reason: u32,
    /// Bounty held when the chase ended.
    pub bounty: u64,
    /// Seconds the chase lasted.
    pub duration: f32,
    /// Padding, zero.
    pub _pad: u32,
}

/// Payload of `MW_EVENT_HEAT_CHANGED`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbMwHeatChanged {
    /// Bytes the host filled in.
    pub size: u32,
    /// The heat level after the change, 1 to 10.
    pub level: u32,
    /// Heat before the change.
    pub old_heat: f32,
    /// Heat after the change.
    pub new_heat: f32,
}

/// Payload of `MW_EVENT_BUSTED`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbMwBusted {
    /// Bytes the host filled in.
    pub size: u32,
    /// The heat level when the player was busted, 1 to 10.
    pub level: u32,
    /// Bounty at the time of the bust.
    pub bounty: u64,
}

/// Payload of `MW_EVENT_ESCAPED`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbMwEscaped {
    /// Bytes the host filled in.
    pub size: u32,
    /// The heat level when the player escaped, 1 to 10.
    pub level: u32,
    /// Seconds the chase lasted.
    pub duration: f32,
}

/// Payload of `MW_EVENT_INFRACTION_RECORDED`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbMwInfraction {
    /// Bytes the host filled in.
    pub size: u32,
    /// `Infraction` number.
    pub infraction: u32,
    /// Fine for the infraction, in cash.
    pub fine: u32,
}

/// Payload of `MW_EVENT_RACE_STARTED`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbMwRaceStarted {
    /// Bytes the host filled in.
    pub size: u32,
    /// The race id, as `nfsmw.races` lists it.
    pub race: u32,
    /// `RaceType` number.
    pub kind: u32,
    /// Racers on the grid, the player included.
    pub racers: u32,
    /// Laps.
    pub laps: u32,
}

/// Payload of `MW_EVENT_RACE_FINISHED`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbMwRaceFinished {
    /// Bytes the host filled in.
    pub size: u32,
    /// The race id, as `nfsmw.races` lists it.
    pub race: u32,
    /// The player's place, 1 for first.
    pub position: u32,
    /// 1 when the player won, else 0.
    pub won: u32,
    /// The player's race time, seconds.
    pub time: f32,
    /// Cash paid for the finish.
    pub winnings: u32,
}
