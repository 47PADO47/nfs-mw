//! `nfsmw.races` on the host side: the trampolines behind [`BbNfsmwRaces`] and the builder that makes the table.

#![allow(unsafe_code)] // ADR 0005 section 8: unsafe confined to host table, io and slot modules.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use blackbox_modding::abi::{BbIface, BbStatus};

use super::RacesBackend;
use crate::abi::{
    BbNfsmwRaces, BbRaceDef, BbRaceInfo, BbRaceResult, BbRaceState, BbVec3, IFACE_RACES_VERSION, MW_MAX_CHECKPOINTS,
};
use crate::host::io::{put, put_sized, read_list, read_sized};
use crate::host::slot::{Slot, guard, slot_of};

/// A built `nfsmw.races` table for one mod. It keeps the table and the backend alive.
pub struct RacesTable {
    table: Box<BbNfsmwRaces>,
    _slot: Rc<Slot<dyn RacesBackend>>,
}

impl RacesTable {
    /// The header of the table: the pointer `query_interface` answers with.
    pub fn header(&self) -> *const BbIface {
        core::ptr::from_ref(&*self.table).cast::<BbIface>()
    }
}

/// Build the `nfsmw.races` table for mod `owner`, backed by `backend`.
pub fn races_table(backend: Rc<dyn RacesBackend>, owner: &str) -> RacesTable {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbNfsmwRaces {
        head: BbIface {
            size: size_of::<BbNfsmwRaces>() as u32,
            major: IFACE_RACES_VERSION.0,
            minor: IFACE_RACES_VERSION.1,
        },
        ctx: Slot::ctx(&slot),
        race_count: Some(race_count),
        race_at: Some(race_at),
        race_checkpoint_count: Some(race_checkpoint_count),
        race_checkpoint_at: Some(race_checkpoint_at),
        race_start: Some(race_start),
        race_abort: Some(race_abort),
        race_current: Some(race_current),
        race_register: Some(race_register),
        race_set_opponent_skill: Some(race_set_opponent_skill),
        race_set_reward: Some(race_set_reward),
        race_results: Some(race_results),
    });
    RacesTable { table, _slot: slot }
}

/// `race_count`.
unsafe extern "C" fn race_count(ctx: *mut c_void, out: *mut u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn RacesBackend>(ctx)? };
        let count = slot.backend.race_count(&slot.owner)?;
        // SAFETY: `out` is checked for null by `put`.
        unsafe { put(out, count) }
    })
}

/// `race_at`.
unsafe extern "C" fn race_at(ctx: *mut c_void, index: u32, out: *mut BbRaceInfo) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn RacesBackend>(ctx)? };
        let race = slot.backend.race_at(&slot.owner, index)?;
        // SAFETY: `out` is a record the mod sized (see `put_sized`).
        unsafe { put_sized(out, &race) }
    })
}

/// `race_checkpoint_count`.
unsafe extern "C" fn race_checkpoint_count(ctx: *mut c_void, id: u32, out: *mut u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn RacesBackend>(ctx)? };
        let count = slot.backend.race_checkpoint_count(&slot.owner, id)?;
        // SAFETY: `out` is checked for null by `put`.
        unsafe { put(out, count) }
    })
}

/// `race_checkpoint_at`.
unsafe extern "C" fn race_checkpoint_at(ctx: *mut c_void, id: u32, index: u32, out: *mut BbVec3) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn RacesBackend>(ctx)? };
        let point = slot.backend.race_checkpoint_at(&slot.owner, id, index)?;
        // SAFETY: `out` is checked for null by `put`.
        unsafe { put(out, point) }
    })
}

/// `race_start`.
unsafe extern "C" fn race_start(ctx: *mut c_void, id: u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn RacesBackend>(ctx)? };
        slot.backend.race_start(&slot.owner, id)
    })
}

/// `race_abort`.
unsafe extern "C" fn race_abort(ctx: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn RacesBackend>(ctx)? };
        slot.backend.race_abort(&slot.owner)
    })
}

/// `race_current`.
unsafe extern "C" fn race_current(ctx: *mut c_void, out: *mut BbRaceState) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn RacesBackend>(ctx)? };
        let state = slot.backend.race_current(&slot.owner)?;
        // SAFETY: `out` is a record the mod sized (see `put_sized`).
        unsafe { put_sized(out, &state) }
    })
}

/// `race_register`: the definition is read by its size word, the checkpoints as a list, the id goes to `out_id`.
unsafe extern "C" fn race_register(
    ctx: *mut c_void,
    def: *const BbRaceDef,
    checkpoints: *const BbVec3,
    checkpoint_count: u32,
    out_id: *mut u32,
) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn RacesBackend>(ctx)? };
        // SAFETY: `def` is the mod's record, read by its own size word (see `read_sized`).
        let def = unsafe { read_sized(def)? };
        // SAFETY: `checkpoints` holds `checkpoint_count` readable points, or is null for none (see `read_list`).
        let points = unsafe { read_list(checkpoints, checkpoint_count, MW_MAX_CHECKPOINTS)? };
        let id = slot.backend.race_register(&slot.owner, &def, &points)?;
        // SAFETY: `out_id` is checked for null by `put`.
        unsafe { put(out_id, id) }
    })
}

/// `race_set_opponent_skill`.
unsafe extern "C" fn race_set_opponent_skill(ctx: *mut c_void, id: u32, skill: f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn RacesBackend>(ctx)? };
        slot.backend.race_set_opponent_skill(&slot.owner, id, skill)
    })
}

/// `race_set_reward`.
unsafe extern "C" fn race_set_reward(ctx: *mut c_void, id: u32, cash: u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn RacesBackend>(ctx)? };
        slot.backend.race_set_reward(&slot.owner, id, cash)
    })
}

/// `race_results`: a size-prefixed record.
unsafe extern "C" fn race_results(ctx: *mut c_void, id: u32, out: *mut BbRaceResult) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn RacesBackend>(ctx)? };
        let result = slot.backend.race_results(&slot.owner, id)?;
        // SAFETY: `out` is a record the mod sized (see `put_sized`).
        unsafe { put_sized(out, &result) }
    })
}
