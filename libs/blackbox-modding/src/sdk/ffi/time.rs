//! `bb.time` on the mod side: read the table, and call its clock functions.

use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{BbTime, BbTimeFreezeFn, BbTimeGetFn, BbTimeOfDay, BbTimeSetDayLengthFn, BbTimeSetHourFn};
use crate::sdk::{Error, Interface, ModResult};

/// The function pointers of one `bb.time` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct TimeFns {
    ctx: *mut c_void,
    get: Option<BbTimeGetFn>,
    set_hour: Option<BbTimeSetHourFn>,
    set_day_length: Option<BbTimeSetDayLengthFn>,
    freeze: Option<BbTimeFreezeFn>,
}

/// Read the `bb.time` table the host offered.
pub(crate) fn read(iface: Interface) -> TimeFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        TimeFns {
            ctx: read_or(&iface, offset_of!(BbTime, ctx), null_mut()),
            get: read_or(&iface, offset_of!(BbTime, get), None),
            set_hour: read_or(&iface, offset_of!(BbTime, set_hour), None),
            set_day_length: read_or(&iface, offset_of!(BbTime, set_day_length), None),
            freeze: read_or(&iface, offset_of!(BbTime, freeze), None),
        }
    }
}

/// The clock as the host reports it now.
pub(crate) fn get(fns: &TimeFns) -> ModResult<BbTimeOfDay> {
    let get = fns.get.ok_or(Error::Unsupported)?;
    let mut clock = BbTimeOfDay { hour: 0.0, seconds_per_hour: 0.0, frozen: 0, lights_on: 0, _pad: [0; 2] };
    // SAFETY: `clock` is valid for one write, and the host writes a whole `BbTimeOfDay` into it.
    status_result(unsafe { get(fns.ctx, &mut clock) })?;
    Ok(clock)
}

/// Set the hour.
pub(crate) fn set_hour(fns: &TimeFns, hour: f32) -> ModResult<()> {
    let set_hour = fns.set_hour.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded; a scalar argument needs no other check.
    status_result(unsafe { set_hour(fns.ctx, hour) })
}

/// Set the real seconds per in-world hour.
pub(crate) fn set_day_length(fns: &TimeFns, seconds_per_hour: f32) -> ModResult<()> {
    let set_day_length = fns.set_day_length.ok_or(Error::Unsupported)?;
    // SAFETY: as for `set_hour`.
    status_result(unsafe { set_day_length(fns.ctx, seconds_per_hour) })
}

/// Hold or release the hour.
pub(crate) fn freeze(fns: &TimeFns, on: bool) -> ModResult<()> {
    let freeze = fns.freeze.ok_or(Error::Unsupported)?;
    // SAFETY: as for `set_hour`.
    status_result(unsafe { freeze(fns.ctx, u8::from(on)) })
}
