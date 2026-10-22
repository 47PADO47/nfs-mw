//! Where a roadblock goes: 250 m along the road ahead of the target, on a plain segment that is long enough and has a
//! lane profile. The checks are the original's (`CreateRoadBlock`, spec section 2.1, read in the PC binary too); the
//! occupancy and on-road checks that follow in `build.rs` are the rewrite's.

use blackbox_pursuit::roadblock::Road;
use blackbox_roads::{LaneType, NavKind, RoadNav, SegmentFilter, flags};
use glam::{Vec2, Vec3};

use crate::scenes::world::ai::TrafficWorld;

/// How far ahead of the target the roadblock stands, metres (`kRoadBlockAheadDistance`).
pub const AHEAD: f32 = 250.0;
/// The shortest segment a roadblock may stand on, metres.
pub const MIN_SEGMENT: f32 = 40.0;
/// The site must be at least this far from the target, metres. A nav that could not advance (a dead end, a loop) lands
/// near the target; the original has no such check, and a roadblock on top of the player is never wanted.
const MIN_DISTANCE: f32 = 100.0;

/// A site for a roadblock.
#[derive(Debug, Clone, Copy)]
pub struct Site {
    /// The road under the nav, physics space.
    pub centre: Vec3,
    /// The road's direction there (unit, horizontal).
    pub forward: Vec3,
    /// The edges, the target and the direction, as the layout placer reads them.
    pub road: Road,
    /// The road's width plus one metre (`cr_width`).
    pub cr_width: f32,
}

impl TrafficWorld {
    /// Finds the site ahead of a target at `target` heading `heading` (physics space). `Err` names the check that
    /// refused it; the request stays latched and tries again on the next run, from where the target is then.
    pub(super) fn roadblock_site(&mut self, target: Vec3, heading: Vec3) -> Result<Site, String> {
        let mut nav = RoadNav::new(
            NavKind::Direction,
            LaneType::Racing,
            SegmentFilter { cop: true, no_decision: true, ..SegmentFilter::default() },
        );
        if !nav.init_at_point(&self.network, &self.index, target, heading, false) {
            return Err("there is no cop road under the target".into());
        }
        nav.half_width = 0.0;
        nav.advance(&self.network, AHEAD, heading, &mut self.rng.roadblock);
        if nav.dead_end || nav.position.distance(target) < MIN_DISTANCE {
            return Err("the road ends before a roadblock site".into());
        }
        let segment = self.network.segment(nav.segment);
        if segment.is_decision() {
            return Err("the site is on a junction".into());
        }
        if segment.has(flags::NO_TRAFFIC) {
            return Err("the road there takes no traffic".into());
        }
        if segment.length < MIN_SEGMENT {
            return Err("the segment is shorter than 40 m".into());
        }
        let node = segment.nodes[nav.node_ind.min(1)];
        if self.network.profile_at(nav.segment, node).zones.is_empty() {
            return Err("the road has no lane profile".into());
        }
        let forward = nav.forward.with_y(0.0).normalize_or_zero();
        let section = nav.road_cross_section(&self.network);
        let centre = nav.position;
        let lift = |point: Vec2| Vec3::new(point.x, centre.y, point.y);
        let road = Road { left: lift(section.left), right: lift(section.right), target, forward };
        let cr_width = section.left.distance(section.right) + 1.0;
        Ok(Site { centre, forward, road, cr_width })
    }
}
