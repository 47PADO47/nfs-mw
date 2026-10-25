//! Optional continuous lateral geometry; indexed traffic retains its existing lane curves.
//! Spec: ai-pursuit-reinforcement-prediction.md, continuous driving offsets.

use glam::Vec3;

use super::RoadNav;
use crate::{RoadNetwork, centre_line, right_of, travel_profile};

impl RoadNav {
    /// Retain the physical position's lateral offset on the already selected road.
    /// This changes geometry only: segment, travel direction, progress and path stay intact.
    pub fn preserve_offset_at(&mut self, net: &RoadNetwork, position: Vec3) {
        let centre = centre_line(net, self.segment, self.node_ind);
        let side = right_of(centre.tangent(self.t));
        let offset = (position - centre.position(self.t)).dot(side);
        self.set_lateral_offset(net, offset);
        self.reset_trail(net);
    }

    pub(crate) fn continuous_offset(&self) -> Option<f32> {
        self.lane_offset
    }

    pub(crate) fn set_lateral_offset(&mut self, net: &RoadNetwork, offset: f32) {
        if !offset.is_finite() {
            return;
        }
        self.lane_offset = Some(offset);
        self.rebuild(net);
    }

    /// Select the nearest allowed lane interval, retaining the requested position inside it.
    /// Cop/racing geometry uses the heading-end profile and the connected corridor's safety inset.
    /// Returns the lateral correction, or `None` when no selectable lane exists.
    pub fn snap_offset_at(&mut self, net: &RoadNetwork, position: Vec3) -> Option<f32> {
        let centre = centre_line(net, self.segment, self.node_ind);
        let requested = (position - centre.position(self.t)).dot(right_of(centre.tangent(self.t)));
        if !requested.is_finite() {
            return None;
        }
        let profile = travel_profile(net, self.segment, self.node_ind, true);
        let selectable = self.lane_type.selectable_mask();
        let lane = (0..profile.zones.len()).filter(|&i| profile.zones[i].in_mask(selectable)).min_by(|&a, &b| {
            (profile.signed_offset(a) - requested).abs().total_cmp(&(profile.signed_offset(b) - requested).abs())
        });
        let lane = lane?;
        let offset = profile.signed_offset(lane);
        let half = profile.zones[lane].width * 0.5;
        let candidate = requested.clamp(offset - half, offset + half);
        let mask = self.lane_type.drivable_mask();
        let (mut left, mut right) = (lane, lane);
        while left > 0 && profile.zones[left - 1].in_mask(mask) {
            left -= 1;
        }
        while right + 1 < profile.zones.len() && profile.zones[right + 1].in_mask(mask) {
            right += 1;
        }
        let margin = self.half_width + 1.5;
        let low = profile.signed_offset(left) - profile.zones[left].width * 0.5 + margin;
        let high = profile.signed_offset(right) + profile.zones[right].width * 0.5 - margin;
        let offset = match low <= high {
            true => candidate.clamp(low, high),
            false => (low + high) * 0.5,
        };
        self.lane = lane;
        self.set_lateral_offset(net, offset);
        Some((requested - offset).abs())
    }
}
