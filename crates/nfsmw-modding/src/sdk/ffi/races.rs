//! `nfsmw.races` on the mod side: read the table, and call its functions.

#![allow(unsafe_code)] // ADR 0005 section 8: unsafe confined to sdk::ffi.

use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use blackbox_modding::abi::BbIface;
use blackbox_modding::sdk::{Error, Interface, ModResult};

use super::{check, read_field, sized_out};
use crate::abi::{
    BbNfsmwRaces, BbRaceAbortFn, BbRaceAtFn, BbRaceCheckpointAtFn, BbRaceCheckpointCountFn, BbRaceCountFn,
    BbRaceCurrentFn, BbRaceDef, BbRaceInfo, BbRaceRegisterFn, BbRaceResult, BbRaceResultsFn, BbRaceSetOpponentSkillFn,
    BbRaceSetRewardFn, BbRaceStartFn, BbRaceState, BbVec3,
};

/// The context and function pointers of one `nfsmw.races` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct RacesFns {
    ctx: *mut c_void,
    race_count: Option<BbRaceCountFn>,
    race_at: Option<BbRaceAtFn>,
    race_checkpoint_count: Option<BbRaceCheckpointCountFn>,
    race_checkpoint_at: Option<BbRaceCheckpointAtFn>,
    race_start: Option<BbRaceStartFn>,
    race_abort: Option<BbRaceAbortFn>,
    race_current: Option<BbRaceCurrentFn>,
    race_register: Option<BbRaceRegisterFn>,
    race_set_opponent_skill: Option<BbRaceSetOpponentSkillFn>,
    race_set_reward: Option<BbRaceSetRewardFn>,
    race_results: Option<BbRaceResultsFn>,
}

/// Read the `nfsmw.races` table the host offered.
pub(crate) fn read(iface: &Interface) -> RacesFns {
    // SAFETY: the host keeps an offered table valid while the mod is loaded, and `size` is the table's own size word.
    unsafe { read_raw(iface.as_ptr(), iface.size()) }
}

/// Read the `nfsmw.races` table at `base`.
///
/// # Safety
/// `base` must point to an `nfsmw.races` table that stays valid while the mod is loaded, and `size` its size word.
pub(crate) unsafe fn read_raw(base: *const BbIface, size: u32) -> RacesFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_field`).
    unsafe {
        RacesFns {
            ctx: read_field(base, size, offset_of!(BbNfsmwRaces, ctx), null_mut()),
            race_count: read_field(base, size, offset_of!(BbNfsmwRaces, race_count), None),
            race_at: read_field(base, size, offset_of!(BbNfsmwRaces, race_at), None),
            race_checkpoint_count: read_field(base, size, offset_of!(BbNfsmwRaces, race_checkpoint_count), None),
            race_checkpoint_at: read_field(base, size, offset_of!(BbNfsmwRaces, race_checkpoint_at), None),
            race_start: read_field(base, size, offset_of!(BbNfsmwRaces, race_start), None),
            race_abort: read_field(base, size, offset_of!(BbNfsmwRaces, race_abort), None),
            race_current: read_field(base, size, offset_of!(BbNfsmwRaces, race_current), None),
            race_register: read_field(base, size, offset_of!(BbNfsmwRaces, race_register), None),
            race_set_opponent_skill: read_field(base, size, offset_of!(BbNfsmwRaces, race_set_opponent_skill), None),
            race_set_reward: read_field(base, size, offset_of!(BbNfsmwRaces, race_set_reward), None),
            race_results: read_field(base, size, offset_of!(BbNfsmwRaces, race_results), None),
        }
    }
}

/// The number of races.
pub(crate) fn race_count(fns: &RacesFns) -> ModResult<u32> {
    let call = fns.race_count.ok_or(Error::Unsupported)?;
    let mut out = 0_u32;
    // SAFETY: `out` is valid for one write; the host keeps the table valid while the mod is loaded.
    check(unsafe { call(fns.ctx, &mut out) })?;
    Ok(out)
}

/// The race at `index`.
pub(crate) fn race_at(fns: &RacesFns, index: u32) -> ModResult<BbRaceInfo> {
    let call = fns.race_at.ok_or(Error::Unsupported)?;
    // SAFETY: the record is sized by its own size word (see `sized_out`).
    let mut out: BbRaceInfo = unsafe { sized_out() };
    // SAFETY: `out` is a record the mod owns, sized as above.
    check(unsafe { call(fns.ctx, index, &mut out) })?;
    Ok(out)
}

/// The number of checkpoints of race `id`.
pub(crate) fn race_checkpoint_count(fns: &RacesFns, id: u32) -> ModResult<u32> {
    let call = fns.race_checkpoint_count.ok_or(Error::Unsupported)?;
    let mut out = 0_u32;
    // SAFETY: `out` is valid for one write.
    check(unsafe { call(fns.ctx, id, &mut out) })?;
    Ok(out)
}

/// Checkpoint `index` of race `id`.
pub(crate) fn race_checkpoint_at(fns: &RacesFns, id: u32, index: u32) -> ModResult<BbVec3> {
    let call = fns.race_checkpoint_at.ok_or(Error::Unsupported)?;
    let mut out = BbVec3 { x: 0.0, y: 0.0, z: 0.0 };
    // SAFETY: `out` is valid for one write.
    check(unsafe { call(fns.ctx, id, index, &mut out) })?;
    Ok(out)
}

/// Start race `id`.
pub(crate) fn race_start(fns: &RacesFns, id: u32) -> ModResult<()> {
    let call = fns.race_start.ok_or(Error::Unsupported)?;
    // SAFETY: a scalar argument.
    check(unsafe { call(fns.ctx, id) })
}

/// Abort the running race.
pub(crate) fn race_abort(fns: &RacesFns) -> ModResult<()> {
    let call = fns.race_abort.ok_or(Error::Unsupported)?;
    // SAFETY: no arguments besides the context.
    check(unsafe { call(fns.ctx) })
}

/// The running race.
pub(crate) fn race_current(fns: &RacesFns) -> ModResult<BbRaceState> {
    let call = fns.race_current.ok_or(Error::Unsupported)?;
    // SAFETY: the record is sized by its own size word (see `sized_out`).
    let mut out: BbRaceState = unsafe { sized_out() };
    // SAFETY: `out` is a record the mod owns, sized as above.
    check(unsafe { call(fns.ctx, &mut out) })?;
    Ok(out)
}

/// Register a custom race; returns its id.
pub(crate) fn race_register(fns: &RacesFns, def: &BbRaceDef, checkpoints: &[BbVec3]) -> ModResult<u32> {
    let call = fns.race_register.ok_or(Error::Unsupported)?;
    let mut id = 0_u32;
    // SAFETY: `def`, the checkpoint list and `id` are valid for the call; the host reads `checkpoints.len()` points.
    check(unsafe { call(fns.ctx, def, checkpoints.as_ptr(), checkpoints.len() as u32, &mut id) })?;
    Ok(id)
}

/// Set the opponents' skill of race `id`.
pub(crate) fn race_set_opponent_skill(fns: &RacesFns, id: u32, skill: f32) -> ModResult<()> {
    let call = fns.race_set_opponent_skill.ok_or(Error::Unsupported)?;
    // SAFETY: scalar arguments.
    check(unsafe { call(fns.ctx, id, skill) })
}

/// Set the reward of race `id`.
pub(crate) fn race_set_reward(fns: &RacesFns, id: u32, cash: u32) -> ModResult<()> {
    let call = fns.race_set_reward.ok_or(Error::Unsupported)?;
    // SAFETY: scalar arguments.
    check(unsafe { call(fns.ctx, id, cash) })
}

/// The result of race `id`.
pub(crate) fn race_results(fns: &RacesFns, id: u32) -> ModResult<BbRaceResult> {
    let call = fns.race_results.ok_or(Error::Unsupported)?;
    // SAFETY: the record is sized by its own size word (see `sized_out`).
    let mut out: BbRaceResult = unsafe { sized_out() };
    // SAFETY: `out` is a record the mod owns, sized as above.
    check(unsafe { call(fns.ctx, id, &mut out) })?;
    Ok(out)
}
