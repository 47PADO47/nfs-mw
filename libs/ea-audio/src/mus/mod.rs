//! The interactive-music map (`.mpf`, magic `xDFP`) and its stream file (`.mus`).
//!
//! The map holds a PathFinder node graph (which segment follows which, driven by game state) and a table of the
//! streams in the `.mus` file. [`Mpf`] reads the tracks and the stream table: enough to list and decode every
//! stream. [`graph::Graph`] reads the node graph, the events and the routers, and [`Mpf::chain`] joins a walk of
//! the graph with the stream table: the streams a song plays one after the other.

mod chain;
pub mod graph;
mod mpf;
mod play;

pub use chain::{Chain, Segment};
pub use mpf::{Mpf, MusStream, Track};
pub use play::ChainReader;
