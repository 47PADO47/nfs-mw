//! What kind of road a segment is on the map, from what the network stores (`docs/formats/road-network.md`):
//!
//! - **Path:** a segment flagged `NO_TRAFFIC`. Traffic never drives it: alleys, lots and the like. The Expanded map
//!   alone draws paths.
//! - **Main:** a segment whose end profiles have at least [`MAIN_LANES`] traffic lanes between them (two each way), or
//!   a median or barrier strip (a divided road).
//! - **Side:** every other segment.
//!
//! There is no shortcut class: the network's `SHORTCUT` flag is zero in the file and only race setup would set it,
//! which the rewrite does not do, so nothing could be classed by it.
//!
//! Junction connectors (`DECISION` segments) join every way in to every way out, so drawing them all would fill each
//! junction with a fan of curves. Only the ones that go through, turning less than [`THROUGH_DEGREES`], are kept:
//! they close the gap the junction leaves between its roads, and draw a plain crossing.

use blackbox_roads::{RoadNetwork, RoadProfile, centre_line, flags, zone};

/// The traffic lanes (both ways together) from which a road is a main road.
pub const MAIN_LANES: usize = 4;
/// A junction connector turning less than this, degrees, goes through the junction and is drawn.
pub const THROUGH_DEGREES: f32 = 35.0;

/// The kind of a road on the map, drawn in its own width and tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoadClass {
    Main,
    Side,
    Path,
}

/// The class of segment `segment`, or `None` for a junction connector that turns.
pub(super) fn class_of(net: &RoadNetwork, segment: u16) -> Option<RoadClass> {
    let seg = net.segment(segment);
    if seg.is_decision() && !goes_through(net, segment) {
        return None;
    }
    if seg.has(flags::NO_TRAFFIC) {
        return Some(RoadClass::Path);
    }
    let profiles = seg.nodes.map(|node| &net.profiles[usize::from(net.node(node).profile)]);
    let lanes = profiles.iter().map(|profile| traffic_lanes(profile)).max().unwrap_or(0);
    let divided = profiles.iter().any(|profile| is_divided(profile));
    if lanes >= MAIN_LANES || divided {
        return Some(RoadClass::Main);
    }
    Some(RoadClass::Side)
}

/// The traffic lanes of a profile, both ways.
fn traffic_lanes(profile: &RoadProfile) -> usize {
    profile.zones.iter().filter(|z| z.kind == zone::TRAFFIC).count()
}

/// Whether a profile has a median or a barrier strip.
fn is_divided(profile: &RoadProfile) -> bool {
    profile.zones.iter().any(|z| z.kind == zone::MEDIAN || z.kind == zone::BARRIER)
}

/// Whether the connector `segment` goes through its junction: the angle between its direction at the start and at the
/// end, on the ground, is under [`THROUGH_DEGREES`]. A curve with no direction at an end counts as straight.
fn goes_through(net: &RoadNetwork, segment: u16) -> bool {
    let curve = centre_line(net, segment, 1);
    let (a, b) = (curve.tangent(0.0), curve.tangent(1.0));
    let (a, b) = ([a.x, a.z], [b.x, b.z]);
    let small = |v: [f32; 2]| v[0].hypot(v[1]) <= 1e-4;
    if small(a) || small(b) {
        return true;
    }
    let cross = a[0] * b[1] - a[1] * b[0];
    let dot = a[0] * b[0] + a[1] * b[1];
    cross.atan2(dot).abs().to_degrees() < THROUGH_DEGREES
}
