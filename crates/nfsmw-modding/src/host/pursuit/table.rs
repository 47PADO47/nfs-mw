//! `nfsmw.pursuit` on the host side: the trampolines behind [`BbNfsmwPursuit`] and the builder that makes the table.

#![allow(unsafe_code)] // ADR 0005 section 8: unsafe confined to host table, io and slot modules.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use blackbox_modding::abi::{BbIface, BbStatus};

use super::check;
use super::{HeatRowSpec, PursuitBackend};
use crate::abi::{BbCopRecord, BbHeatRow, BbNfsmwPursuit, EndMode, IFACE_PURSUIT_VERSION, MW_MAX_COPS_PER_ROW};
use crate::host::io::{put, put_sized, read_list, read_sized, write_list};
use crate::host::slot::{Slot, guard, slot_of};

/// A built `nfsmw.pursuit` table for one mod. It keeps the table and the backend alive: drop it and the table is gone.
pub struct PursuitTable {
    table: Box<BbNfsmwPursuit>,
    _slot: Rc<Slot<dyn PursuitBackend>>,
}

impl PursuitTable {
    /// The header of the table: the pointer `query_interface` answers with.
    pub fn header(&self) -> *const BbIface {
        core::ptr::from_ref(&*self.table).cast::<BbIface>()
    }
}

/// Build the `nfsmw.pursuit` table for mod `owner`, backed by `backend`.
pub fn pursuit_table(backend: Rc<dyn PursuitBackend>, owner: &str) -> PursuitTable {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbNfsmwPursuit {
        head: BbIface {
            size: size_of::<BbNfsmwPursuit>() as u32,
            major: IFACE_PURSUIT_VERSION.0,
            minor: IFACE_PURSUIT_VERSION.1,
        },
        ctx: Slot::ctx(&slot),
        heat_get: Some(heat_get),
        heat_set: Some(heat_set),
        cop_table_get: Some(cop_table_get),
        cop_table_set: Some(cop_table_set),
        start: Some(start),
        end: Some(end),
        is_pursuing: Some(is_pursuing),
        busted_get: Some(busted_get),
        cops_in_pursuit: Some(cops_in_pursuit),
    });
    PursuitTable { table, _slot: slot }
}

/// `heat_get`: write the heat to `out`.
unsafe extern "C" fn heat_get(ctx: *mut c_void, out: *mut f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn PursuitBackend>(ctx)? };
        let heat = slot.backend.heat_get(&slot.owner)?;
        // SAFETY: `out` is checked for null by `put`; a non-null pointer is valid for one write (the mod's contract).
        unsafe { put(out, heat) }
    })
}

/// `heat_set`: set the heat; a non-finite value is refused.
unsafe extern "C" fn heat_set(ctx: *mut c_void, heat: f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn PursuitBackend>(ctx)? };
        slot.backend.heat_set(&slot.owner, check::finite(heat)?)
    })
}

/// `cop_table_get`: write the header of the wave to `out`, and up to `cops_cap` records to `cops`.
unsafe extern "C" fn cop_table_get(
    ctx: *mut c_void,
    table: u32,
    level: u32,
    out: *mut BbHeatRow,
    cops: *mut BbCopRecord,
    cops_cap: u32,
) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn PursuitBackend>(ctx)? };
        let table = check::table(table)?;
        let level = check::level(level)?;
        let mut spec = slot.backend.cop_table_get(&slot.owner, table, level)?;
        spec.row.cop_count = spec.cops.len() as u32;
        // SAFETY: `out` is a header the mod owns, sized by its own size word (see `put_sized`).
        unsafe { put_sized(out, &spec.row)? };
        // SAFETY: `cops` holds `cops_cap` writable records, or is null with a zero capacity (see `write_list`).
        unsafe { write_list(cops, cops_cap, &spec.cops) }
    })
}

/// `cop_table_set`: replace the wave of `table` at `level`, after the checks of [`check::wave`].
unsafe extern "C" fn cop_table_set(
    ctx: *mut c_void,
    table: u32,
    level: u32,
    row: *const BbHeatRow,
    cops: *const BbCopRecord,
    cops_len: u32,
) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn PursuitBackend>(ctx)? };
        let table = check::table(table)?;
        let level = check::level(level)?;
        // SAFETY: `row` is the mod's header, read by its own size word (see `read_sized`).
        let row = unsafe { read_sized(row)? };
        // SAFETY: `cops` holds `cops_len` readable records, or is null with a zero length (see `read_list`).
        let cops = unsafe { read_list(cops, cops_len, MW_MAX_COPS_PER_ROW)? };
        check::wave(&row, &cops)?;
        let spec = HeatRowSpec { row, cops };
        slot.backend.cop_table_set(&slot.owner, table, level, &spec)
    })
}

/// `start`: start a pursuit at `level`. A zero `near` means no vehicle.
unsafe extern "C" fn start(ctx: *mut c_void, level: u32, near: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn PursuitBackend>(ctx)? };
        let level = check::level(level)?;
        let near = (near != 0).then_some(near);
        slot.backend.start(&slot.owner, level, near)
    })
}

/// `end`: end the running pursuit by a known mode.
unsafe extern "C" fn end(ctx: *mut c_void, mode: u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn PursuitBackend>(ctx)? };
        let mode = EndMode::from_raw(mode).ok_or(blackbox_modding::abi::BB_ERR_ARG)?;
        slot.backend.end(&slot.owner, mode)
    })
}

/// `is_pursuing`: write 1 or 0 to `out`.
unsafe extern "C" fn is_pursuing(ctx: *mut c_void, out: *mut u8) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn PursuitBackend>(ctx)? };
        let on = slot.backend.is_pursuing(&slot.owner)?;
        // SAFETY: `out` is checked for null by `put`; a non-null pointer is valid for one write.
        unsafe { put(out, u8::from(on)) }
    })
}

/// `busted_get`: write the busted meter to `out`.
unsafe extern "C" fn busted_get(ctx: *mut c_void, out: *mut f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn PursuitBackend>(ctx)? };
        let meter = slot.backend.busted_get(&slot.owner)?;
        // SAFETY: `out` is checked for null by `put`; a non-null pointer is valid for one write.
        unsafe { put(out, meter) }
    })
}

/// `cops_in_pursuit`: write the cop count to `out`.
unsafe extern "C" fn cops_in_pursuit(ctx: *mut c_void, out: *mut u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn PursuitBackend>(ctx)? };
        let count = slot.backend.cops_in_pursuit(&slot.owner)?;
        // SAFETY: `out` is checked for null by `put`; a non-null pointer is valid for one write.
        unsafe { put(out, count) }
    })
}
