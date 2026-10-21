//! The session and per-peer state machines (ADR 0009 section 6). Generic: no game modes, no messages. The server drives
//! one [`Session`]; the game maps phases to screens and messages.
//!
//! The host calls `start_loading`, `host_loaded`, `finish` and `back_to_lobby`, reports each peer with `peer_loaded`,
//! and calls `update` every tick. A call that does not fit the current phase returns [`SessionError::WrongPhase`] and
//! changes nothing. Phase changes and peer removals are queued as [`SessionEvent`]s and read with `poll_event`.

mod config;
mod error;
mod event;
mod machine;
mod peer;
mod phase;
#[cfg(test)]
mod tests;

pub use config::SessionConfig;
pub use error::SessionError;
pub use event::SessionEvent;
pub use machine::Session;
pub use peer::{PeerId, PeerState};
pub use phase::Phase;
