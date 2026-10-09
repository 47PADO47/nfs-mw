//! Choosing the segment after the one the cursor leaves.
//! Spec: `docs/specs/ai-road-network.md` (§4.2–§4.4).

use glam::Vec3;

use super::traffic_lanes::{forward_traffic_lanes, nth_from_centre, pick_lane};
use super::{NavKind, RandomSource, RoadNav};
use crate::{RoadNetwork, RoadSegment, centre_line, right_of, travel_profile};

/// What the cursor does at the end of its segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    /// There is nowhere to go.
    Stay,
    To {
        segment: u16,
        node_ind: usize,
        lane: usize,
    },
}

/// The end of `segment` that touches `node`, as the `node_ind` of a cursor that leaves from `node`.
pub(super) fn leaving_from(seg: &RoadSegment, node: u16) -> usize {
    match seg.nodes[0] == node {
        true => 1,
        false => 0,
    }
}

impl RoadNav {
    pub(super) fn next_segment(&self, net: &RoadNetwork, toward: Vec3, rng: &mut impl RandomSource) -> Step {
        match self.kind {
            NavKind::Traffic => self.next_traffic(net, toward, rng),
            NavKind::Direction => self.next_direction(net, toward),
            NavKind::Path => self.next_path(net, toward),
        }
    }

    /// Whether a cursor of this kind may enter `segment` leaving from `node`.
    pub(super) fn may_enter(&self, net: &RoadNetwork, segment: u16, node: u16) -> bool {
        let seg = net.segment(segment);
        if !self.filter.allows(seg) {
            return false;
        }
        !(seg.is_one_way() && seg.nodes[0] != node)
    }

    /// The traffic lane position (from the centre) of the lane the cursor is in at the end of its segment.
    fn traffic_nth(&self, net: &RoadNetwork) -> usize {
        let end = travel_profile(net, self.segment, self.node_ind, true);
        nth_from_centre(&end, self.lane).unwrap_or(0)
    }

    fn next_traffic(&self, net: &RoadNetwork, toward: Vec3, rng: &mut impl RandomSource) -> Step {
        let current = net.segment(self.segment);
        let node = current.nodes[self.node_ind];
        let attached = &net.node(node).segments;
        if attached.len() < 2 {
            return Step::Stay;
        }
        let nth = self.traffic_nth(net);
        let into_junction =
            !current.is_decision() && attached.iter().any(|&s| s != self.segment && net.segment(s).is_decision());
        if into_junction {
            return self.junction_exit(net, node, nth, toward, rng);
        }
        let Some(next) = net.plain_segment_at(node, self.segment) else { return Step::Stay };
        if !self.may_enter(net, next, node) {
            return Step::Stay;
        }
        self.continue_on(net, next, node, nth, false)
    }

    /// The step onto `segment`, leaving from `node`, in traffic lane `nth` of its profile.
    fn continue_on(&self, net: &RoadNetwork, segment: u16, node: u16, nth: usize, from_curb: bool) -> Step {
        let node_ind = leaving_from(net.segment(segment), node);
        let start = travel_profile(net, segment, node_ind, false);
        Step::To { segment, node_ind, lane: pick_lane(&start, nth, from_curb) }
    }

    /// Picks a decision segment at the junction node and the exit road behind it.
    fn junction_exit(
        &self,
        net: &RoadNetwork,
        node: u16,
        nth: usize,
        toward: Vec3,
        rng: &mut impl RandomSource,
    ) -> Step {
        struct Candidate {
            decision: u16,
            exit_dir: Vec3,
        }
        let mut candidates = Vec::new();
        for &d in &net.node(node).segments {
            let seg = net.segment(d);
            if d == self.segment || !seg.is_decision() || !self.may_enter(net, d, node) {
                continue;
            }
            let Some(far) = seg.other_node(node) else { continue };
            let Some(exit) = net.plain_segment_at(far, d) else { continue };
            if !self.may_enter(net, exit, far) {
                continue;
            }
            let exit_ind = leaving_from(net.segment(exit), far);
            let exit_start = travel_profile(net, exit, exit_ind, false);
            if forward_traffic_lanes(&exit_start).is_empty() {
                continue;
            }
            let exit_dir = centre_line(net, exit, exit_ind).tangent(0.0).try_normalize().unwrap_or(Vec3::Z);
            candidates.push(Candidate { decision: d, exit_dir });
        }
        if candidates.is_empty() {
            return Step::Stay;
        }

        // How far right each exit turns, to find the rightmost entrance (the curb lane's exit).
        let right = right_of(self.forward);
        let turn = |c: &Candidate| right.dot(c.exit_dir).atan2(self.forward.dot(c.exit_dir));
        let rightmost = (0..candidates.len()).max_by(|&a, &b| turn(&candidates[a]).total_cmp(&turn(&candidates[b])));

        let pick = match toward.try_normalize() {
            Some(t) => (0..candidates.len())
                .max_by(|&a, &b| candidates[a].exit_dir.dot(t).total_cmp(&candidates[b].exit_dir.dot(t)))
                .unwrap_or(0),
            None => {
                let first = rng.index(candidates.len());
                let lanes = forward_traffic_lanes(&travel_profile(net, self.segment, self.node_ind, true));
                let nth_from_curb = lanes.len().saturating_sub(1).saturating_sub(nth);
                // Turning right out of an inner lane would cut across the others: last resort.
                let last_resort = Some(first) == rightmost && nth_from_curb > 0;
                match last_resort {
                    true => (first + 1) % candidates.len(),
                    false => first,
                }
            }
        };
        let from_curb = Some(pick) == rightmost;
        self.continue_on(net, candidates[pick].decision, node, nth, from_curb)
    }
}
