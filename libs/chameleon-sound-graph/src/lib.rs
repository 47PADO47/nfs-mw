//! Sound graphs of the Chameleon engine. Spec: `docs/formats/chameleon-sound-graph.md`.
//!
//! The sound half of a car bundle holds `Sequence` objects: each is driven by a live value of the game (the audible
//! engine speed, the engine load, the transmission speed, ...) and holds items that are active in windows of it: wave
//! loops, Ginsu engine voices and mixer channel gains, each with value modulators (curves) for gain and pitch. This
//! crate reads them from a [`chameleon_bundle::Bundle`] into plain data and evaluates windows, fades, modulators and
//! curves; it plays nothing. It also splits the `Ginsu` engine resources of a bundle ([`GinsuResource`]).

mod bytes;
mod curve;
mod ginsu;
mod graphs;
mod sequence;
mod value;

pub use curve::{Curve, Point};
pub use ginsu::GinsuResource;
pub use graphs::{SoundGraphs, ginsu_resource};
pub use sequence::{GinsuItem, Item, ItemKind, MixerItem, Sequence, WaveItem};
pub use value::{Fade, Modulator};

#[cfg(test)]
mod real_tests;
