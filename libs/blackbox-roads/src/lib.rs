//! The road network of EA Black Box games (Need for Speed: Most Wanted): the lanes, roads and
//! junctions AI cars drive on.
//! Spec: `docs/formats/road-network.md`, `docs/specs/ai-road-network.md`.
//!
//! - [`RoadNetwork`]: nodes, segments, profiles and roads of the `RNgp` group in the world metadata.
//!
//! Coordinates are the game's physics space: x right, y up, z forward, metres. This crate never
//! touches the filesystem: it takes bytes.

mod bytes;
mod error;
mod network;
mod node;
mod profile;
mod road;
mod segment;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use network::RoadNetwork;
pub use node::RoadNode;
pub use profile::{RoadProfile, Zone, zone};
pub use road::Road;
pub use segment::{RoadSegment, flags};
