//! `bb.traffic` on the mod side: read the table, list the pool, ask for a car.

use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{BbSinkFn, BbStr, BbTraffic, BbTrafficListFn, BbTrafficSpawnFn, BbVec3};
use crate::sdk::{Error, Interface, ModResult};

/// The function pointers of one `bb.traffic` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct TrafficFns {
    ctx: *mut c_void,
    list: Option<BbTrafficListFn>,
    spawn: Option<BbTrafficSpawnFn>,
}

/// Read the `bb.traffic` table the host offered.
pub(crate) fn read(iface: Interface) -> TrafficFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        TrafficFns {
            ctx: read_or(&iface, offset_of!(BbTraffic, ctx), null_mut()),
            list: read_or(&iface, offset_of!(BbTraffic, list), None),
            spawn: read_or(&iface, offset_of!(BbTraffic, spawn), None),
        }
    }
}

/// A text the host writes to the sink during the call: the sink appends it to the `String` the user pointer names.
unsafe extern "C" fn capture(user: *mut c_void, text: BbStr) {
    // SAFETY: `user` is the `String` the reader passed in, alive for the call.
    let Some(output) = (unsafe { user.cast::<String>().as_mut() }) else {
        return;
    };
    // SAFETY: the host passes a string valid for this call.
    let Some(text) = (unsafe { text.as_str() }) else {
        return;
    };
    output.push_str(text);
}

/// The text of the pool: a line per car.
pub(crate) fn list(fns: &TrafficFns) -> ModResult<String> {
    let list = fns.list.ok_or(Error::Unsupported)?;
    let mut output = String::new();
    let sink: BbSinkFn = capture;
    // SAFETY: `output` lives for the call, and the sink takes it as `user`.
    status_result(unsafe { list(fns.ctx, Some(sink), (&mut output as *mut String).cast::<c_void>()) })?;
    Ok(output)
}

/// Ask for the car `car` at `position`, heading `heading`.
pub(crate) fn spawn(fns: &TrafficFns, car: &str, position: [f32; 3], heading: f32) -> ModResult<()> {
    let spawn = fns.spawn.ok_or(Error::Unsupported)?;
    let at = BbVec3 { x: position[0], y: position[1], z: position[2] };
    // SAFETY: the name lives for the call; the other arguments are plain values.
    status_result(unsafe { spawn(fns.ctx, BbStr::new(car), at, heading) })
}
