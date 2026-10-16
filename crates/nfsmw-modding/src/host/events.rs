//! The `BbEvent` the game dispatches for an `nfsmw.*` event. The game fills a payload struct from [`abi::events`](crate::abi)
//! and hands the event to `ModRuntime::dispatch_event` from blackbox-modding.

use core::ffi::c_void;
use core::mem::size_of;

use blackbox_modding::abi::BbEvent;

/// Build the event of `kind` for `payload`. The payload must stay alive until the dispatch call returns: the event holds
/// only a pointer to it, as ADR 0005 section 4 requires.
pub fn mw_event<T>(kind: u32, frame: u64, payload: &T) -> BbEvent {
    BbEvent { size: size_of::<BbEvent>() as u32, kind, frame, payload: core::ptr::from_ref(payload).cast::<c_void>() }
}
