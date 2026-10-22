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
//! The gameplay rules the server runs on top of the world:
//!
//! - [`modes`]: one state machine per game mode (free roam, circuit, sprint, speedtrap, knockout, pursuit co-op, tag),
//!   fed by [`nfsmw_sim::SimEvent`]s and car samples, returning the events clients hear; the custom rules (MP-11) and
//!   the modes this build refuses (drag, human cops).
//! - [`fairness`]: catch-up, collision modes, nitrous and damage toggles, the reset rule and the wrong-way notice.
//! - [`results`]: the per-race result record (fixed and versioned), session stats and their result message.
//! - [`record`]: the replay file of the server and of a client, written to and read from caller-owned bytes.
//!
//! The session rules, pure state machines that take the time from the caller and return what to send:
//!
//! - [`settings`]: session settings defaults, validation, a host's patch, and the flat key and value form.
//! - [`lobby`]: the lobby and session state machine: roster, cars, ready, start countdown, hold, load and race phases.
//! - [`join`]: the join-in-progress policy and the admission pipeline with its reject reasons.
//! - [`roles`]: host, admin and player permissions, bans, and the local mute list.
//! - [`vote`]: kick, restart, mode, track and skip votes counted by the server.
//! - [`chat`]: chat rate and text rules, quick messages and system notices.
//! - [`drop`]: disconnects, resume tokens, and the late-join bulk transfer.
//! - [`spectate`]: who may watch, the spectator rate and the follow camera.
//!
//! The client's netcode (`docs/specs/multiplayer/sync-model.md`), driven by the caller's times:
//!
//! - [`predict`]: the owner's car run ahead of the server, with the ring of unacknowledged ticks.
//! - [`reconcile`]: comparison with the server's own-car block, rewind and replay, the smoothed correction offset,
//!   correction statistics and the [`reconcile::Tuning`] values.
//! - [`interp`]: snapshot buffers of remote cars, traffic and police, the adaptive display delay, interpolation and
//!   bounded extrapolation.
//! - [`clock`]: the estimated server tick from PING and PONG, the ping schedule and the input lead.
//! - [`netstats`]: the counters of the network overlay line.
//! - [`netsim`]: the `net.sim` bad-network setting for the link simulator.
//! - [`frames`]: the `SNAPSHOT`, `PING` and `PONG` payloads and the snapshot's global block, written by the server and
//!   read by the client.
//! - [`client_session`]: the client's session driver over a `ClientEndpoint` (or a mock link): admission, the lobby
//!   mirror, snapshots into prediction and interpolation, events, reconnect and spectating.

pub mod chat;
pub mod client_session;
pub mod clock;
pub mod drop;
pub mod fairness;
pub mod frames;
mod geom;
pub mod input;
pub mod interp;
pub mod join;
pub mod loadout;
pub mod lobby;
pub mod message;
pub mod modes;
pub mod netsim;
pub mod netstats;
pub mod predict;
pub mod reconcile;
pub mod record;
pub mod results;
pub mod roles;
pub mod schema;
pub mod settings;
pub mod spectate;
pub mod version;
pub mod vote;
