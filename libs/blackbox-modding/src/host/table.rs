//! The host table a mod receives in `bb_mod_create` (ADR 0005 sections 1 and 3).
//!
//! The loader builds one table per mod. The table and the state it points to stay in boxes owned by the table, so
//! their addresses do not move while the mod is loaded. The host functions in the table are the only code the mod
//! calls back into; each one is wrapped in `catch_unwind`, so a panic in them never unwinds into the mod.

#![allow(unsafe_code)] // ADR 0005 section 8: `unsafe` is confined to `abi`, `host::table`, `host::loader` and `sdk::ffi`.

use core::ffi::c_void;
use core::ptr;
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::abi::{
    ABI_MAJOR, ABI_MINOR, BB_LOG_DEBUG, BB_LOG_ERROR, BB_LOG_INFO, BB_LOG_TRACE, BB_LOG_WARN, BbHost, BbIface, BbStr,
};

use super::offer::HostOffer;

/// The state a host function reads through the table's `ctx`: which mod is calling, and the game's identity.
struct CallState {
    mod_id: String,
}

/// The host table of one mod. Owns the table, the state its `ctx` points to, and the strings its game fields point
/// to. Dropping it invalidates the mod's pointer to it, so the loader drops it only after the mod is destroyed.
pub(crate) struct HostTable {
    _state: Box<CallState>,
    _strings: Box<(String, String)>,
    table: Box<BbHost>,
}

impl HostTable {
    /// Build the table for mod `mod_id`, answering with the identity of `offer`.
    pub(crate) fn new(mod_id: &str, offer: &HostOffer) -> Self {
        let state = Box::new(CallState { mod_id: mod_id.to_owned() });
        let strings = Box::new((offer.game_id.clone(), offer.game_version.clone()));
        let ctx: *const CallState = &*state;
        let table = Box::new(BbHost {
            size: core::mem::size_of::<BbHost>() as u32,
            abi_major: ABI_MAJOR,
            abi_minor: ABI_MINOR,
            ctx: ctx.cast_mut().cast::<c_void>(),
            game_id: BbStr::new(&strings.0),
            game_version: BbStr::new(&strings.1),
            log: Some(host_log),
            query_interface: Some(host_query),
        });
        HostTable { _state: state, _strings: strings, table }
    }

    /// The address the mod receives in `bb_mod_create`.
    pub(crate) fn as_ptr(&self) -> *const BbHost {
        &*self.table
    }
}

/// Map a `BB_LOG_*` level to the `log` crate. An unknown level logs at info.
fn level_of(level: u32) -> log::Level {
    match level {
        BB_LOG_ERROR => log::Level::Error,
        BB_LOG_WARN => log::Level::Warn,
        BB_LOG_INFO => log::Level::Info,
        BB_LOG_DEBUG => log::Level::Debug,
        BB_LOG_TRACE => log::Level::Trace,
        _ => log::Level::Info,
    }
}

/// Read the `CallState` behind a host table's `ctx`. `None` for a null pointer.
///
/// # Safety
/// `ctx` must be null or the pointer a [`HostTable`] stored, and that table must still be alive.
unsafe fn state_of<'a>(ctx: *mut c_void) -> Option<&'a CallState> {
    if ctx.is_null() {
        return None;
    }
    // SAFETY: the caller guarantees `ctx` is a live `CallState` (see `# Safety`).
    Some(unsafe { &*ctx.cast::<CallState>() })
}

/// The `log` host function: one line to the game log, tagged with the mod's id.
unsafe extern "C" fn host_log(ctx: *mut c_void, level: u32, msg: BbStr) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `ctx` is the table's own state pointer; the table lives as long as the mod (see `# Safety` of
        // `state_of`).
        let Some(state) = (unsafe { state_of(ctx) }) else {
            return;
        };
        // SAFETY: the mod passes a string that is valid for this call (ADR 0005 section 4).
        let Some(text) = (unsafe { msg.as_str() }) else {
            return;
        };
        log::log!(target: "blackbox_modding::mod", level_of(level), "[{}] {}", state.mod_id, text);
    }));
}

/// The `query_interface` host function. No interface tables are attached yet: the game facade adds them (milestone
/// 12). Until then every interface is absent, and a mod that asks gets null, which the SDK reports as `None`.
unsafe extern "C" fn host_query(_ctx: *mut c_void, _id: BbStr, _major: u16, _min_minor: u16) -> *const BbIface {
    ptr::null()
}
