//! Decoders for Electronic Arts' audio containers and codecs, as used by EA Black Box games.
//!
//! Bytes in, PCM out: the crate never opens files or plays sound.
//!
//! - [`schl`]: `SCHl` block streams (`.big` members, music streams) and the `PT` / `GSTR` header.
//! - [`abk`]: `ABKC` sound banks with their embedded `BNKl` sound list.
//! - [`mus`]: the interactive-music map (`.mpf`) that locates streams in the `.mus` file.
//! - [`big`]: scanning `.big` containers that hold many streams.
//! - [`gin`]: the audio inside granular engine loops.
//! - [`codec`]: EA-XA, EA-XAS and MicroTalk.
//! - [`ReadAt`]: random access for sources too large to load, such as the 533 MB music file.
//!
//! ```no_run
//! # fn main() -> Result<(), ea_audio::Error> {
//! let bytes: Vec<u8> = std::fs::read("sound.big").unwrap();
//! let pcm = ea_audio::decode(&bytes)?; // a stream starting at the beginning of `bytes`
//! println!("{} Hz, {} channel(s), {} frames", pcm.sample_rate, pcm.channels, pcm.frames());
//! # Ok(())
//! # }
//! ```
//!
//! Format notes and evidence: `docs/specs/audio-containers.md` and `docs/formats/audio.md`.

pub mod codec;
mod error;
mod pcm;
mod source;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use pcm::Pcm;
pub use source::{ReadAt, read_array, read_vec};
