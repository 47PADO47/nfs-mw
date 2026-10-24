//! Retained road spots and the existing lane-index adapter.

use glam::Vec3;

use crate::{LaneType, NavKind, PathType, RoadNav, RoadNetwork, SegmentFilter, SplitMix, lane_line, travel_profile};

#[derive(Debug, Clone, Copy)]
pub(super) struct Spot {
    pub segment: u16,
    pub node: usize,
    pub t: f32,
    pub lane: usize,
}

impl Spot {
    pub fn of(nav: &RoadNav) -> Self {
        Self { segment: nav.segment, node: nav.node_ind, t: nav.t, lane: nav.lane }
    }

    pub fn nav(self, net: &RoadNetwork, point: Option<Vec3>) -> RoadNav {
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Cop, SegmentFilter::default());
        nav.path_type = PathType::Cop;
        nav.place(net, self.segment, self.node, self.lane, self.t);
        if let Some(point) = point {
            snap(net, &mut nav, point);
        }
        nav
    }
}

pub(super) fn snap(net: &RoadNetwork, nav: &mut RoadNav, point: Vec3) {
    let from = travel_profile(net, nav.segment, nav.node_ind, false);
    let to = travel_profile(net, nav.segment, nav.node_ind, true);
    if from.zones.is_empty() || to.zones.is_empty() {
        return;
    }
    let side = crate::right_of(nav.forward);
    let nearest = (0..from.zones.len().max(to.zones.len()))
        .filter(|&lane| {
            from.zones[lane.min(from.zones.len() - 1)].in_mask(nav.lane_type.selectable_mask())
                && to.zones[lane.min(to.zones.len() - 1)].in_mask(nav.lane_type.selectable_mask())
        })
        .min_by(|&a, &b| {
            let gap = |lane| {
                let at = lane_line(net, nav.segment, nav.node_ind, lane).position(nav.t);
                (point - at).dot(side).abs()
            };
            gap(a).total_cmp(&gap(b))
        });
    if let Some(lane) = nearest {
        nav.place(net, nav.segment, nav.node_ind, lane, nav.t);
    }
}

pub(super) fn advance(nav: &mut RoadNav, net: &RoadNetwork, distance: f32, direction: Vec3) {
    // Direction navigation is deterministic and never reads this trait argument;
    // do not consume any caller's random stream for a shared target observation.
    nav.advance(net, distance, direction, &mut SplitMix(0));
}
