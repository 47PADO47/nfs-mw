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
//! The session rules, pure state machines that take the time from the caller and return what to send:
//!
//! - [`settings`]: session settings defaults, validation, a host's patch, and the flat key and value form.
//! - [`lobby`]: the token buckets of the per-player rate limits.
//! - [`join`]: the join-in-progress policy and the admission pipeline with its reject reasons.
//! - [`roles`]: host, admin and player permissions, bans, and the local mute list.
//! - [`vote`]: kick, restart, mode, track and skip votes counted by the server.
//! - [`chat`]: chat rate and text rules, quick messages and system notices.
//! - [`drop`]: disconnects, resume tokens, and the late-join bulk transfer.
//! - [`spectate`]: who may watch, the spectator rate and the follow camera.

pub mod chat;
pub mod drop;
pub mod input;
pub mod join;
pub mod loadout;
pub mod lobby;
pub mod message;
pub mod roles;
pub mod schema;
pub mod settings;
pub mod spectate;
pub mod version;
pub mod vote;
