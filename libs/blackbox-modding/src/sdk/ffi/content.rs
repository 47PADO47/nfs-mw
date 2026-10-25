//! `bb.content` on the mod side: read the table, and call its function.

use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{BbContent, BbContentRescanFn};
use crate::sdk::{Error, Interface, ModResult};

/// The function pointers of one `bb.content` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct ContentFns {
    ctx: *mut c_void,
    rescan: Option<BbContentRescanFn>,
}

/// Read the `bb.content` table the host offered.
pub(crate) fn read(iface: Interface) -> ContentFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        ContentFns {
            ctx: read_or(&iface, offset_of!(BbContent, ctx), null_mut()),
            rescan: read_or(&iface, offset_of!(BbContent, rescan), None),
        }
    }
}

/// Ask the host to read the mod's folder again.
pub(crate) fn rescan(fns: &ContentFns) -> ModResult<()> {
    let rescan = fns.rescan.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded; the call takes no other argument.
    status_result(unsafe { rescan(fns.ctx) })
}
