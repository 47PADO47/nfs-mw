//! `bb.saves` on the mod side: read the table, and read, write and remove the mod's own blob.

use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{BB_ERR_NOT_FOUND, BbSaves, BbSavesReadFn, BbSavesRemoveFn, BbSavesWriteFn, MAX_BLOB_BYTES};
use crate::sdk::{Error, Interface, ModResult};

/// The function pointers of one `bb.saves` table, read once. Each is `None` when the host does not provide it.
#[derive(Clone, Copy)]
pub(crate) struct SavesFns {
    ctx: *mut c_void,
    read: Option<BbSavesReadFn>,
    write: Option<BbSavesWriteFn>,
    remove: Option<BbSavesRemoveFn>,
}

/// Read the `bb.saves` table the host offered.
pub(crate) fn read(iface: Interface) -> SavesFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        SavesFns {
            ctx: read_or(&iface, offset_of!(BbSaves, ctx), null_mut()),
            read: read_or(&iface, offset_of!(BbSaves, read), None),
            write: read_or(&iface, offset_of!(BbSaves, write), None),
            remove: read_or(&iface, offset_of!(BbSaves, remove), None),
        }
    }
}

/// The mod's blob, or `None` when the mod has written none.
pub(crate) fn read_blob(fns: &SavesFns) -> ModResult<Option<Vec<u8>>> {
    let read = fns.read.ok_or(Error::Unsupported)?;
    let mut body: Vec<u8> = Vec::new();
    let user: *mut c_void = (&mut body as *mut Vec<u8>).cast();
    // SAFETY: the host calls `capture_blob` only during this call, with `user` naming `body`, which outlives it.
    let status = unsafe { read(fns.ctx, Some(capture_blob), user) };
    if status == BB_ERR_NOT_FOUND {
        return Ok(None);
    }
    status_result(status)?;
    Ok(Some(body))
}

/// The sink of [`read_blob`]: appends the host's bytes to the `Vec` it names.
unsafe extern "C" fn capture_blob(user: *mut c_void, data: *const u8, len: u64) {
    // SAFETY: `user` is the `Vec` that `read_blob` passed, alive for the call.
    let Some(body) = (unsafe { user.cast::<Vec<u8>>().as_mut() }) else {
        return;
    };
    if len == 0 || len > MAX_BLOB_BYTES || data.is_null() {
        return;
    }
    // SAFETY: the host passes `len` bytes valid for this call.
    body.extend_from_slice(unsafe { core::slice::from_raw_parts(data, len as usize) });
}

/// Replace the mod's blob. A body over the blob limit is refused before the call.
pub(crate) fn write_blob(fns: &SavesFns, body: &[u8]) -> ModResult<()> {
    let write = fns.write.ok_or(Error::Unsupported)?;
    if body.len() as u64 > MAX_BLOB_BYTES {
        return Err(Error::Arg);
    }
    // SAFETY: `body` is valid for the call, and its length is the one passed.
    status_result(unsafe { write(fns.ctx, body.as_ptr(), body.len() as u64) })
}

/// Delete the mod's blob.
pub(crate) fn remove_blob(fns: &SavesFns) -> ModResult<()> {
    let remove = fns.remove.ok_or(Error::Unsupported)?;
    // SAFETY: the table is the host's and stays valid while the mod is loaded.
    status_result(unsafe { remove(fns.ctx) })
}
