//! LAN play in the front end (ADR 0011, `docs/specs/multiplayer/frontend.md`): the "LAN Game" menu path (host, join, the
//! lobby), the session HUD pieces, and the seam they talk to the network through.
//!
//! - [`backend`]: the [`NetFrontend`] trait and its plain types. The game's `NetSession` implements it; nothing here knows
//!   about sockets or the world.
//! - [`handle`]: the [`LanBackend`] resource that holds the game's `NetFrontend`.
//! - `mock`: [`mock::MockNetFrontend`] for the tests.

pub mod backend;
pub mod car;
pub mod handle;
pub mod host_options;
#[cfg(test)]
pub mod mock;
pub mod text;

pub use backend::{Disconnect, HostOptions, LeadInfo, NetFrontend, NetStatus, ServerEntry};
pub use handle::LanBackend;
