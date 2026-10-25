//! The cursor's look-ahead trail: recording cookies as it moves.
//! Spec: `docs/specs/ai-road-nav-trail.md` (§1, §2).

use super::{LaneType, NavKind, RoadNav};
use crate::lane::{lane_line_at_offsets, lane_line_from_endpoints};
use crate::trail::{Cookie, DEFAULT_GAP, Trail, xz};
use crate::{Bezier, RoadNetwork, lane_line, travel_profile};

/// Default corridor half width round a lane that is not a traffic lane, metres.
const DEFAULT_BOUND: f32 = 2.0;
/// Room kept between a car and the edge of the road, beyond its half width.
const EDGE_MARGIN: f32 = 1.5;
/// The centre spline stays this far inside the inset bounds.
const CENTRE_INSET: f32 = 0.1;

struct Endpoint {
    offset: f32,
    left: f32,
    right: f32,
}

impl RoadNav {
    pub fn trail(&self) -> Option<&Trail> {
        self.trail.as_ref()
    }

    /// Starts keeping a trail (and records its first cookie at the cursor).
    pub fn enable_trail(&mut self, net: &RoadNetwork) {
        self.trail = Some(Trail::default());
        self.rebuild(net);
        self.reset_trail(net);
    }

    /// Clears the trail and records one cookie at the cursor.
    pub fn reset_trail(&mut self, net: &RoadNetwork) {
        if self.trail.is_none() {
            return;
        }
        let cookie = self.cookie(net);
        if let Some(trail) = self.trail.as_mut() {
            trail.reset(cookie);
        }
    }

    /// Records a cookie at the cursor if it is `gap` metres from the newest one.
    pub(super) fn record_cookie(&mut self, net: &RoadNetwork, gap: f32) {
        if self.trail.is_none() {
            return;
        }
        let cookie = self.cookie(net);
        if let Some(trail) = self.trail.as_mut() {
            trail.record(cookie, gap);
        }
    }

    /// The distance between cookies for a look-ahead of `max_lookahead` metres (3 m when it is not given).
    pub(super) fn cookie_gap(max_lookahead: f32) -> f32 {
        match max_lookahead > 0.0 {
            true => (max_lookahead / 26.0).clamp(1.0, DEFAULT_GAP),
            false => DEFAULT_GAP,
        }
    }

    /// Absolute lane and corridor offsets at one endpoint, in the travel frame.
    fn endpoint(&self, net: &RoadNetwork, heading_end: bool, margin: f32) -> Endpoint {
        let profile = travel_profile(net, self.segment, self.node_ind, heading_end);
        let indexed_offset = match profile.zones.is_empty() {
            true => 0.0,
            false => profile.signed_offset(self.lane.min(profile.zones.len() - 1)),
        };
        let offset = self.lane_offset.unwrap_or(indexed_offset);
        let default = Endpoint { offset, left: offset - DEFAULT_BOUND, right: offset + DEFAULT_BOUND };
        if profile.zones.is_empty() {
            return default;
        }
        let mask = match self.lane_type {
            LaneType::Traffic => LaneType::Racing.drivable_mask(),
            other => other.drivable_mask(),
        };
        let drivable = |i: usize| profile.zones[i].in_mask(mask);
        let nearest = (0..profile.zones.len()).filter(|&i| drivable(i)).min_by(|&a, &b| {
            (profile.signed_offset(a) - offset).abs().total_cmp(&(profile.signed_offset(b) - offset).abs())
        });
        let Some(mid) = nearest else { return default };
        let (mut lo, mut hi) = (mid, mid);
        while lo > 0 && drivable(lo - 1) {
            lo -= 1;
        }
        while hi + 1 < profile.zones.len() && drivable(hi + 1) {
            hi += 1;
        }
        let mut left = profile.signed_offset(lo) - profile.zones[lo].width * 0.5 + margin;
        let mut right = profile.signed_offset(hi) + profile.zones[hi].width * 0.5 - margin;
        if right - left < 2.0 * margin {
            let shortfall = 2.0 * margin - (right - left);
            left -= shortfall * 0.5;
            right += shortfall * 0.5;
        }
        Endpoint { offset, left, right }
    }

    /// Build the centre from the same endpoint corridors as its active trail.
    pub(super) fn drive_line(&self, net: &RoadNetwork) -> Bezier {
        if self.trail.is_none() || self.kind == NavKind::Traffic {
            if let Some(offset) = self.lane_offset {
                return lane_line_at_offsets(net, self.segment, self.node_ind, offset, offset);
            }
            return lane_line(net, self.segment, self.node_ind, self.lane);
        }
        let margin = self.half_width + EDGE_MARGIN;
        let offset = |end| {
            let endpoint = self.endpoint(net, end, margin);
            endpoint.offset.clamp(endpoint.left + CENTRE_INSET, endpoint.right - CENTRE_INSET)
        };
        lane_line_at_offsets(net, self.segment, self.node_ind, offset(false), offset(true))
    }

    fn corridor_lines(&self, net: &RoadNetwork, margin: f32) -> [Bezier; 2] {
        let start = self.endpoint(net, false, margin);
        let end = self.endpoint(net, true, margin);
        [(start.left, end.left), (start.right, end.right)]
            .map(|(a, b)| lane_line_at_offsets(net, self.segment, self.node_ind, a, b))
    }

    /// The cross-section of the corridor at the cursor.
    pub fn cookie(&self, net: &RoadNetwork) -> Cookie {
        let [left, right] = self.bound_lines(net).map(|line| xz(line.position(self.t)));
        Cookie::new(self.position, left, right, self.curvature, self.segment)
    }

    /// Uninset drivable road edges at this cursor, for placing objects across the road.
    /// Unlike the steering corridor, these edges reserve no room for a moving car.
    pub fn road_cross_section(&self, net: &RoadNetwork) -> Cookie {
        let [left, right] = self.corridor_lines(net, 0.0).map(|line| xz(line.position(self.t)));
        Cookie::new(self.position, left, right, self.curvature, self.segment)
    }

    /// Lane bounds use the same join as the centre, so the trail cannot snap back to the node's layout.
    pub(super) fn bound_lines(&self, net: &RoadNetwork) -> [Bezier; 2] {
        let lines = match self.kind {
            NavKind::Traffic => {
                let start = self.endpoint(net, false, 0.0).offset;
                let end = self.endpoint(net, true, 0.0).offset;
                [-self.half_width * 0.5, self.half_width * 0.5]
                    .map(|delta| lane_line_at_offsets(net, self.segment, self.node_ind, start + delta, end + delta))
            }
            _ => self.corridor_lines(net, self.half_width + EDGE_MARGIN),
        };
        let Some(starts) = self.bound_starts else { return lines };
        std::array::from_fn(|i| lane_line_from_endpoints(net, self.segment, self.node_ind, starts[i], lines[i].end()))
    }
}
