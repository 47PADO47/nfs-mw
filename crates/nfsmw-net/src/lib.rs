//! NFS: Most Wanted multiplayer protocol and rules (ADR 0011, `docs/specs/multiplayer/`). Bevy-free and I/O-free: every
//! rule runs in a unit test without a window, a GPU, a socket or a game install.
//!
//! The wire foundation, on top of `blackbox_net_core` (bits, replicate, conn) and `nfsmw_sim` (the world's plain types):
//!
//! - [`schema`]: the game's entity kinds and field layout registered with `blackbox_net_core::replicate`, and the
//!   conversion of [`nfsmw_sim::EntityView`] and [`nfsmw_sim::OwnState`] to quantised records and back.
//! - [`input`]: the per-tick input frame: quantisation of [`blackbox_vehicle::InputState`], the `same` compression,
//!   redundant frames on the client and the server's input window.
//! - [`message`]: every reliable message of the protocol and its bounded, versioned codec.
//! - [`version`]: protocol versions, the game-data and sim-build ids, the mod list and the join compatibility decision.
//! - [`loadout`]: car selection by ids and its validation against the caller's game data.
//!
//! The client's netcode (`docs/specs/multiplayer/sync-model.md`), driven by the caller's times:
//!
//! - [`predict`]: the owner's car run ahead of the server, with the ring of unacknowledged ticks.

pub mod input;
pub mod loadout;
pub mod message;
pub mod schema;
pub mod version;

mod geom;
pub mod predict;
