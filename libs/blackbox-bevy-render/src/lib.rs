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
#[cfg(feature = "trace")]
pub fn init_tracing() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    tracing_subscriber::registry().with(tracing_tracy::TracyLayer::default()).init();
}
