//! The C ABI shared by the game and every mod library (ADR 0005 section 1).
//!
//! Only `#[repr(C)]` plain data, fixed-width integers, raw pointers and `extern "C"` function pointers cross
//! the boundary. No `usize` in a struct (lengths are `u64`), no `bool` (use `u8`), no Rust enum with a payload.
//!
//! Append-only rules:
//!
//! - Every table starts with `size: u32`, the number of bytes its builder filled in, then its version. A reader
//!   checks `size` against the end of a field before it reads that field (see [`has_field`]). A field the table
//!   does not cover means "not supported".
//! - Within one major version a table only grows. A field is never moved, retyped or removed. A removed call
//!   keeps its slot and answers `BB_ERR_REMOVED`.
//! - Status codes and event kinds are never reused. A new code takes the next unused value.
//! - A breaking change bumps the major version ([`ABI_MAJOR`]) and gets a new interface id suffix.
//! - Every layout change edits the snapshot numbers in `tests.rs` in the same commit.
//!
//! Function pointers in tables are `Option<fn>`. A null pointer is then a valid value, and the layout is the
//! same as the bare pointer (both are one pointer wide). The C side can fill a null for an absent function.

#![allow(unsafe_code)] // ADR 0005 section 8: `unsafe` is confined to `abi`, `host::loader` and `sdk::ffi`.

mod host;
mod iface;
mod module;
mod size;
mod status;
mod string;
mod version;

#[cfg(test)]
mod tests;

pub use host::{
    BB_LOG_DEBUG, BB_LOG_ERROR, BB_LOG_INFO, BB_LOG_TRACE, BB_LOG_WARN, BbHost, BbLogFn, BbQueryInterfaceFn,
};
pub use iface::BbIface;
pub use module::{
    BB_EVENT_CANCEL, BB_EVENT_CONTINUE, BbDestroyFn, BbEvent, BbEventResult, BbModApi, BbModCreateFn, BbModInfo,
    BbModInfoFn, BbOnEventFn, MOD_CREATE_SYMBOL, MOD_INFO_SYMBOL,
};
pub use size::has_field;
pub use status::{
    BB_ERR_ABI, BB_ERR_ARG, BB_ERR_DENIED, BB_ERR_EXISTS, BB_ERR_HOST, BB_ERR_NOT_FOUND, BB_ERR_NOT_IN_CALLBACK,
    BB_ERR_NOT_READY, BB_ERR_PANIC, BB_ERR_PHASE, BB_ERR_REMOVED, BB_ERR_STALE, BB_ERR_UNSUPPORTED,
    BB_ERR_WRONG_THREAD, BB_OK, BbStatus,
};
pub use string::BbStr;
pub use version::{ABI_MAJOR, ABI_MINOR, ABI_VERSION, AbiMismatch, AbiVersion, check_handshake};
