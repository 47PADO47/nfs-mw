//! `bb.world` on the mod side: read the table, read the track and the district, set the rain and the traffic.

use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{BbSinkFn, BbStr, BbWorld, BbWorldSetRainFn, BbWorldSetTrafficFn, BbWorldTextFn};
use crate::sdk::{Error, Interface, ModResult};

/// The function pointers of one `bb.world` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct WorldFns {
    ctx: *mut c_void,
    track: Option<BbWorldTextFn>,
    section: Option<BbWorldTextFn>,
    set_rain: Option<BbWorldSetRainFn>,
    set_traffic: Option<BbWorldSetTrafficFn>,
}

/// Read the `bb.world` table the host offered.
pub(crate) fn read(iface: Interface) -> WorldFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        WorldFns {
            ctx: read_or(&iface, offset_of!(BbWorld, ctx), null_mut()),
            track: read_or(&iface, offset_of!(BbWorld, track), None),
            section: read_or(&iface, offset_of!(BbWorld, section), None),
            set_rain: read_or(&iface, offset_of!(BbWorld, set_rain), None),
            set_traffic: read_or(&iface, offset_of!(BbWorld, set_traffic), None),
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

/// Run one of the name calls and return the name it wrote.
fn name(ctx: *mut c_void, call: Option<BbWorldTextFn>) -> ModResult<String> {
    let call = call.ok_or(Error::Unsupported)?;
    let mut output = String::new();
    let sink: BbSinkFn = capture;
    // SAFETY: `output` lives for the call, and the sink takes it as `user`.
    status_result(unsafe { call(ctx, Some(sink), (&mut output as *mut String).cast::<c_void>()) })?;
    Ok(output)
}

/// The track's name.
pub(crate) fn track(fns: &WorldFns) -> ModResult<String> {
    name(fns.ctx, fns.track)
}

/// The district's name at the player's position.
pub(crate) fn section(fns: &WorldFns) -> ModResult<String> {
    name(fns.ctx, fns.section)
}

/// Set the rain: `always` keeps it raining; `amount` is 0 to 1.
pub(crate) fn set_rain(fns: &WorldFns, always: bool, amount: f32) -> ModResult<()> {
    let set_rain = fns.set_rain.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded; the arguments are plain values.
    status_result(unsafe { set_rain(fns.ctx, u8::from(always), amount) })
}

/// Set the traffic: `cars` cars, `cop_share` percent of them cops.
pub(crate) fn set_traffic(fns: &WorldFns, cars: u32, cop_share: u32) -> ModResult<()> {
    let set_traffic = fns.set_traffic.ok_or(Error::Unsupported)?;
    // SAFETY: as for `set_rain`.
    status_result(unsafe { set_traffic(fns.ctx, cars, cop_share) })
}
