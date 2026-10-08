//! Dynamic-mixer maps of EA Black Box games (`MIXMAPS/*.mxb`): a parser and a deterministic evaluator.
//! Specs: `docs/specs/dynamic-mixer.md`; file layout: `docs/formats/mixmap.md`.
//!
//! A map is a graph the sound system evaluates once per frame: *controls* turn published values (the car's
//! speed, a nitrous flag) into levels through curves, *events* are envelopes a trigger starts, *3D controls* turn a
//! distance and an azimuth into a rolloff, *channels* add levels, and each *master channel* writes the sum to
//! the output slots (volume, pitch, filter, azimuth) of one sound object. [`MixMap::parse`] reads the bytes of a
//! file; a [`Mixer`] instantiates it for the sound objects a game has, takes the values the game publishes
//! ([`Mixer::set_input`]) and gives back what each object should do ([`Mixer::volume`], [`Mixer::pitch`], ...).
//!
//! Pure and deterministic: no files, no audio output, no game knowledge. Which object is which, what the
//! inputs mean and what a slot means to an object is the game's table.
//!
//! ```
//! use std::sync::Arc;
//! use blackbox_mixmap::{InputKey, MixMap, Mixer, ObjectRef};
//!
//! # fn main() -> Result<(), blackbox_mixmap::Error> {
//! # let bytes = blackbox_mixmap::minimal_map();
//! let map = Arc::new(MixMap::parse(&bytes)?);
//! let mut mixer = Mixer::new(map, &[1]); // one copy of state 0
//! let engine = ObjectRef { state: 0, instance: 0, object: 1 };
//! mixer.attach(engine, true);
//! mixer.set_input(InputKey::controller(0, 0, 0, 0), 20_000);
//! mixer.process(1.0 / 60.0);
//! let gain = mixer.volume(engine, 2);
//! # Ok(()) }
//! ```

mod error;
mod id;
mod map;
mod mixer;
mod shape;
mod testmap;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use id::{SourceId, SourceKind};
pub use map::{
    Control, EnvelopeKind, Event, MasterChannel, MixMap, OutputKind, PresetWord, Spatial, SpatialRecord, State,
    SubChannel,
};
pub use mixer::{InputKey, InputSource, MAX_INSTANCES, Mixer, ObjectRef};
pub use shape::{SILENCE_DB, UNITY, curve, curve_db, db_from_q15, pitch_ratio, q15_from_db};
pub use testmap::{MapBuilder, minimal_map};
