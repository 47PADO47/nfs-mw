//! Native mod plugins for Black Box games (ADR 0005, `docs/decisions/0005-native-mod-plugins.md`).
//!
//! A mod is a dynamic library. It talks to the game only through versioned, size-prefixed `#[repr(C)]`
//! tables and `extern "C"` functions, so no Rust type or allocator crosses the boundary. Three parts, one
//! feature each:
//!
//! - `abi` (default): the tables, status codes, ids and entry-symbol names that the mod and the host agree on.
//!   Plain data only; see [`abi`].
//! - `sdk`: what a mod links. The `Mod` trait, the `declare_mod!` macro that exports the entry symbols, a
//!   panic guard around every callback, and safe wrappers over the host table. Needs only `abi`; see [`sdk`].
//! - `host`: the loader and dispatcher (manifest, discovery, handshake, quarantine). Reserved; not written yet.
//!
//! This crate is engine-generic. It names no game. A game offers its own interfaces through the query
//! mechanism (ADR 0005 section 8). `unsafe` is confined to `abi`, `sdk::ffi` and, later, the host loader.

#[cfg(feature = "abi")]
pub mod abi;

#[cfg(feature = "sdk")]
pub mod sdk;
