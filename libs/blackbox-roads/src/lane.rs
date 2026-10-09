//! Lane lines: the curve a car in one lane of a segment follows, for one direction of travel.
//! Spec: `docs/specs/ai-road-network.md` (§1.1, §1.2, §1.4).

use glam::Vec3;

use crate::curve::{Bezier, MIN_CHORD};
use crate::profile::RoadProfile;
use crate::{RoadNetwork, flags};

/// Which end of a segment a car heads to: `1` is the stored direction, `0` is against it.
pub type NodeInd = usize;

/// The profile of `segment` at the node a car leaves from (`from`) or heads to (`to`), read in the
/// car's travel frame: lanes right of the middle run in the travel direction and offsets are positive to
/// the right of travel.
pub fn travel_profile(net: &RoadNetwork, segment: u16, node_ind: NodeInd, heading_end: bool) -> RoadProfile {
    let seg = net.segment(segment);
    // The end the profile belongs to: heading to node 1 leaves from 0 and arrives at 1.
    let end = match heading_end {
        true => node_ind,
        false => 1 - node_ind,
    };
    let profile = net.profile_at(segment, seg.nodes[end]);
    match node_ind == 0 {
        true => profile.inverted(),
        false => profile,
    }
}

/// The right-of-travel vector (planar, unit length) for a direction of travel.
pub fn right_of(forward: Vec3) -> Vec3 {
    Vec3::new(forward.z, 0.0, -forward.x).try_normalize().unwrap_or(Vec3::X)
}

/// The centre-line curve of `segment` in the travel direction `node_ind` (handles point into the curve).
pub fn centre_line(net: &RoadNetwork, segment: u16, node_ind: NodeInd) -> Bezier {
    let seg = net.segment(segment);
    let (a, b) = (net.node(seg.nodes[0]).position, net.node(seg.nodes[1]).position);
    if !seg.has(flags::CURVED) {
        return match node_ind {
            1 => Bezier::line(a, b),
            _ => Bezier::line(b, a),
        };
    }
    match node_ind {
        1 => Bezier::with_handles(a, seg.start_handle, seg.end_handle, b),
        _ => Bezier::with_handles(b, seg.end_handle, seg.start_handle, a),
    }
}

/// The curve of the lane `lane` (index into the travel-frame profile) of `segment`. A lane index beyond
/// the zones of one end is clamped to that end's last zone.
pub fn lane_line(net: &RoadNetwork, segment: u16, node_ind: NodeInd, lane: usize) -> Bezier {
    lane_line_shifted(net, segment, node_ind, lane, 0.0)
}

/// The lane line moved `delta` metres to the right of travel (to the left when negative): the bounds of
/// the corridor round a lane.
pub fn lane_line_shifted(net: &RoadNetwork, segment: u16, node_ind: NodeInd, lane: usize, delta: f32) -> Bezier {
    let centre = centre_line(net, segment, node_ind);
    let offset_at = |heading_end: bool| {
        let profile = travel_profile(net, segment, node_ind, heading_end);
        match profile.zones.is_empty() {
            true => 0.0,
            false => profile.signed_offset(lane.min(profile.zones.len() - 1)) + delta,
        }
    };
    let [p0, p1, p2, p3] = centre.points;
    let (forward_start, forward_end) = (p1 - p0, p3 - p2);
    let start = p0 + right_of(forward_start) * offset_at(false);
    let end = p3 + right_of(forward_end) * offset_at(true);
    let scale = start.distance(end) / p0.distance(p3).max(MIN_CHORD);
    Bezier { points: [start, start + (p1 - p0) * scale, end + (p2 - p3) * scale, end] }
}
