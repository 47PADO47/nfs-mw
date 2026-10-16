//! `nfsmw.races` 0.1: the race list, starts, custom races and the results (catalogue: docs/modding/api-nfsmw.md, "Races").
//!
//! The race runtime is not built yet, so every slot answers `BB_ERR_UNSUPPORTED` until it is. The list of race kinds
//! ([`BbRaceInfo::kind`]) is [unconfirmed]: it is the `RaceType` of `nfsmw-data/src/gameplay/races.rs`.

use core::ffi::c_void;

use blackbox_modding::abi::{BbIface, BbStatus};

use super::shared::{BbName, BbVec3};

/// One race of the list, as the host reports it.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbRaceInfo {
    /// Bytes the host filled in.
    pub size: u32,
    /// Race id.
    pub id: u32,
    /// `RaceType` number. [unconfirmed] list.
    pub kind: u32,
    /// Laps.
    pub laps: u32,
    /// Racers.
    pub racers: u32,
    /// Non-zero once the race is done.
    pub done: u8,
    /// Padding, zero.
    pub _pad: [u8; 3],
    /// Where the race starts, render space, metres.
    pub pos: BbVec3,
    /// The race's name.
    pub name: BbName,
}

/// The running race, when one runs.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbRaceState {
    /// Bytes the host filled in.
    pub size: u32,
    /// Non-zero while a race runs; the other fields are zero otherwise.
    pub active: u8,
    /// Padding, zero.
    pub _pad: [u8; 3],
    /// Race id.
    pub id: u32,
    /// Lap the player is on.
    pub lap: u32,
    /// Checkpoint the player is heading for.
    pub checkpoint: u32,
    /// Position in the field, 1 is first.
    pub position: u32,
    /// Seconds since the start.
    pub elapsed: f32,
}

/// The result of a finished race.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbRaceResult {
    /// Bytes the host filled in.
    pub size: u32,
    /// Final position, 1 is first.
    pub position: u32,
    /// Finish time, seconds.
    pub time: f32,
    /// Non-zero when the player won.
    pub won: u8,
    /// Padding, zero.
    pub _pad: [u8; 3],
    /// Cash won.
    pub winnings: u32,
}

/// The definition of a custom race. Size-prefixed. Its checkpoints are passed beside it as a `BbVec3` array.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbRaceDef {
    /// Bytes the mod filled in.
    pub size: u32,
    /// `RaceType` number. [unconfirmed] list.
    pub kind: u32,
    /// Laps.
    pub laps: u32,
    /// Cash paid for a win.
    pub reward_cash: u32,
    /// Start position, render space, metres.
    pub start: BbVec3,
    /// Start heading, radians.
    pub start_heading: f32,
    /// The registry key, `modid:name`.
    pub key: BbName,
    /// The name shown to the player.
    pub name: BbName,
}

/// `race_count`: write the number of races to `out`.
pub type BbRaceCountFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut u32) -> BbStatus;
/// `race_at`: write the race at `index` to `out`.
pub type BbRaceAtFn = unsafe extern "C" fn(ctx: *mut c_void, index: u32, out: *mut BbRaceInfo) -> BbStatus;
/// `race_checkpoint_count`: write the number of checkpoints of race `id` to `out`.
pub type BbRaceCheckpointCountFn = unsafe extern "C" fn(ctx: *mut c_void, id: u32, out: *mut u32) -> BbStatus;
/// `race_checkpoint_at`: write checkpoint `index` of race `id` to `out`.
pub type BbRaceCheckpointAtFn =
    unsafe extern "C" fn(ctx: *mut c_void, id: u32, index: u32, out: *mut BbVec3) -> BbStatus;
/// `race_start`: start race `id`, as driving into its start would (W).
pub type BbRaceStartFn = unsafe extern "C" fn(ctx: *mut c_void, id: u32) -> BbStatus;
/// `race_abort`: abort the running race (W).
pub type BbRaceAbortFn = unsafe extern "C" fn(ctx: *mut c_void) -> BbStatus;
/// `race_current`: write the running race to `out` (`active` is zero when none runs).
pub type BbRaceCurrentFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut BbRaceState) -> BbStatus;
/// `race_register`: register a custom race; its id goes to `out_id` (W). `checkpoint_count` entries are read from
/// `checkpoints`.
pub type BbRaceRegisterFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    def: *const BbRaceDef,
    checkpoints: *const BbVec3,
    checkpoint_count: u32,
    out_id: *mut u32,
) -> BbStatus;
/// `race_set_opponent_skill`: set the opponents' skill of race `id`, 0 to 1 (W).
pub type BbRaceSetOpponentSkillFn = unsafe extern "C" fn(ctx: *mut c_void, id: u32, skill: f32) -> BbStatus;
/// `race_set_reward`: set the cash for a win of race `id` (W).
pub type BbRaceSetRewardFn = unsafe extern "C" fn(ctx: *mut c_void, id: u32, cash: u32) -> BbStatus;
/// `race_results`: write the result of race `id` to `out`. Answers `BB_ERR_NOT_FOUND` when the race has no result.
pub type BbRaceResultsFn = unsafe extern "C" fn(ctx: *mut c_void, id: u32, out: *mut BbRaceResult) -> BbStatus;

/// The `nfsmw.races` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbNfsmwRaces {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Count the races.
    pub race_count: Option<BbRaceCountFn>,
    /// Read a race.
    pub race_at: Option<BbRaceAtFn>,
    /// Count a race's checkpoints.
    pub race_checkpoint_count: Option<BbRaceCheckpointCountFn>,
    /// Read a checkpoint.
    pub race_checkpoint_at: Option<BbRaceCheckpointAtFn>,
    /// Start a race.
    pub race_start: Option<BbRaceStartFn>,
    /// Abort the running race.
    pub race_abort: Option<BbRaceAbortFn>,
    /// Read the running race.
    pub race_current: Option<BbRaceCurrentFn>,
    /// Register a custom race.
    pub race_register: Option<BbRaceRegisterFn>,
    /// Set the opponents' skill.
    pub race_set_opponent_skill: Option<BbRaceSetOpponentSkillFn>,
    /// Set a race's reward.
    pub race_set_reward: Option<BbRaceSetRewardFn>,
    /// Read a race's result.
    pub race_results: Option<BbRaceResultsFn>,
}
