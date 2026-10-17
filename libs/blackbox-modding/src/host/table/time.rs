//! `bb.time` on the host side: the backend the game implements, and the trampolines behind `BbTime`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, guard, slot_of};
use crate::abi::{BB_ERR_ARG, BbIface, BbStatus, BbTime, BbTimeOfDay, IFACE_TIME, IFACE_TIME_VERSION};

/// What the game implements for `bb.time`. `owner` is the id of the mod that made the call.
pub trait TimeBackend {
    /// The current clock.
    fn get(&self, owner: &str) -> BackendResult<BbTimeOfDay>;
    /// Set the hour, already checked to be finite. The game wraps it into 0 to 24.
    fn set_hour(&self, owner: &str, hour: f32) -> BackendResult<()>;
    /// Set the real seconds per in-world hour, already checked to be finite and not negative.
    fn set_day_length(&self, owner: &str, seconds_per_hour: f32) -> BackendResult<()>;
    /// Hold the hour, or let it run.
    fn freeze(&self, owner: &str, on: bool) -> BackendResult<()>;
}

/// Build the `bb.time` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn TimeBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbTime {
        head: BbIface { size: size_of::<BbTime>() as u32, major: IFACE_TIME_VERSION.0, minor: IFACE_TIME_VERSION.1 },
        ctx: Slot::ctx(&slot),
        get: Some(get),
        set_hour: Some(set_hour),
        set_day_length: Some(set_day_length),
        freeze: Some(freeze),
    });
    // Time keeps no registrations, so nothing is removed on unload; the release keeps the slot alive for the table.
    let kept = slot.clone();
    let release = Box::new(move || kept.release_with(|_, _, _| {}));
    Attached::new(IFACE_TIME, IFACE_TIME_VERSION, table, release)
}

/// A value the mod sends must be a finite number.
fn finite(value: f32) -> Result<f32, BbStatus> {
    if !value.is_finite() {
        return Err(BB_ERR_ARG);
    }
    Ok(value)
}

/// `get`: write the clock to the caller's memory.
unsafe extern "C" fn get(ctx: *mut c_void, out: *mut BbTimeOfDay) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn TimeBackend>(ctx)? };
        if out.is_null() {
            return Err(BB_ERR_ARG);
        }
        let clock = slot.backend.get(&slot.owner)?;
        // SAFETY: `out` is non-null (checked above) and valid for one write of a `BbTimeOfDay`.
        unsafe { out.write(clock) };
        Ok(())
    })
}

/// `set_hour`.
unsafe extern "C" fn set_hour(ctx: *mut c_void, hour: f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn TimeBackend>(ctx)? };
        slot.backend.set_hour(&slot.owner, finite(hour)?)
    })
}

/// `set_day_length`.
unsafe extern "C" fn set_day_length(ctx: *mut c_void, seconds_per_hour: f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn TimeBackend>(ctx)? };
        let seconds = finite(seconds_per_hour)?;
        if seconds < 0.0 {
            return Err(BB_ERR_ARG);
        }
        slot.backend.set_day_length(&slot.owner, seconds)
    })
}

/// `freeze`: any non-zero value holds the hour.
unsafe extern "C" fn freeze(ctx: *mut c_void, on: u8) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn TimeBackend>(ctx)? };
        slot.backend.freeze(&slot.owner, on != 0)
    })
}
