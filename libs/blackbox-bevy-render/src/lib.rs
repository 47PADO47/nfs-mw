//! An optional Bevy 0.20 renderer behind `blackbox-gfx`.
//!
//! [`BlackboxBevyRenderPlugin`] joins a Bevy `App`; [`BackendFactory`] then gives the game a [`BevyBackend`], a
//! `blackbox_gfx::RenderBackend`. [`probe`] says before any of that whether the machine can run it.
//! [`HeadlessBevy`] is the same renderer in a window-less `App`, for tests and tools.
//!
//! See the README and `docs/bevy-backend.md` for the design.

pub mod apply;
pub mod axes;
mod caps;
pub mod facade;
pub mod harness;
pub mod material;
pub mod mesh;
pub mod ops;
#[cfg(test)]
mod parity_tests;
pub mod plugin;
pub mod post;
#[cfg(test)]
mod post_tests;
pub mod probe;
#[cfg(test)]
mod stress_tests;
pub mod systems;
pub mod texture;

pub use facade::BevyBackend;
pub use harness::HeadlessBevy;
pub use ops::BlackboxBridge;
pub use plugin::{BackendFactory, BlackboxBevyRenderPlugin, backend};
pub use probe::{Probe, ProbeError, Vendor, probe, probe_with};

/// Install a Tracy `tracing` subscriber so Bevy's per-system spans (enabled by the `trace` feature) are
/// recorded. Call once at startup, before the app runs, and attach the Tracy client to read them.
///
/// This sets only the `tracing` dispatcher, not a `log` bridge: `env_logger` already owns the global `log`
/// logger, so `SubscriberInitExt::init`/`try_init` would fail trying to install a second one.
#[cfg(feature = "trace")]
pub fn init_tracing() {
    use tracing_subscriber::layer::SubscriberExt;
    let subscriber = tracing_subscriber::registry().with(tracing_tracy::TracyLayer::default());
    if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
        log::warn!("Tracy tracing subscriber not installed: {err}");
    }
}
