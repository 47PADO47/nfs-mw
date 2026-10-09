//! The cursor's look-ahead trail: recording cookies as it moves.
//! Spec: `docs/specs/ai-road-nav-trail.md` (§1, §2).

use super::{LaneType, NavKind, RoadNav};
use crate::trail::{Cookie, DEFAULT_GAP, Trail, xz};
use crate::{RoadNetwork, lane_line_shifted, travel_profile};

/// Default corridor half width round a lane that is not a traffic lane, metres.
const DEFAULT_BOUND: f32 = 2.0;
/// Room kept between a car and the edge of the road, beyond its half width.
const EDGE_MARGIN: f32 = 1.5;

impl RoadNav {
    pub fn trail(&self) -> Option<&Trail> {
        self.trail.as_ref()
    }

    /// Starts keeping a trail (and records its first cookie at the cursor).
    pub fn enable_trail(&mut self, net: &RoadNetwork) {
        self.trail = Some(Trail::default());
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

    /// The corridor round the lane at the cursor: the bounds' lateral offsets from the lane line.
    fn corridor(&self, net: &RoadNetwork) -> (f32, f32) {
        if self.kind == NavKind::Traffic {
            return (-self.half_width * 0.5, self.half_width * 0.5);
        }
        let heading_end = self.t >= 0.5;
        let profile = travel_profile(net, self.segment, self.node_ind, heading_end);
        let lane_offset = |end: bool| {
            let p = travel_profile(net, self.segment, self.node_ind, end);
            p.zones.get(self.lane).map(|_| p.signed_offset(self.lane))
        };
        let offset = match (lane_offset(false), lane_offset(true)) {
            (Some(a), Some(b)) => a * (1.0 - self.t) + b * self.t,
            (Some(o), None) | (None, Some(o)) => o,
            (None, None) => 0.0,
        };
        if profile.zones.is_empty() {
            return (-DEFAULT_BOUND, DEFAULT_BOUND);
        }
        let mask = match self.lane_type {
            LaneType::Traffic => LaneType::Racing.drivable_mask(),
            other => other.drivable_mask(),
        };
        let drivable = |i: usize| profile.zones[i].in_mask(mask);
        let nearest = (0..profile.zones.len()).filter(|&i| drivable(i)).min_by(|&a, &b| {
            (profile.signed_offset(a) - offset).abs().total_cmp(&(profile.signed_offset(b) - offset).abs())
        });
        let Some(mid) = nearest else { return (-DEFAULT_BOUND, DEFAULT_BOUND) };
        let (mut lo, mut hi) = (mid, mid);
        while lo > 0 && drivable(lo - 1) {
            lo -= 1;
        }
        while hi + 1 < profile.zones.len() && drivable(hi + 1) {
            hi += 1;
        }
        let margin = self.half_width + EDGE_MARGIN;
        let mut left = profile.signed_offset(lo) - profile.zones[lo].width * 0.5 + margin;
        let mut right = profile.signed_offset(hi) + profile.zones[hi].width * 0.5 - margin;
        if right - left < 2.0 * margin {
            let shortfall = 2.0 * margin - (right - left);
            left -= shortfall * 0.5;
            right += shortfall * 0.5;
        }
        (left - offset, right - offset)
    }

    /// The cross-section of the corridor at the cursor.
    pub fn cookie(&self, net: &RoadNetwork) -> Cookie {
        let (left, right) = self.corridor(net);
        let bound =
            |delta: f32| xz(lane_line_shifted(net, self.segment, self.node_ind, self.lane, delta).position(self.t));
        Cookie::new(self.position, bound(left), bound(right), self.curvature, self.segment)
    }
}
