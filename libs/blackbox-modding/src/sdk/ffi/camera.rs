//! `bb.camera` on the mod side: read the table, read the camera, choose a view.

use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{BbCamera, BbCameraGetFn, BbCameraSetModeFn, BbCameraState};
use crate::sdk::{Error, Interface, ModResult};

/// The function pointers of one `bb.camera` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct CameraFns {
    ctx: *mut c_void,
    get: Option<BbCameraGetFn>,
    set_mode: Option<BbCameraSetModeFn>,
}

/// Read the `bb.camera` table the host offered.
pub(crate) fn read(iface: Interface) -> CameraFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        CameraFns {
            ctx: read_or(&iface, offset_of!(BbCamera, ctx), null_mut()),
            get: read_or(&iface, offset_of!(BbCamera, get), None),
            set_mode: read_or(&iface, offset_of!(BbCamera, set_mode), None),
        }
    }
}

/// The camera now, as the host wrote it (the mode is still the raw `BB_CAMERA_*` number).
pub(crate) fn get(fns: &CameraFns) -> ModResult<BbCameraState> {
    let get = fns.get.ok_or(Error::Unsupported)?;
    let mut raw = BbCameraState { size: size_of::<BbCameraState>() as u32, ..BbCameraState::default() };
    // SAFETY: `raw` is valid for one write of its whole layout, and its size word says so.
    status_result(unsafe { get(fns.ctx, &mut raw) })?;
    Ok(raw)
}

/// Switch to the view `mode` names.
pub(crate) fn set_mode(fns: &CameraFns, mode: u32) -> ModResult<()> {
    let set_mode = fns.set_mode.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded; a scalar argument needs no other check.
    status_result(unsafe { set_mode(fns.ctx, mode) })
}
