//! Placing a navigator on the graph.
//! Spec: `docs/specs/ai-road-network.md` (§3).

use glam::Vec3;

use super::traffic_lanes::{forward_traffic_lanes, pick_lane};
use super::{LaneType, NavKind, RandomSource, RoadNav};
use crate::{RoadNetwork, SegmentFilter, SegmentIndex, closest_segment, flags, lane_line, travel_profile};

/// The default half width of a car.
const DEFAULT_HALF_WIDTH: f32 = 1.0;

impl RoadNav {
    /// A navigator that is not on the graph yet.
    pub fn new(kind: NavKind, lane_type: LaneType, filter: SegmentFilter) -> Self {
        Self {
            kind,
            lane_type,
            filter,
            segment: 0,
            node_ind: 1,
            t: 0.0,
            lane: 0,
            dead_end: false,
            half_width: DEFAULT_HALF_WIDTH,
            position: Vec3::ZERO,
            forward: Vec3::Z,
            curvature: 0.0,
            line: crate::Bezier::line(Vec3::ZERO, Vec3::Z),
        }
    }

    /// A traffic navigator: traffic lanes only, on segments traffic may use.
    pub fn traffic() -> Self {
        Self::new(NavKind::Traffic, LaneType::Traffic, SegmentFilter { traffic: true, ..SegmentFilter::default() })
    }

    /// Puts the cursor on `segment` heading to `node_ind`, in `lane` (an index of the travel-frame
    /// profile) at progress `t`.
    pub fn place(&mut self, net: &RoadNetwork, segment: u16, node_ind: usize, lane: usize, t: f32) {
        self.segment = segment;
        self.node_ind = node_ind;
        self.lane = lane;
        self.t = t.clamp(0.0, 1.0);
        self.dead_end = false;
        self.rebuild(net);
    }

    /// Puts the cursor on the closest segment to `position`, heading the way `heading` points, in the
    /// nearest lane (the centre lane when `force_centre_lane`). Returns whether a segment was found.
    pub fn init_at_point(
        &mut self,
        net: &RoadNetwork,
        index: &SegmentIndex,
        position: Vec3,
        heading: Vec3,
        force_centre_lane: bool,
    ) -> bool {
        let Some(hit) = closest_segment(net, index, position, heading, 1.0, self.filter) else {
            return false;
        };
        self.init_at_segment(net, hit.segment, hit.t, position, heading, force_centre_lane);
        true
    }

    /// Like [`RoadNav::init_at_point`] on a given segment at stored-direction parameter `t`.
    pub fn init_at_segment(
        &mut self,
        net: &RoadNetwork,
        segment: u16,
        t: f32,
        position: Vec3,
        heading: Vec3,
        force_centre_lane: bool,
    ) {
        let seg = net.segment(segment);
        let stored = crate::centre_line(net, segment, 1);
        let forward = stored.tangent(t).try_normalize().unwrap_or(Vec3::Z);
        let node_ind = match seg.is_one_way() || heading.dot(forward) >= 0.0 {
            true => 1,
            false => 0,
        };
        let t_travel = match node_ind {
            1 => t,
            _ => 1.0 - t,
        };
        let offset = match force_centre_lane {
            true => 0.0,
            false => {
                (position - stored.position(t)).dot(crate::right_of(forward)) * if node_ind == 1 { 1.0 } else { -1.0 }
            }
        };
        let lane = self.nearest_lane(net, segment, node_ind, t_travel, offset);
        self.place(net, segment, node_ind, lane, t_travel);
    }

    /// The lane whose offset at `t` is closest to `offset`: the traffic lanes for a traffic cursor,
    /// every drivable zone otherwise.
    fn nearest_lane(&self, net: &RoadNetwork, segment: u16, node_ind: usize, t: f32, offset: f32) -> usize {
        let (from, to) = (travel_profile(net, segment, node_ind, false), travel_profile(net, segment, node_ind, true));
        let candidates: Vec<usize> = match self.kind {
            NavKind::Traffic => forward_traffic_lanes(&to),
            NavKind::Direction => {
                let mask = self.lane_type.drivable_mask();
                (0..to.zones.len()).filter(|&i| to.zones[i].in_mask(mask)).collect()
            }
        };
        let at = |lane: usize| {
            let (a, b) = (from.zones.len().checked_sub(1), to.zones.len().checked_sub(1));
            let (Some(a), Some(b)) = (a, b) else { return 0.0 };
            from.signed_offset(lane.min(a)) * (1.0 - t) + to.signed_offset(lane.min(b)) * t
        };
        candidates
            .into_iter()
            .min_by(|&x, &y| (at(x) - offset).abs().total_cmp(&(at(y) - offset).abs()))
            .unwrap_or_else(|| pick_lane(&to, 0, false))
    }

    /// Puts the cursor in traffic lane number `nth` (from the centre) of `segment`, heading the way the
    /// lane runs: lanes right of the profile's middle zone run in the stored direction, the others
    /// against it; one-way segments always run in the stored direction.
    pub fn init_in_lane(&mut self, net: &RoadNetwork, segment: u16, stored_lane: usize, t: f32) {
        let seg = net.segment(segment);
        let stored = net.profile_at(segment, seg.nodes[0]);
        let forward_lane = stored_lane >= usize::from(stored.middle);
        let node_ind = match seg.is_one_way() || forward_lane {
            true => 1,
            false => 0,
        };
        let lane = match node_ind {
            1 => stored_lane,
            _ => stored.zones.len() - 1 - stored_lane,
        };
        self.place(net, segment, node_ind, lane, t);
    }

    /// Whether a traffic car may be spawned where the cursor is: the segment is a plain road where traffic
    /// is allowed, not a one-way entered against its direction, and has a traffic lane in the direction of
    /// travel. When it can, the cursor becomes a traffic cursor in a random one of those lanes.
    pub fn can_traffic_spawn(&mut self, net: &RoadNetwork, rng: &mut impl RandomSource) -> bool {
        let seg = net.segment(self.segment);
        if seg.is_decision() || seg.has(flags::NO_TRAFFIC) || (seg.is_one_way() && self.node_ind == 0) {
            return false;
        }
        let lanes = forward_traffic_lanes(&travel_profile(net, self.segment, self.node_ind, false));
        if lanes.is_empty() {
            return false;
        }
        let lane = lanes[rng.index(lanes.len())];
        *self = Self { kind: NavKind::Traffic, lane_type: LaneType::Traffic, ..self.clone() };
        self.filter = SegmentFilter { traffic: true, ..SegmentFilter::default() };
        self.place(net, self.segment, self.node_ind, lane, self.t);
        true
    }

    /// Whether traffic may use the segment the cursor is on.
    pub fn on_legal_road(&self, net: &RoadNetwork) -> bool {
        !net.segment(self.segment).has(flags::NO_TRAFFIC)
    }

    /// Rebuilds the lane line, then evaluates the cursor on it.
    pub(super) fn rebuild(&mut self, net: &RoadNetwork) {
        self.line = lane_line(net, self.segment, self.node_ind, self.lane);
        self.evaluate();
    }

    pub(super) fn evaluate(&mut self) {
        self.position = self.line.position(self.t);
        self.forward = self.line.tangent(self.t).try_normalize().unwrap_or(self.forward);
        self.curvature = self.line.curvature_xz(self.t);
    }
}
