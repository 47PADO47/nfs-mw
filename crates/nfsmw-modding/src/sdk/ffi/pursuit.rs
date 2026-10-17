//! `nfsmw.pursuit` on the mod side: read the table, and call its functions.

#![allow(unsafe_code)] // ADR 0005 section 8: unsafe confined to sdk::ffi.

use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use blackbox_modding::abi::BbIface;
use blackbox_modding::sdk::{Error, Interface, ModResult};

use super::{check, read_field, sized_out, zeroed};
use crate::abi::{
    BbCopRecord, BbHeatRow, BbNfsmwPursuit, BbPursuitBustedGetFn, BbPursuitCopTableGetFn, BbPursuitCopTableSetFn,
    BbPursuitCopsInPursuitFn, BbPursuitEndFn, BbPursuitHeatGetFn, BbPursuitHeatSetFn, BbPursuitIsPursuingFn,
    BbPursuitStartFn,
};

/// The context and function pointers of one `nfsmw.pursuit` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct PursuitFns {
    ctx: *mut c_void,
    heat_get: Option<BbPursuitHeatGetFn>,
    heat_set: Option<BbPursuitHeatSetFn>,
    cop_table_get: Option<BbPursuitCopTableGetFn>,
    cop_table_set: Option<BbPursuitCopTableSetFn>,
    start: Option<BbPursuitStartFn>,
    end: Option<BbPursuitEndFn>,
    is_pursuing: Option<BbPursuitIsPursuingFn>,
    busted_get: Option<BbPursuitBustedGetFn>,
    cops_in_pursuit: Option<BbPursuitCopsInPursuitFn>,
}

/// Read the `nfsmw.pursuit` table the host offered.
pub(crate) fn read(iface: &Interface) -> PursuitFns {
    // SAFETY: the host keeps an offered table valid while the mod is loaded, and `size` is the table's own size word.
    unsafe { read_raw(iface.as_ptr(), iface.size()) }
}

/// Read the `nfsmw.pursuit` table at `base`.
///
/// # Safety
/// `base` must point to an `nfsmw.pursuit` table that stays valid while the mod is loaded, and `size` its size word.
pub(crate) unsafe fn read_raw(base: *const BbIface, size: u32) -> PursuitFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_field`).
    unsafe {
        PursuitFns {
            ctx: read_field(base, size, offset_of!(BbNfsmwPursuit, ctx), null_mut()),
            heat_get: read_field(base, size, offset_of!(BbNfsmwPursuit, heat_get), None),
            heat_set: read_field(base, size, offset_of!(BbNfsmwPursuit, heat_set), None),
            cop_table_get: read_field(base, size, offset_of!(BbNfsmwPursuit, cop_table_get), None),
            cop_table_set: read_field(base, size, offset_of!(BbNfsmwPursuit, cop_table_set), None),
            start: read_field(base, size, offset_of!(BbNfsmwPursuit, start), None),
            end: read_field(base, size, offset_of!(BbNfsmwPursuit, end), None),
            is_pursuing: read_field(base, size, offset_of!(BbNfsmwPursuit, is_pursuing), None),
            busted_get: read_field(base, size, offset_of!(BbNfsmwPursuit, busted_get), None),
            cops_in_pursuit: read_field(base, size, offset_of!(BbNfsmwPursuit, cops_in_pursuit), None),
        }
    }
}

/// The heat.
pub(crate) fn heat_get(fns: &PursuitFns) -> ModResult<f32> {
    let call = fns.heat_get.ok_or(Error::Unsupported)?;
    let mut out = 0.0_f32;
    // SAFETY: the host keeps the table valid while the mod is loaded; `out` is valid for one write.
    check(unsafe { call(fns.ctx, &mut out) })?;
    Ok(out)
}

/// Set the heat.
pub(crate) fn heat_set(fns: &PursuitFns, heat: f32) -> ModResult<()> {
    let call = fns.heat_set.ok_or(Error::Unsupported)?;
    // SAFETY: a scalar argument; the table is valid while the mod is loaded.
    check(unsafe { call(fns.ctx, heat) })
}

/// One cop wave: the header, and every record of it. Two calls: the first learns the count, the second takes the records.
pub(crate) fn cop_table_get(fns: &PursuitFns, table: u32, level: u32) -> ModResult<(BbHeatRow, Vec<BbCopRecord>)> {
    let call = fns.cop_table_get.ok_or(Error::Unsupported)?;
    // SAFETY: the record is sized by its own size word (see `sized_out`).
    let mut count_row: BbHeatRow = unsafe { sized_out() };
    // SAFETY: a count-only call writes no record, so a null list with a zero capacity is valid.
    check(unsafe { call(fns.ctx, table, level, &mut count_row, null_mut(), 0) })?;
    let total = count_row.cop_count as usize;
    // SAFETY: `BbCopRecord` is plain data, so all-zero bytes are a valid record.
    let mut cops: Vec<BbCopRecord> = vec![unsafe { zeroed() }; total];
    // SAFETY: the record is sized by its own size word (see `sized_out`).
    let mut row: BbHeatRow = unsafe { sized_out() };
    // SAFETY: `cops` holds `total` writable records, and the host writes at most that many.
    check(unsafe { call(fns.ctx, table, level, &mut row, cops.as_mut_ptr(), total as u32) })?;
    Ok((row, cops))
}

/// Replace one cop wave. `cops.len()` must equal `row.cop_count`.
pub(crate) fn cop_table_set(
    fns: &PursuitFns,
    table: u32,
    level: u32,
    row: &BbHeatRow,
    cops: &[BbCopRecord],
) -> ModResult<()> {
    let call = fns.cop_table_set.ok_or(Error::Unsupported)?;
    // SAFETY: the host reads one header and `cops.len()` records, both valid for the call.
    check(unsafe { call(fns.ctx, table, level, row, cops.as_ptr(), cops.len() as u32) })
}

/// Start a pursuit; `near` is a vehicle handle, or 0 for none.
pub(crate) fn start(fns: &PursuitFns, level: u32, near: u64) -> ModResult<()> {
    let call = fns.start.ok_or(Error::Unsupported)?;
    // SAFETY: scalar arguments; the table is valid while the mod is loaded.
    check(unsafe { call(fns.ctx, level, near) })
}

/// End the running pursuit by a mode number.
pub(crate) fn end(fns: &PursuitFns, mode: u32) -> ModResult<()> {
    let call = fns.end.ok_or(Error::Unsupported)?;
    // SAFETY: a scalar argument; the table is valid while the mod is loaded.
    check(unsafe { call(fns.ctx, mode) })
}

/// Whether a chase runs.
pub(crate) fn is_pursuing(fns: &PursuitFns) -> ModResult<bool> {
    let call = fns.is_pursuing.ok_or(Error::Unsupported)?;
    let mut out = 0_u8;
    // SAFETY: `out` is valid for one write.
    check(unsafe { call(fns.ctx, &mut out) })?;
    Ok(out != 0)
}

/// The busted meter.
pub(crate) fn busted_get(fns: &PursuitFns) -> ModResult<f32> {
    let call = fns.busted_get.ok_or(Error::Unsupported)?;
    let mut out = 0.0_f32;
    // SAFETY: `out` is valid for one write.
    check(unsafe { call(fns.ctx, &mut out) })?;
    Ok(out)
}

/// The cops of the running pursuit.
pub(crate) fn cops_in_pursuit(fns: &PursuitFns) -> ModResult<u32> {
    let call = fns.cops_in_pursuit.ok_or(Error::Unsupported)?;
    let mut out = 0_u32;
    // SAFETY: `out` is valid for one write.
    check(unsafe { call(fns.ctx, &mut out) })?;
    Ok(out)
}
