//! Retained road spots with continuous lateral offsets.

use glam::Vec3;

use crate::{LaneType, NavKind, PathType, RoadNav, RoadNetwork, SegmentFilter, SplitMix};

#[derive(Debug, Clone, Copy)]
pub(super) struct Spot {
    pub segment: u16,
    pub node: usize,
    pub t: f32,
    pub offset: f32,
}

impl Spot {
    pub fn of(nav: &RoadNav) -> Self {
        Self { segment: nav.segment, node: nav.node_ind, t: nav.t, offset: nav.continuous_offset().unwrap_or(0.0) }
    }

    pub fn nav(self, net: &RoadNetwork, point: Option<Vec3>) -> RoadNav {
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Cop, SegmentFilter::default());
        nav.path_type = PathType::Cop;
        nav.place(net, self.segment, self.node, 0, self.t);
        nav.set_lateral_offset(net, self.offset);
        if let Some(point) = point {
            snap(net, &mut nav, point);
        }
        nav
    }
}

pub(super) fn snap(net: &RoadNetwork, nav: &mut RoadNav, point: Vec3) {
    let _ = nav.snap_offset_at(net, point);
}

pub(super) fn advance(nav: &mut RoadNav, net: &RoadNetwork, distance: f32, direction: Vec3) {
    // Direction navigation is deterministic and never reads this trait argument;
    // do not consume any caller's random stream for a shared target observation.
    nav.advance(net, distance, direction, &mut SplitMix(0));
}
