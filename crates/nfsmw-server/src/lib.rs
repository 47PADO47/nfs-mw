//! The multiplayer server loop (ADR 0011): an authoritative 60 Hz loop over any [`nfsmw_sim::WorldSim`], fed by the net
//! thread of `blackbox-net-core` and the session rules of `nfsmw-net`. The `nfsmw-server` binary runs it over the toy
//! world for protocol tests, soak runs and CI; the game serves its real world with the same [`Server`] (`--dedicated`
//! or a listen server).
//!
//! - [`Server`] and [`ServerSetup`]: the loop ([`Server::tick_once`] for a caller's clock, [`Server::run`] for the
//!   fixed-step timer), admission, lobby, chat, votes, modes, snapshots, drops, resumes, strikes and the replay.
//! - [`WorldControl`]: what the server asks of a world beyond `WorldSim` (resets, ghost cars), with defaults.
//! - [`ServerConfig`]: the TOML config file and its checks (MP-25, MP-84).
//! - [`bot`]: scripted bot clients over the client session.
//! - [`logging`]: the `log` backend (stderr and a file). [`timer`]: the drift-free tick timer.
//! - [`catalog`]: the game data loadouts are checked against; [`catalog::OpenCatalog`] for the toy world.
//!
//! The design and the config keys are in `docs/specs/multiplayer/server.md`.

pub mod bot;
pub mod catalog;
pub mod config;
mod control;
pub mod logging;
mod server;
pub mod timer;

pub use config::{ConfigError, ServerConfig};
pub use control::WorldControl;
pub use server::{
    KICK_SCORE, RESYNC_SCORE, Server, ServerSetup, ServerStats, TickHook, Verdict, Violation, WARN_SCORE, bind,
    endpoint_config, toy_build,
};
