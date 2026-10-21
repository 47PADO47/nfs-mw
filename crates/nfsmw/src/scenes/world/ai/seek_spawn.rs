//! Where the chase puts its next cop: a walk along the cop roads from the player's seek-ahead point, and a disc ahead of the
//! player when that fails. Spec: `docs/specs/ai-pursuit-cops.md` section 3.4, `docs/provenance/ai-pursuit-cop-fleet.md`.
//!
//! The original (`SpawnPursuitIVehicle`) takes the point its target will reach in about 7.8 s along the road (the seek-ahead
//! position), turns the vector from there back to the target by 60 degrees to a random side, and walks that far along the
//! cop roads from the seek-ahead point, steering at junctions by the turned vector. The spot must pass the spawn window
//! from the camera, the no-cop-spawn zones and the 40 m spacing; then the cop is turned to face the target. When no spot
//! comes of it, a point 200 m ahead of the target along its travel plus a random point of a disc of radius 190 m is snapped
//! to the cop roads and checked the same way.

use blackbox_pursuit::spawn_rules::{disc_point, fallback_centre, rotate_in_xz, seek_ahead_distance, side_angle};
use blackbox_roads::{Body, LaneType, NavKind, RandomSource, RoadNav, RoadNetwork, SegmentFilter};
use glam::Vec3;

use super::TrafficWorld;

/// Whether cops consider a segment of the road graph (`ShouldCopsConsider`).
fn cops_consider(net: &RoadNetwork, segment: u16) -> bool {
    SegmentFilter { cop: true, ..SegmentFilter::default() }.allows(net.segment(segment))
}

impl TrafficWorld {
    /// The points a chase cop may be put at, best first: the end of the walk from the seek-ahead point (when the roads give
    /// one) and the fallback point. The caller takes the first that passes its checks.
    pub(super) fn chase_spawn_points(&mut self, player: &Body) -> Vec<Vec3> {
        let mut points = Vec::with_capacity(2);
        points.extend(self.seek_walk_point(player));
        let forward = Vec3::new(player.forward.x, 0.0, player.forward.y);
        let centre = fallback_centre(player.position, player.velocity, forward);
        let (angle_roll, radius_roll) = (self.rng.police.next_f32(), self.rng.police.next_f32());
        points.push(disc_point(centre, angle_roll, radius_roll));
        points
    }

    /// The point the player will reach in a few seconds along the cop roads (`GetSeekAheadPosition`).
    fn seek_ahead_point(&mut self, player: &Body) -> Option<Vec3> {
        let forward = Vec3::new(player.forward.x, 0.0, player.forward.y);
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Cop, SegmentFilter::default());
        if !nav.init_at_point(&self.network, &self.index, player.position, forward, false) {
            return None;
        }
        nav.filter.cop = cops_consider(&self.network, nav.segment);
        let ahead = seek_ahead_distance(player.velocity.length());
        nav.advance(&self.network, ahead, Vec3::ZERO, &mut self.rng.police);
        Some(nav.position)
    }

    /// The end of the walk: from the seek-ahead point, as far as the target is from it, along the cop roads in the direction
    /// of the vector back to the target turned by 60 degrees to a random side.
    fn seek_walk_point(&mut self, player: &Body) -> Option<Vec3> {
        let seek = self.seek_ahead_point(player)?;
        let back = player.position - seek;
        let distance = back.length();
        let toward = rotate_in_xz(back, side_angle(self.rng.police.next_f32()));
        let mut nav =
            RoadNav::new(NavKind::Direction, LaneType::Cop, SegmentFilter { cop: true, ..SegmentFilter::default() });
        if !nav.init_at_point(&self.network, &self.index, seek, toward, false) {
            return None;
        }
        nav.filter.cop = cops_consider(&self.network, nav.segment);
        nav.advance(&self.network, distance, toward, &mut self.rng.police);
        Some(nav.position).filter(|point| point.is_finite())
    }
}
