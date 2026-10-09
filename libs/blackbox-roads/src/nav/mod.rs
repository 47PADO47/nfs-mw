//! The road navigator: a cursor on the road graph that moves along a lane.
//! Spec: `docs/specs/ai-road-network.md` (§2–§4).

mod advance;
mod direction;
mod init;
mod lane_type;
mod next;
mod random;
mod traffic_lanes;

use glam::Vec3;

use crate::{Bezier, NodeInd, SegmentFilter};

pub use lane_type::LaneType;
pub use random::{RandomSource, SplitMix};
pub use traffic_lanes::{forward_traffic_lanes, nth_from_centre, pick_lane};

/// How the cursor chooses the segment after the one it is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavKind {
    /// Follow the lane graph like a driver; never turns around.
    Traffic,
    /// Go where a target vector points; turns around at dead ends.
    Direction,
}

/// A cursor on one lane of one segment, heading towards one of its nodes.
#[derive(Debug, Clone)]
pub struct RoadNav {
    pub kind: NavKind,
    pub lane_type: LaneType,
    pub filter: SegmentFilter,
    pub segment: u16,
    /// The end the cursor heads to: `1` is the stored direction.
    pub node_ind: NodeInd,
    /// Progress along the segment in the travel direction, 0…1.
    pub t: f32,
    /// Index of the lane in the travel-frame profile.
    pub lane: usize,
    /// Set when the cursor could not leave its segment (a traffic cursor stops there).
    pub dead_end: bool,
    /// Half the car's width in metres.
    pub half_width: f32,
    pub position: Vec3,
    /// Unit direction of travel at the cursor.
    pub forward: Vec3,
    /// Planar curvature at the cursor, positive when turning right.
    pub curvature: f32,
    line: Bezier,
}

impl RoadNav {
    /// The lane line the cursor is on.
    pub fn lane_line(&self) -> &Bezier {
        &self.line
    }
}
