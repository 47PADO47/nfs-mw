//! `bb.engine` on the mod side: read the table, read the engine and the pedals, switch the engine.

use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{
    BbEngine, BbEngineCarFn, BbEngineStartFn, BbEngineState, BbEngineStateFn, BbEngineStopFn, BbSinkFn, BbStr,
};
use crate::sdk::{Error, Interface, ModResult};

/// The function pointers of one `bb.engine` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct EngineFns {
    ctx: *mut c_void,
    state: Option<BbEngineStateFn>,
    car: Option<BbEngineCarFn>,
    stop: Option<BbEngineStopFn>,
    start: Option<BbEngineStartFn>,
}

/// Read the `bb.engine` table the host offered.
pub(crate) fn read(iface: Interface) -> EngineFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        EngineFns {
            ctx: read_or(&iface, offset_of!(BbEngine, ctx), null_mut()),
            state: read_or(&iface, offset_of!(BbEngine, state), None),
            car: read_or(&iface, offset_of!(BbEngine, car), None),
            stop: read_or(&iface, offset_of!(BbEngine, stop), None),
            start: read_or(&iface, offset_of!(BbEngine, start), None),
        }
    }
}

/// The engine and the pedals as the host reports them now.
pub(crate) fn state(fns: &EngineFns) -> ModResult<BbEngineState> {
    let state = fns.state.ok_or(Error::Unsupported)?;
    let mut out = BbEngineState { size: size_of::<BbEngineState>() as u32, ..BbEngineState::default() };
    // SAFETY: `out` is valid for a whole `BbEngineState`, which its size word says.
    status_result(unsafe { state(fns.ctx, &mut out) })?;
    Ok(out)
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

/// The id of the player's car.
pub(crate) fn car(fns: &EngineFns) -> ModResult<String> {
    let car = fns.car.ok_or(Error::Unsupported)?;
    let mut output = String::new();
    let sink: BbSinkFn = capture;
    // SAFETY: `output` lives for the call, and the sink takes it as `user`.
    status_result(unsafe { car(fns.ctx, Some(sink), (&mut output as *mut String).cast::<c_void>()) })?;
    Ok(output)
}

/// Switch the engine off.
pub(crate) fn stop(fns: &EngineFns) -> ModResult<()> {
    let stop = fns.stop.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded.
    status_result(unsafe { stop(fns.ctx) })
}

/// Start the engine with `sound`, at `gain`, running after `delay` seconds.
pub(crate) fn start(fns: &EngineFns, sound: &str, gain: f32, delay: f32) -> ModResult<()> {
    let start = fns.start.ok_or(Error::Unsupported)?;
    // SAFETY: the sound path lives for the call; the other arguments are plain values.
    status_result(unsafe { start(fns.ctx, BbStr::new(sound), gain, delay) })
}
