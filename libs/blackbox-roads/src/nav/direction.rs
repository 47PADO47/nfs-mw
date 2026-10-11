//! Direction navigators: go where a target vector points.
//! Spec: `docs/specs/ai-road-network.md` (§4.2, §4.4).

use glam::Vec3;

use super::next::{Step, leaving_from};
use super::{LaneType, RoadNav};
use crate::{RoadNetwork, centre_line, travel_profile};

/// Junction exit walk-ahead: segments and metres.
const WALK_SEGMENTS: usize = 19;
const WALK_METRES: f32 = 100.0;

impl RoadNav {
    pub(super) fn next_direction(&self, net: &RoadNetwork, toward: Vec3) -> Step {
        let current = net.segment(self.segment);
        let node = current.nodes[self.node_ind];
        let toward = toward.try_normalize().unwrap_or(self.forward);
        let turn_around = Step::To { segment: self.segment, node_ind: 1 - self.node_ind, lane: self.lane };
        let attached = &net.node(node).segments;
        if attached.len() < 2 {
            return turn_around;
        }
        let plain = net.plain_segment_at(node, self.segment);
        let entering_junction = !current.is_decision() && attached.iter().any(|&s| net.segment(s).is_decision());
        if !entering_junction {
            return match plain {
                Some(next) if self.may_enter(net, next, node) => self.snap_onto(net, next, node),
                _ => turn_around,
            };
        }
        let mut best: Option<(f32, u16)> = None;
        for &d in attached {
            if d == self.segment || !net.segment(d).is_decision() || !self.may_enter(net, d, node) {
                continue;
            }
            let Some(deviation) = self.walk_deviation(net, d, node, toward, best.map(|b| b.0)) else { continue };
            if best.is_none_or(|b| deviation < b.0) {
                best = Some((deviation, d));
            }
        }
        match best {
            Some((_, d)) => self.snap_onto(net, d, node),
            None => turn_around,
        }
    }

    /// The worst deviation from `toward` along the walk that starts with decision segment `first`, or
    /// `None` when the walk breaks a rule or is already worse than `limit`.
    fn walk_deviation(
        &self,
        net: &RoadNetwork,
        first: u16,
        node: u16,
        toward: Vec3,
        limit: Option<f32>,
    ) -> Option<f32> {
        let (mut segment, mut from) = (first, node);
        let (mut worst, mut walked) = (0.0f32, 0.0f32);
        for _ in 0..WALK_SEGMENTS {
            if !self.may_enter(net, segment, from) {
                return None;
            }
            let seg = net.segment(segment);
            let dir = centre_line(net, segment, leaving_from(seg, from)).tangent(0.5).try_normalize()?;
            worst = worst.max((dir.dot(toward) - 1.0).abs());
            if limit.is_some_and(|l| worst >= l) {
                return None;
            }
            walked += seg.length;
            if walked >= WALK_METRES {
                break;
            }
            let to = seg.other_node(from)?;
            // Continue through the plain segment of the node reached.
            match net.plain_segment_at(to, segment) {
                Some(next) => (segment, from) = (next, to),
                None => break,
            }
        }
        Some(worst)
    }

    /// The step onto `segment`, keeping the nearest selectable lane to the current offset.
    pub(super) fn snap_onto(&self, net: &RoadNetwork, segment: u16, node: u16) -> Step {
        let node_ind = leaving_from(net.segment(segment), node);
        let from = travel_profile(net, self.segment, self.node_ind, true);
        let offset = from.zones.get(self.lane).map(|_| from.signed_offset(self.lane)).unwrap_or(0.0);
        let profile = travel_profile(net, segment, node_ind, false);
        let mask = self.lane_type.selectable_mask();
        let both_sides = matches!(
            self.lane_type,
            LaneType::Cop | LaneType::Racing | LaneType::Drag | LaneType::StartingGrid | LaneType::CopReckless
        );
        let middle = usize::from(profile.middle);
        let nearest = (0..profile.zones.len())
            .filter(|&i| profile.zones[i].in_mask(mask) && (both_sides || i >= middle))
            .min_by(|&a, &b| {
                (profile.signed_offset(a) - offset).abs().total_cmp(&(profile.signed_offset(b) - offset).abs())
            });
        Step::To { segment, node_ind, lane: nearest.unwrap_or(middle.min(profile.zones.len().saturating_sub(1))) }
    }
}
