//! The road network of EA Black Box games (Need for Speed: Most Wanted): the lanes, roads and
//! junctions AI cars drive on.
//! Spec: `docs/formats/road-network.md`, `docs/specs/ai-road-network.md`.
//!
//! - [`RoadNetwork`]: nodes, segments, profiles and roads of the `RNgp` group in the world metadata.
//! - [`SignalController`]: traffic lights for the junctions (an extension: the original has none).
//! - [`TrackZones`]: the typed polygons of the track path manager (traffic patterns, tunnels...).
//!
//! Coordinates are the game's physics space: x right, y up, z forward, metres. This crate never
//! touches the filesystem: it takes bytes.

mod avoidance;
mod barriers;
mod bytes;
mod curve;
mod error;
mod gps;
mod heli_sheet;
mod index;
mod junction_crossing;
mod lane;
mod locate;
mod nav;
mod network;
mod node;
mod path;
mod profile;
mod road;
mod route;
mod segment;
mod signal;
mod signal_props;
mod trail;
mod zones;

#[cfg(test)]
mod tests;

pub use avoidance::{
    AHEAD_EXTRA, AHEAD_SECONDS, BEHIND, BLEND_SECONDS, Blend, Blocked, CHANGE_AFTER, Command, Crossing, GIVE_UP_AFTER,
    Gate, HONK_AFTER, LaneChange, MIN_BLEND_SPEED, Other, Phase, RETURN_AFTER, SLOW, STANDING, STUCK_AFTER, Situation,
    WAIT_FOR_GAP, adjacent_lanes, blend_length, lane_clear, smoothstep, time_to_closest,
};
pub use barriers::{
    ActiveBarrier, BARRIER_LEN, TrackBarrier, mark_barrier_crossings, parse_track_barriers, read_track_barriers,
    segment_crosses,
};
pub use curve::{Bezier, MIN_CHORD};
pub use error::{Error, Result};
pub use gps::{
    ARRIVAL_DISTANCE, Gps, GpsEnd, GpsState, LOOK_AHEAD_CATCH_UP, LOOK_AHEAD_MAX, LOOK_AHEAD_MIN, LOOK_AHEAD_SECONDS,
    MAX_DEVIATION,
};
pub use heli_sheet::{HeliSheet, Triangle as HeliTriangle};
pub use index::SegmentIndex;
pub use junction_crossing::{SAME_WAY, crossing_point, earliest_oncoming};
pub use lane::{NodeInd, centre_line, lane_line, lane_line_shifted, right_of, travel_profile};
pub use locate::{Located, SEARCH_RADIUS, SegmentFilter, closest_segment};
pub use nav::{LaneType, NavKind, RandomSource, RoadNav, SplitMix, forward_traffic_lanes, nth_from_centre, pick_lane};
pub use network::RoadNetwork;
pub use node::RoadNode;
pub use path::{MAX_PATH_SEGMENTS, PathRequest, PathResult, PathState, PathType, find_path};
pub use profile::{RoadProfile, Zone, zone};
pub use road::Road;
pub use route::{
    CHECK_SPACING, END_REACH, Goal, Leg, LegPath, LegSource, Route, RouteBuilder, RoutePoint, RouteStep, build_leg,
    build_route,
};
pub use segment::{RoadSegment, flags};
pub use signal::{
    ALL_RED_TIME, AMBER_TIME, Approach, DEFAULT_KERB, GREEN_TIME, Junction, Light, MIN_SIGNAL_APPROACHES,
    OFFSET_PER_JUNCTION, PHASE_ALIGNMENT, PHASES, STOP_LINE_DISTANCE, SignalController, Timing,
};
pub use signal_props::{MatchParams, PropPose, match_props};
pub use trail::Body;
pub use trail::{
    Avoidable, CAPACITY as TRAIL_CAPACITY, CUT, CUT_BEHIND, Cookie, DEFAULT_GAP, Occlusion, Trail, trail_curvature,
    update_occluded_position,
};
pub use zones::{TrackZone, TrackZones, from_zone_space, to_zone_space};
