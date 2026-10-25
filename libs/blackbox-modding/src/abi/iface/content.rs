//! `bb.content` 1.0: tell the host that the files and manifests a mod keeps in its own folder have changed since start-up
//! (catalogue: docs/modding/api-content.md).

use core::ffi::c_void;

use super::BbIface;
use crate::abi::BbStatus;

/// `rescan`: read the calling mod's folder again and add what it holds that the host has not yet seen: its `files/`
/// layer and the manifests of its content (`cars/`). Content found this way is available to everything the game opens
/// after the call; what is already open keeps what it had. A second call adds only what is new. Main thread only.
pub type BbContentRescanFn = unsafe extern "C" fn(ctx: *mut c_void) -> BbStatus;

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
}
