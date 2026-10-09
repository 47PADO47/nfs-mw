//! The road network of EA Black Box games (Need for Speed: Most Wanted): the lanes, roads and
//! junctions AI cars drive on.
//! Spec: `docs/formats/road-network.md`, `docs/specs/ai-road-network.md`.
//!
//! - [`RoadNetwork`]: nodes, segments, profiles and roads of the `RNgp` group in the world metadata.
//!
//! Coordinates are the game's physics space: x right, y up, z forward, metres. This crate never
//! touches the filesystem: it takes bytes.

mod bytes;
mod curve;
mod error;
mod index;
mod lane;
mod locate;
mod nav;
mod network;
mod node;
mod profile;
mod road;
mod segment;
mod trail;

#[cfg(test)]
mod tests;

pub use curve::{Bezier, MIN_CHORD};
pub use error::{Error, Result};
pub use index::SegmentIndex;
pub use lane::{NodeInd, centre_line, lane_line, lane_line_shifted, right_of, travel_profile};
pub use locate::{Located, SEARCH_RADIUS, SegmentFilter, closest_segment};
pub use nav::{LaneType, NavKind, RandomSource, RoadNav, SplitMix, forward_traffic_lanes, nth_from_centre, pick_lane};
pub use network::RoadNetwork;
pub use node::RoadNode;
pub use profile::{RoadProfile, Zone, zone};
pub use road::Road;
pub use segment::{RoadSegment, flags};
pub use trail::Body;
pub use trail::{
    Avoidable, CAPACITY as TRAIL_CAPACITY, CUT, CUT_BEHIND, Cookie, DEFAULT_GAP, Occlusion, Trail, trail_curvature,
    update_occluded_position,
};
