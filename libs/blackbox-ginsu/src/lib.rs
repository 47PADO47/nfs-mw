//! Ginsu: the granular engine-sound synthesiser of EA Black Box games, and the parser of the `Gnsu` tables
//! it runs on. Spec: `docs/specs/engine-sound-ginsu.md`; file layout: `docs/formats/audio.md`.
//!
//! A `Gnsu` file is a recording of an engine sweeping through its RPM range, cut into pitch cycles, with
//! tables that say where in the recording each frequency and each cycle is. [`GinsuSynth`] plays the
//! recording forward and, to follow a target frequency, jumps whole cycles with a half-millisecond
//! cross-fade, so the pitch tracks the target and the timbre is the recording's at that RPM.
//!
//! This crate reads no files and decodes no audio. [`GinsuTables::parse`] reads the header and the two
//! tables from the bytes of a `.gin` file and returns where the EA-XAS payload starts; the caller decodes it
//! (`ea-audio`) and hands the mono samples to [`GinsuData`].
//!
//! ```
//! use std::sync::Arc;
//! use blackbox_ginsu::{GinsuData, GinsuSynth, GinsuTables, SynthParams};
//!
//! // Tables and samples would come from a `.gin` file; here a one-cycle-per-100-samples toy.
//! let cycle_pos: Vec<u32> = (0..=20).map(|k| k * 100).collect();
//! let tables = GinsuTables::new(1000.0, 2000.0, 24_000, 2001, vec![0, 1000, 2000], cycle_pos)?;
//! let pcm = (0..2001).map(|n| (n as f32 * std::f32::consts::TAU / 100.0).sin() * 0.5).collect();
//! let data = Arc::new(GinsuData::new(tables, pcm)?);
//!
//! let mut synth = GinsuSynth::new(data, 1500.0)?;
//! let mut block = [0.0f32; 256];
//! synth.render(&SynthParams::new(1500.0).with_volume(0.8), &mut block);
//! # Ok::<(), blackbox_ginsu::Error>(())
//! ```

mod data;
mod error;
mod round;
mod synth;
mod tables;

#[cfg(test)]
mod tests;

pub use data::GinsuData;
pub use error::{Error, Result};
pub use synth::{DEFAULT_LATENCY_MS, GinsuSynth, SynthParams};
pub use tables::{FREQUENCY_PER_HZ, GinsuTables, HEADER_LEN, MAGIC};
