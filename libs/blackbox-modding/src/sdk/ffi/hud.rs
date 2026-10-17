//! `bb.hud` on the mod side: read the table, show a message, hide it.

use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{BbHud, BbHudHideFn, BbHudMessageFn, BbStr};
use crate::sdk::{Error, Interface, ModResult};

/// The function pointers of one `bb.hud` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct HudFns {
    ctx: *mut c_void,
    message: Option<BbHudMessageFn>,
    hide: Option<BbHudHideFn>,
}

/// Read the `bb.hud` table the host offered.
pub(crate) fn read(iface: Interface) -> HudFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        HudFns {
            ctx: read_or(&iface, offset_of!(BbHud, ctx), null_mut()),
            message: read_or(&iface, offset_of!(BbHud, message), None),
            hide: read_or(&iface, offset_of!(BbHud, hide), None),
        }
    }
}

/// Show `text` in `style` (a `BB_HUD_*` number) for `secs` seconds.
pub(crate) fn message(fns: &HudFns, text: &str, style: u32, secs: f32) -> ModResult<()> {
    let message = fns.message.ok_or(Error::Unsupported)?;
    // SAFETY: the text lives for the call, which is the contract of `BbStr`.
    status_result(unsafe { message(fns.ctx, BbStr::new(text), style, secs) })
}

/// Take the message down.
pub(crate) fn hide(fns: &HudFns) -> ModResult<()> {
    let hide = fns.hide.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded.
    status_result(unsafe { hide(fns.ctx) })
}
