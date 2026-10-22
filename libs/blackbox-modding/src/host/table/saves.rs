//! `bb.saves` on the host side: the backend the game implements, and the trampolines behind `BbSaves`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, guard, slot_of};
use crate::abi::{
    BB_ERR_ARG, BbIface, BbSaves, BbSavesSinkFn, BbStatus, IFACE_SAVES, IFACE_SAVES_VERSION, MAX_BLOB_BYTES,
};

/// What the game implements for `bb.saves`. `owner` is the id of the mod that made the call; the blob is the owner's.
pub trait SavesBackend {
    /// The owner's blob. `Err(BB_ERR_NOT_FOUND)` when it has none, `Err(BB_ERR_CORRUPT)` when the stored blob fails its
    /// check.
    fn read(&self, owner: &str) -> BackendResult<Vec<u8>>;
    /// Replace the owner's blob with `body`, which is at most `MAX_BLOB_BYTES` long.
    fn write(&self, owner: &str, body: &[u8]) -> BackendResult<()>;
    /// Delete the owner's blob. A blob that does not exist is not an error.
    fn remove(&self, owner: &str) -> BackendResult<()>;
}

/// Build the `bb.saves` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn SavesBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbSaves {
        head: BbIface { size: size_of::<BbSaves>() as u32, major: IFACE_SAVES_VERSION.0, minor: IFACE_SAVES_VERSION.1 },
        ctx: Slot::ctx(&slot),
        read: Some(read),
        write: Some(write),
        remove: Some(remove),
    });
    Attached::new(IFACE_SAVES, IFACE_SAVES_VERSION, table, Box::new(|| {}))
}

/// `read`: the owner's body, given to the caller's sink once.
unsafe extern "C" fn read(ctx: *mut c_void, sink: Option<BbSavesSinkFn>, sink_user: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn SavesBackend>(ctx)? };
        let sink = sink.ok_or(BB_ERR_ARG)?;
        let body = slot.backend.read(&slot.owner)?;
        // SAFETY: the sink is the caller's function, and `body` is valid for the duration of the call.
        unsafe { sink(sink_user, body.as_ptr(), body.len() as u64) };
        Ok(())
    })
}

/// `write`: replace the owner's body with the bytes the caller passes.
unsafe extern "C" fn write(ctx: *mut c_void, data: *const u8, len: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn SavesBackend>(ctx)? };
        let body = unsafe { bytes_within(data, len) }?;
        slot.backend.write(&slot.owner, body)?;
        Ok(())
    })
}

/// `remove`: delete the owner's blob.
unsafe extern "C" fn remove(ctx: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn SavesBackend>(ctx)? };
        slot.backend.remove(&slot.owner)?;
        Ok(())
    })
}

/// The caller's bytes, when `len` is within the blob limit and the pointer is valid for it.
///
/// # Safety
/// `data` must point at `len` readable bytes for the duration of the call, unless `len` is zero.
unsafe fn bytes_within<'a>(data: *const u8, len: u64) -> BackendResult<&'a [u8]> {
    if len > MAX_BLOB_BYTES {
        return Err(BB_ERR_ARG);
    }
    if len == 0 {
        return Ok(&[]);
    }
    if data.is_null() {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the caller's contract above; `len` is at most `MAX_BLOB_BYTES`, so it fits in `usize`.
    Ok(unsafe { core::slice::from_raw_parts(data, len as usize) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_are_limited_and_need_a_pointer() {
        let bytes = [1u8, 2, 3];
        // SAFETY: `bytes` holds the three bytes the length names.
        let body = unsafe { bytes_within(bytes.as_ptr(), 3) }.unwrap();
        assert_eq!(body, &bytes);
        // SAFETY: a zero length never reads.
        assert!(unsafe { bytes_within(core::ptr::null(), 0) }.unwrap().is_empty());
        // SAFETY: the null pointer is refused before any read.
        assert_eq!(unsafe { bytes_within(core::ptr::null(), 1) }.unwrap_err(), BB_ERR_ARG);
        // SAFETY: an over-long length is refused before any read.
        assert_eq!(unsafe { bytes_within(bytes.as_ptr(), MAX_BLOB_BYTES + 1) }.unwrap_err(), BB_ERR_ARG);
    }
}
