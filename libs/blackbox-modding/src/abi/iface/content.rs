//! `bb.content` 1.1: tell the host that the files and manifests a mod keeps in its own folder have changed since start-up,
//! and let the mod make some of its files only when the game reads them (catalogue: docs/modding/api-content.md).
//!
//! Version history: 1.0 had `rescan`; 1.1 adds `provide_lazy_files`.

use core::ffi::c_void;

use super::BbIface;
use crate::abi::{BbStatus, BbStr};

/// `rescan`: read the calling mod's folder again and add what it holds that the host has not yet seen: its `files/`
/// layer and the manifests of its content (`cars/`). Content found this way is available to everything the game opens
/// after the call; what is already open keeps what it had. A second call adds only what is new. Main thread only.
pub type BbContentRescanFn = unsafe extern "C" fn(ctx: *mut c_void) -> BbStatus;

/// Receives the bytes of one lazy file. The mod calls it from inside [`BbContentReadLazyFn`] with the whole file; the bytes
/// are valid only for the call and the host copies them. A zero `len` is an empty file (`bytes` may then be null).
pub type BbContentBytesSinkFn = unsafe extern "C" fn(sink: *mut c_void, bytes: *const u8, len: u64);

/// Make one lazy file: the mod's function that the host calls with the `key` of a stub in the mod's `files/` layer
/// (`GameDir::read` in the host; the stub format is in the catalogue). The mod calls `sink(sink_ctx, bytes, len)` once with
/// the whole contents of the file and returns `BB_OK`, or returns an error status without calling it. `user` is the pointer
/// given to `provide_lazy_files`, passed back unchanged.
///
/// It is called on **any thread** (the thread that opens the file, several at once), not only the main thread, and never
/// inside a mod callback: the mod must not call the host's functions from it. The `key` is valid for the call only.
pub type BbContentReadLazyFn = unsafe extern "C" fn(
    user: *mut c_void,
    key: BbStr,
    sink: Option<BbContentBytesSinkFn>,
    sink_ctx: *mut c_void,
) -> BbStatus;

/// `provide_lazy_files`: tell the host which function makes the calling mod's lazy files. The host keeps `read` and `user`
/// until the mod unloads, after which it never calls them again and a lazy file of the mod is an error when read. A second call
/// replaces the first. `read` may be called as soon as this returns. Main thread only.
pub type BbContentProvideLazyFn =
    unsafe extern "C" fn(ctx: *mut c_void, read: Option<BbContentReadLazyFn>, user: *mut c_void) -> BbStatus;

/// The `bb.content` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbContent {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Read the mod's folder again.
    pub rescan: Option<BbContentRescanFn>,
    /// Say which function makes the mod's lazy files (since 1.1).
    pub provide_lazy_files: Option<BbContentProvideLazyFn>,
}
