//! LAN play in the front end (ADR 0011, `docs/specs/multiplayer/frontend.md`): the "LAN Game" menu path (host, join, the
//! lobby), the session HUD pieces, and the seam they talk to the network through.
//!
//! - [`backend`]: the [`NetFrontend`] trait and its plain types. The game's `NetSession` implements it; nothing here knows
//!   about sockets or the world. [`handle`]: the [`LanBackend`] resource that holds it.
//! - [`menu`]: the LAN menu's pages (pad-driven, with clicks), [`view`] draws them and the HUD with egui (the original has
//!   no LAN screens), [`system`] runs them in the app, [`gate`] keeps the FEng screens waiting while the menu is up, and
//!   [`link`] carries the requests between the FEng screens (the main menu's LAN icon, the car select) and the menu.
//! - [`feed`], [`hud`], [`results`]: the chat and notice feed, the HUD pieces, the race results.
//! - [`text`], [`car`], [`quick`], [`address`], [`host_options`]: the words, the car ids, the quick lines, typed addresses and
//!   a host's options, shared with the `net.*` console commands.
//! - [`offline`] and [`demo`]: the networks of a run with none wired in (refusals, or a pretend session with
//!   `NFSMW_LAN_DEMO=1`); `mock`: the tests' network.

pub mod address;
pub mod backend;
pub mod car;
pub mod demo;
pub mod feed;
pub(in crate::frontend) mod gate;
pub mod handle;
pub mod host_options;
pub mod hud;
pub mod link;
pub mod menu;
#[cfg(test)]
pub mod mock;
pub mod offline;
pub mod quick;
pub mod results;
pub mod system;
pub mod text;
pub mod view;

pub use backend::{Disconnect, LeadInfo, NetFrontend, NetStatus};
pub(in crate::frontend) use gate::gate;
pub use handle::LanBackend;
pub(in crate::frontend) use system::build;
pub use system::{LanUi, show, stats_line};

/// Serialises the tests that use the shared requests of [`link`].
#[cfg(test)]
pub fn link_tests_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}
