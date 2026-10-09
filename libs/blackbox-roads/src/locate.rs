//! Finding the road segment closest to a point.
//! Spec: `docs/specs/ai-road-network.md` (§3.1).

use glam::Vec3;

use crate::lane::{centre_line, right_of, travel_profile};
use crate::{RoadNetwork, SegmentIndex, flags};

/// Radius of the closest-segment search.
pub const SEARCH_RADIUS: f32 = 32.0;
/// Weight of the heading term.
const DIRECTION_WEIGHT: f32 = 10.0;
/// Initial best score.
const WORST_SCORE: f32 = 20_000.0;

/// Which segments a search may consider.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SegmentFilter {
    /// Only segments where traffic is allowed.
    pub traffic: bool,
    /// Only segments cops should consider (`NO_TRAFFIC` xor `COPS_XOR_TRAFFIC` is false).
    pub cop: bool,
    /// Skip junction connectors.
    pub no_decision: bool,
}

impl SegmentFilter {
    pub fn allows(&self, seg: &crate::RoadSegment) -> bool {
        let no_traffic = seg.has(flags::NO_TRAFFIC);
        if self.traffic && no_traffic {
            return false;
        }
        if self.cop && no_traffic != seg.has(flags::COPS_XOR_TRAFFIC) {
            return false;
        }
        !(self.no_decision && seg.is_decision())
    }
}

/// The result of a search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Located {
    pub segment: u16,
    /// Parameter along the stored direction (0 at node 0, 1 at node 1).
    pub t: f32,
    pub point: Vec3,
}

/// The segment closest to `point` among those near it. With `direction_weight > 0` the distance is the
/// distance to the edge of the road and `heading` breaks ties towards the segment direction.
pub fn closest_segment(
    net: &RoadNetwork,
    index: &SegmentIndex,
    point: Vec3,
    heading: Vec3,
    direction_weight: f32,
    filter: SegmentFilter,
) -> Option<Located> {
    let mut best: Option<(f32, Located)> = None;
    let mut best_score = WORST_SCORE;
    for s in index.near(point, SEARCH_RADIUS) {
        let seg = net.segment(s);
        if !filter.allows(seg) {
            continue;
        }
        let curve = centre_line(net, s, 1);
        let t = curve.closest_t(point, seg.length);
        let on_road = curve.position(t);
        let mut score = on_road.distance(point);
        if direction_weight > 0.0 {
            let forward = curve.tangent(t).try_normalize().unwrap_or(Vec3::Z);
            let lateral = (point - on_road).dot(right_of(forward)).abs();
            let half = half_width(net, s, t);
            let along = heading.try_normalize().map(|h| h.dot(forward)).unwrap_or(1.0);
            let alignment = match seg.is_one_way() {
                true => 1.0 - along,
                false => 1.0 - along.abs(),
            };
            score = (lateral - half).max(0.0) + direction_weight * DIRECTION_WEIGHT * alignment;
        }
        if score < best_score {
            best_score = score;
            best = Some((score, Located { segment: s, t, point: on_road }));
        }
    }
    best.map(|(_, located)| located)
}

/// Half the width of the drivable cross section of segment `s` at `t`, to the outer edge of its
/// outermost zones.
fn half_width(net: &RoadNetwork, s: u16, t: f32) -> f32 {
    let extent = |heading_end: bool| {
        let profile = travel_profile(net, s, 1, heading_end);
        let edge = |i: usize| profile.signed_offset(i).abs() + profile.zones[i].width * 0.5;
        match profile.zones.is_empty() {
            true => 0.0,
            false => edge(0).max(edge(profile.zones.len() - 1)),
        }
    };
    extent(false) * (1.0 - t) + extent(true) * t
}
