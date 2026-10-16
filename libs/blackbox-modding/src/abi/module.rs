//! The tables a mod library exports and the event header (ADR 0005 sections 1 and 4).

use core::ffi::c_void;

use super::{BbHost, BbStatus, BbStr};

/// Name of the symbol a mod exports to return its [`BbModInfo`].
pub const MOD_INFO_SYMBOL: &str = "bb_mod_info";
/// Name of the symbol a mod exports to fill its [`BbModApi`] from the host table.
pub const MOD_CREATE_SYMBOL: &str = "bb_mod_create";

/// What `bb_mod_info` returns. The loader reads it first, before it builds any host table, so a mod for another
/// major version is refused without running any of its other code.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BbModInfo {
    /// Bytes the mod filled in.
    pub size: u32,
    /// The ABI major version the mod was built against.
    pub abi_major: u16,
    /// The ABI minor version the mod was built against.
    pub abi_minor: u16,
    /// The SDK version string, for diagnostics.
    pub sdk_version: BbStr,
}

impl BbModInfo {
    /// The info the SDK exports for a mod built against this ABI.
    pub const fn for_sdk(sdk_version: &str) -> Self {
        BbModInfo {
            size: core::mem::size_of::<BbModInfo>() as u32,
            abi_major: super::ABI_MAJOR,
            abi_minor: super::ABI_MINOR,
            sdk_version: BbStr::new(sdk_version),
        }
    }
}

/// The mod table `bb_mod_create` fills in. `state` is the mod's own pointer; the host passes it back to
/// `on_event` and `destroy` and never reads through it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbModApi {
    /// Bytes the mod filled in.
    pub size: u32,
    /// The ABI major version the mod was built against.
    pub abi_major: u16,
    /// The ABI minor version the mod was built against.
    pub abi_minor: u16,
    /// The mod's state, opaque to the host.
    pub state: *mut c_void,
    /// Delivers one event to the mod. Returns a [`BbEventResult`].
    pub on_event: Option<BbOnEventFn>,
    /// Frees the mod's state and runs its unload code. Called once, before the library is closed.
    pub destroy: Option<BbDestroyFn>,
}

/// One event, as the host delivers it. `payload` points to a per-kind struct that starts with its own `size`.
/// Kinds are `u32` constants that are never reused; a mod ignores kinds it does not know.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BbEvent {
    /// Bytes the host filled in.
    pub size: u32,
    /// Event kind.
    pub kind: u32,
    /// Frame number at delivery.
    pub frame: u64,
    /// Per-kind payload, or null when the kind has none.
    pub payload: *const c_void,
}

/// What `on_event` returns: [`BB_EVENT_CONTINUE`], [`BB_EVENT_CANCEL`] (only for cancelable events), or a negative
/// `BB_ERR_*` code. A negative code counts as a fault of the mod.
pub type BbEventResult = BbStatus;

/// The event goes on to the game as usual.
pub const BB_EVENT_CONTINUE: BbEventResult = 0;
/// Cancel the action behind a cancelable event.
pub const BB_EVENT_CANCEL: BbEventResult = 1;

/// Signature of `on_event`.
pub type BbOnEventFn = unsafe extern "C" fn(state: *mut c_void, event: *const BbEvent) -> BbEventResult;
/// Signature of `destroy`.
pub type BbDestroyFn = unsafe extern "C" fn(state: *mut c_void);
/// Signature of the exported `bb_mod_info` symbol.
pub type BbModInfoFn = extern "C" fn() -> *const BbModInfo;
/// Signature of the exported `bb_mod_create` symbol.
pub type BbModCreateFn = unsafe extern "C" fn(host: *const BbHost, out: *mut BbModApi) -> BbStatus;
