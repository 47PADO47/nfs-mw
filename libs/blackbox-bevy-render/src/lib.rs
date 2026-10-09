//! An optional Bevy 0.20 renderer behind `blackbox-gfx`.
//!
//! See the README and `docs/bevy-backend.md` for the design.

pub mod probe;

pub use probe::{Probe, ProbeError, Vendor, probe, probe_with};
