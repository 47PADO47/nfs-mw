//! The C ABI of the `nfsmw.*` interfaces (ADR 0005 sections 1 and 8).
//!
//! The rules of `blackbox-modding::abi` hold here too: only `#[repr(C)]` plain data, fixed-width integers, raw pointers
//! and `extern "C"` function pointers cross the boundary; every table and every payload starts with its `size`; a field
//! is appended, never moved or retyped; a status and an event kind is never reused. This module holds no unsafe code.
//! The pointers are read and written in `host::io` and `sdk::ffi`, and nowhere else.
//!
//! Function pointers in a table are `Option<fn>`, so a null pointer is a valid value and the layout is one pointer wide.
//! The status codes are the `BB_ERR_*` of `blackbox-modding`.

#![allow(unsafe_code)] // Declares `unsafe extern "C" fn` types only; no unsafe block is written in this module tree.

pub mod career;
pub mod career_records;
pub mod codes;
pub mod events;
pub mod ids;
pub mod pursuit;
pub mod races;
pub mod shared;

#[cfg(test)]
mod tests;

pub use career::*;
pub use career_records::*;
pub use codes::*;
pub use events::*;
pub use ids::*;
pub use pursuit::*;
pub use races::*;
pub use shared::*;
