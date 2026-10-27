//! The engine on/off state of a Chameleon car (NFS: Most Wanted 2012). Spec: `docs/formats/chameleon-engine-state.md`.
//!
//! Two parts. [`EngineMachine`] is a behaviour model: a car that stands still for [`EngineParams::idle_seconds`] switches
//! its engine off, and the throttle starts it again after [`EngineParams::crank_seconds`] of cranking. Its numbers are
//! **[unconfirmed]**: the install gives the engine an on/off state and an ignition sound, but no time or speed for the
//! standstill rule was found (the doc lists what was searched). [`read_ignition`] reads the part that *is* in the data,
//! the car's `Player Vehicle Ignition` sound sequence: its sample, its length and its gain **[verified]**.
//!
//! The crate takes a parsed [`chameleon_bundle::Bundle`] and never opens files or plays anything.

mod ignition;
mod machine;
mod params;

#[cfg(test)]
mod install_tests;
#[cfg(test)]
mod tests;

pub use ignition::{Ignition, read_ignition};
pub use machine::{EngineMachine, Event, Input, Phase};
pub use params::EngineParams;
