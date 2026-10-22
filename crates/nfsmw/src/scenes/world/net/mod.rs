//! The real world as a multiplayer server's authoritative world (ADR 0011, `docs/specs/multiplayer/world-host.md`).
//!
//! [`WorldSimHost`](host::WorldSimHost) owns a [`WorldScene`](super::WorldScene) and a renderer that draws nothing
//! ([`NullBackend`](crate::app::null_backend::NullBackend)) and implements [`nfsmw_sim::WorldSim`] over them: each
//! `step` hands every seated player's [`InputState`](nfsmw_sim::InputState) to its car, runs exactly one 60 Hz tick of the
//! scene under a session's rules, and reports what happened as [`SimEvent`](nfsmw_sim::SimEvent)s. Remote players are
//! co-op guests (ADR 0010): they join through the co-op spawn path on their own `PlayerId`, share the chase and are
//! busted on their own. The host's own player is `PlayerId(0)`, the world's lead.
//!
//! - `catalog.rs`: car type ids and the small type numbers of the entity views.
//! - `ids.rs`: the 16-bit entity ids of everything that is not a player car.
//! - `export.rs`: the entity views, in ascending id order.
//! - `events/`: the per-tick events (contacts, pursuit, race, moved), from what the world already knows.
//! - `seat.rs`: joining and leaving.
//! - `host.rs`, `sim.rs`: the host and its `WorldSim` implementation.

// Nothing outside the tests drives the host yet: the listen host and the dedicated server wire it in (ADR 0011).
#![allow(dead_code)]

pub mod catalog;
mod events;
mod export;
pub mod flags;
pub mod host;
pub mod ids;
mod seat;
mod sim;
#[cfg(test)]
mod tests;
