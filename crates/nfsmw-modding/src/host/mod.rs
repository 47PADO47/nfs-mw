//! The game's side of the `nfsmw.*` interfaces (ADR 0005 section 8): the backend traits the game implements, and the
//! builders that turn a backend into the `extern "C"` tables a mod calls.
//!
//! - [`pursuit`], [`career`], [`races`]: one backend trait each, and `*_table` builders that return an owner of the
//!   table. The owner keeps the table and the backend alive; drop it and the table is gone.
//! - [`events`]: the `BbEvent` the game dispatches for an `nfsmw.*` event.
//! - `io` and `slot` are the shared pointer work and the panic guard. Every trampoline runs inside `guard`, so a panic in
//!   a backend becomes `BB_ERR_PANIC`, never an unwind into the mod.
//!
//! The backends answer with plain Rust values, or with `Err(status)`. The trampolines check the mod's arguments before
//! they call the backend, so a backend sees only valid numbers (finite floats, known enum numbers, names that are UTF-8).

#![allow(unsafe_code)] // ADR 0005 section 8: `unsafe` is confined to the table, io and slot modules of `host` and `sdk::ffi`.

pub mod career;
pub mod events;
pub mod pursuit;
pub mod races;

mod io;
mod slot;

use blackbox_modding::abi::BbStatus;

/// What a backend method returns: its value, or the status the mod sees.
pub type BackendResult<T> = Result<T, BbStatus>;

pub use career::{CareerBackend, career_table};
pub use events::mw_event;
pub use pursuit::{HeatRowSpec, PursuitBackend, PursuitTable, pursuit_table};
pub use races::{RacesBackend, races_table};
