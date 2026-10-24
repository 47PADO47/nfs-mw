//! Arrest staging on the queried road cross-section. Spec: `docs/specs/pursuit-cinematic-control.md`.

use blackbox_roads::{Body, LaneType, NavKind, PathType, RoadNav, SegmentFilter, closest_segment};
use glam::Vec3;

use super::TrafficWorld;
use crate::scenes::world::moment::heading_angle;
use crate::scenes::world::scene_space::Anchor;
use crate::scenes::world::space;

impl TrafficWorld {
    /// The centre of the drivable cross-section at the player's nearest road point, keeping the player's facing.
    pub(in crate::scenes::world) fn arrest_anchor(&self, player: Body) -> Option<Anchor> {
        let forward = Vec3::new(player.forward.x, 0.0, player.forward.y);
        let filter = SegmentFilter::default();
        // The original arrest query has no direction weight; it is not a heading-biased spawn query.
        let hit = closest_segment(&self.network, &self.index, player.position, forward, 0.0, filter)?;
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, filter);
        nav.path_type = PathType::Cop;
        nav.init_at_segment(&self.network, hit.segment, hit.t, player.position, forward, true);
        let bounds = nav.road_cross_section(&self.network);
        let centre = (bounds.left + bounds.right) * 0.5;
        let position = space::to_render([centre.x, bounds.centre.y, centre.y]);
        let facing = space::to_render(forward.to_array());
        Some(Anchor::facing(position, heading_angle(facing)))
    }
}
