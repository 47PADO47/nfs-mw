//! `bb.saves` 1.0: the mod's own blob in the career profile it is loaded with (`docs/specs/mod-save-data.md`).
//!
//! The blob belongs to the mod that calls. The host names it by the mod's id, so the calls take no key: a mod cannot read
//! or write another mod's blob. A blob is at most [`MAX_BLOB_BYTES`] long.

use core::ffi::c_void;

use super::BbIface;
use crate::abi::BbStatus;

/// The most bytes one blob may hold (256 KiB, `docs/specs/mod-save-data.md` section 3).
pub const MAX_BLOB_BYTES: u64 = 256 * 1024;

/// The callback that receives a blob's body in `read`. The bytes are valid for the call only.
pub type BbSavesSinkFn = unsafe extern "C" fn(user: *mut c_void, data: *const u8, len: u64);

/// `read`: give the mod's body to `sink`, once. `BB_ERR_NOT_FOUND` when the mod has written none, `BB_ERR_CORRUPT` when
/// the stored blob fails its check (it is left in place).
pub type BbSavesReadFn =
    unsafe extern "C" fn(ctx: *mut c_void, sink: Option<BbSavesSinkFn>, sink_user: *mut c_void) -> BbStatus;

/// `write`: replace the mod's body with the `len` bytes at `data`. `BB_ERR_ARG` when `len` is over [`MAX_BLOB_BYTES`].
pub type BbSavesWriteFn = unsafe extern "C" fn(ctx: *mut c_void, data: *const u8, len: u64) -> BbStatus;

/// `remove`: delete the mod's blob. A blob that does not exist is not an error.
pub type BbSavesRemoveFn = unsafe extern "C" fn(ctx: *mut c_void) -> BbStatus;

/// The `bb.saves` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbSaves {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Read the mod's blob.
    pub read: Option<BbSavesReadFn>,
    /// Replace the mod's blob.
    pub write: Option<BbSavesWriteFn>,
    /// Delete the mod's blob.
    pub remove: Option<BbSavesRemoveFn>,
}
