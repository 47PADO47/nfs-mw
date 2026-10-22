//! LAN multiplayer in the game (ADR 0011, `docs/specs/multiplayer/game-wiring.md`), behind the cargo feature
//! `multiplayer`.
//!
//! - `session.rs`: [`NetSession`], the game's multiplayer state and its modes (idle, host, join, dedicated).
//! - `frontend.rs`: `NetFrontend` for `NetSession`, what the LAN screens and the `net.*` commands see.
//! - `plugin.rs`: [`NetPlugin`], the session in the front end's `LanBackend` and its systems; [`NetStart`], what the
//!   command line asked for.
//! - `client.rs`: [`NetClient`], the client session over the player's real car, shared with the world's client mode
//!   (`scenes/world/net_client/`).
//! - `listen.rs`: the listen server on its own thread; `dedicated.rs`: `--dedicated`; `discover.rs`: LAN scans and
//!   `--discover`.
//! - `build.rs`: the build and data ids, the car catalogue loadouts are checked against, the server setup.
//! - `identity.rs`: the player's identity key file (`net_identity_file`).
//! - `options.rs`: [`ServeOptions`], how a server starts.

mod build;
pub mod client;
mod dedicated;
pub mod discover;
mod frontend;
mod identity;
mod listen;
mod options;
mod plugin;
mod session;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub use build::{data_id, server_setup, sim_build};
pub use client::{ClientHandle, PlayerSetup};
pub use options::ServeOptions;
pub use plugin::{NetPlugin, NetStart};
pub use session::NetSession;
