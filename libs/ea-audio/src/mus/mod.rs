//! The interactive-music map (`.mpf`, magic `xDFP`) and its stream file (`.mus`).
//!
//! The map holds a PathFinder node graph (which segment follows which, driven by game state) and a table of the
//! streams in the `.mus` file. This module reads only the tracks and the stream table: enough to list and decode
//! every stream. The graph is not parsed.

mod mpf;

pub use mpf::{Mpf, MusStream, Track};
