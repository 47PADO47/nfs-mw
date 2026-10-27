//! `bb.config` on the mod side: read the table, read the merged configuration text and the bits of its sources.

use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{BbConfig, BbConfigReadFn, BbConfigSourcesFn, BbSinkFn, BbStr};
use crate::sdk::{Error, Interface, ModResult};

/// The function pointers of one `bb.config` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct ConfigFns {
    ctx: *mut c_void,
    read: Option<BbConfigReadFn>,
    sources: Option<BbConfigSourcesFn>,
}

/// Read the `bb.config` table the host offered.
pub(crate) fn read(iface: Interface) -> ConfigFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        ConfigFns {
            ctx: read_or(&iface, offset_of!(BbConfig, ctx), null_mut()),
            read: read_or(&iface, offset_of!(BbConfig, read), None),
            sources: read_or(&iface, offset_of!(BbConfig, sources), None),
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

/// The merged configuration of the mod, as TOML text.
pub(crate) fn text(fns: &ConfigFns) -> ModResult<String> {
    let read = fns.read.ok_or(Error::Unsupported)?;
    let mut output = String::new();
    let sink: BbSinkFn = capture;
    // SAFETY: `output` lives for the call, and the sink takes it as `user`.
    status_result(unsafe { read(fns.ctx, Some(sink), (&mut output as *mut String).cast::<c_void>()) })?;
    Ok(output)
}

/// The `BB_CONFIG_*` bits of the sources that exist.
pub(crate) fn sources(fns: &ConfigFns) -> ModResult<u32> {
    let sources = fns.sources.ok_or(Error::Unsupported)?;
    let mut out = 0;
    // SAFETY: `out` is a live local, valid for one write.
    status_result(unsafe { sources(fns.ctx, &mut out) })?;
    Ok(out)
}
