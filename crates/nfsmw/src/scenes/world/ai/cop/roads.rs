//! Adapters from road and collision queries into the close pursuit rules.

use blackbox_collision::{CollisionWorld, RayOptions};
use blackbox_roads::{Body, LaneType, NavKind, RandomSource, RoadNav, SegmentFilter};
use glam::{Vec2, Vec3};

use super::{Target, close};
use crate::scenes::world::ai::car::Ctx;

#[cfg(test)]
mod wall_tests;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::scenes::world::ai) struct TargetRoad {
    pub gap: f32,
    pub direction: Vec2,
    pub centre: Vec3,
    pub seek: Vec3,
}

impl TargetRoad {
    pub(in crate::scenes::world::ai) fn hash_into(self, hash: &mut crate::scenes::world::StateHasher) {
        hash.f32(self.gap);
        hash.f32(self.direction.x);
        hash.f32(self.direction.y);
        hash.vec3(self.centre);
        hash.vec3(self.seek);
    }
}

pub(super) fn clear(collision: &CollisionWorld, start: Vec3, end: Vec3) -> bool {
    if start.distance_squared(end) > 200.0_f32.powi(2) {
        return false;
    }
    let end = end + Vec3::Y * 0.5;
    let hit = collision.ray_cast((start + Vec3::Y * 0.5).to_array(), end.to_array(), &RayOptions::default());
    hit.is_none_or(|hit| Vec3::from_array(hit.point).distance_squared(end) <= 0.5)
}

pub(super) fn at_car<R>(ctx: &Ctx<'_, R>, body: &Body, level: u32) -> Option<RoadNav> {
    road_at(ctx, body, level, true)
}

fn road_at<R>(ctx: &Ctx<'_, R>, body: &Body, level: u32, cop: bool) -> Option<RoadNav> {
    let lane = match level >= 3 {
        true => LaneType::CopReckless,
        false => LaneType::Cop,
    };
    let mut nav = RoadNav::new(NavKind::Direction, lane, SegmentFilter { cop, ..Default::default() });
    nav.half_width = body.half_width;
    let heading = close::lift(body.forward, 0.0);
    nav.init_at_point(ctx.net, ctx.index, body.position, heading, false).then_some(nav)
}

pub(super) fn target<R: RandomSource>(
    ctx: &mut Ctx<'_, R>,
    target: &Target,
    level: u32,
    jerk: bool,
) -> Option<TargetRoad> {
    let speed = target.body.velocity.length();
    let heading = match speed >= 1.0 {
        true => close::xz(target.body.velocity).try_normalize().unwrap_or(target.body.forward),
        false => target.body.forward,
    };
    let body = Body { forward: heading, ..target.body };
    let mut road = road_at(ctx, &body, level, false)?;
    let gap = target.body.position.distance(road.position);
    let current = road.position;
    let direction = close::lift(heading, 0.0);
    road.advance(ctx.net, speed.max(2.0), direction, ctx.rng);
    let travel = close::xz(road.position - current).try_normalize().unwrap_or(target.body.forward);
    let mut seek_nav = road_at(ctx, &body, level, false)?;
    let horizon = match jerk {
        true => 3.12,
        false => 7.8,
    };
    seek_nav.advance(ctx.net, (speed * horizon + 8.0).min(500.0), direction, ctx.rng);
    Some(TargetRoad { gap, direction: travel, centre: current, seek: seek_nav.position })
}

/// Walk the persistent drive-to cursor towards the predicted slot in five-metre increments.
/// A slot outside the lane-snap tolerance keeps its free-form steering and resets the cursor.
pub(super) fn snap<R: RandomSource>(
    ctx: &mut Ctx<'_, R>,
    nav: &mut RoadNav,
    me: &Body,
    point: Vec3,
    direction: Vec2,
    traffic: &[Body],
    level: u32,
) -> close::RoadSteer {
    nav.kind = NavKind::Direction;
    nav.half_width = me.half_width;
    nav.lane_type = match level >= 3 {
        true => LaneType::CopReckless,
        false => LaneType::Cop,
    };
    let toward = close::lift(direction, 0.0);
    let mut along = (point - nav.position).dot(nav.forward);
    if along >= -5.0 {
        // The guard only bounds malformed road graphs, whose cursor never progresses.
        for _ in 0..100 {
            if along <= 5.0 {
                break;
            }
            let previous = nav.position;
            nav.advance(ctx.net, 5.0, toward, ctx.rng);
            if nav.position.distance_squared(previous) < 0.0001 {
                break;
            }
            along = (point - nav.position).dot(nav.forward);
        }
        let side = close::lift(Vec2::new(nav.forward.z, -nav.forward.x), 0.0);
        let profile = blackbox_roads::travel_profile(ctx.net, nav.segment, nav.node_ind, nav.t >= 0.5);
        let mask = nav.lane_type.selectable_mask();
        let lane = (0..profile.zones.len())
            .filter(|&lane| profile.zones[lane].in_mask(mask))
            .map(|lane| {
                let position = blackbox_roads::lane_line(ctx.net, nav.segment, nav.node_ind, lane).position(nav.t);
                (lane, (point - position).dot(side).abs())
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((lane, gap)) = lane
            && gap < 4.0
        {
            nav.place(ctx.net, nav.segment, nav.node_ind, lane, nav.t);
            nav.advance(ctx.net, along.max(0.1), toward, ctx.rng);
            let occlusion = nav.trail().and_then(|trail| {
                blackbox_roads::update_occluded_position(
                    trail,
                    me,
                    nav.position,
                    traffic.iter().copied(),
                    false,
                    nav.half_width,
                )
            });
            return close::RoadSteer {
                point: Some(occlusion.as_ref().map_or(nav.position, |o| o.position)),
                current: occlusion.map(|o| o.current),
            };
        }
    }
    if let Some(own) = at_car(ctx, me, level) {
        *nav = own;
        nav.enable_trail(ctx.net);
        return close::RoadSteer { point: None, current: Some(nav.cookie(ctx.net)) };
    }
    close::RoadSteer { point: None, current: None }
}

pub(super) fn wall(collision: &CollisionWorld, me: &Body) -> Vec2 {
    let speed = me.velocity.length();
    if speed < 2.0 {
        return Vec2::ZERO;
    }
    let length = (speed + 10.0).min(80.0);
    let start = me.position;
    let end = start + me.velocity / speed * length;
    let options = RayOptions { faces: false, ..Default::default() };
    let Some(hit) = collision.ray_cast(start.to_array(), end.to_array(), &options) else {
        return Vec2::ZERO;
    };
    close::wall(me.velocity, Vec3::from_array(hit.normal), hit.t * length)
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_roads::{Road, RoadNetwork, RoadNode, RoadProfile, RoadSegment, SegmentIndex, SplitMix, Zone, zone};

    fn wide_road() -> RoadNetwork {
        let node = |z| RoadNode { position: Vec3::new(0.0, 0.0, z), profile: 0, segments: vec![0] };
        RoadNetwork {
            nodes: vec![node(-100.0), node(100.0)],
            segments: vec![RoadSegment {
                nodes: [0, 1],
                length: 200.0,
                road: Some(0),
                flags: 0,
                start_handle: Vec3::ZERO,
                end_handle: Vec3::ZERO,
            }],
            profiles: vec![RoadProfile {
                middle: 0,
                zones: vec![Zone { kind: zone::TRAFFIC, width: 20.0, offset: 0.0 }],
            }],
            roads: vec![Road { scale: 1.0, speech_id: 0 }],
        }
    }

    #[test]
    fn close_snap_keeps_the_inset_current_cookie_near_the_car_and_preserves_width_on_reset() {
        let net = wide_road();
        let index = SegmentIndex::build(&net);
        let mut rng = SplitMix(4);
        let mut ctx =
            Ctx { net: &net, index: &index, rng: &mut rng, bodies: &[], me: 0, player: None, aims: &[], lights: None };
        let me = Body {
            position: Vec3::new(3.0, 0.0, 0.0),
            velocity: Vec3::new(2.0, 0.0, 10.0),
            forward: Vec2::Y,
            half_width: 1.7,
            half_length: 2.5,
        };
        let mut nav = at_car(&ctx, &me, 1).unwrap();
        assert_eq!(nav.half_width, me.half_width);
        nav.enable_trail(&net);
        let result = snap(&mut ctx, &mut nav, &me, Vec3::Z * 30.0, Vec2::Y, &[], 1);
        let current = result.current.expect("successful snap must update the current cookie");
        assert!(result.point.is_some());
        assert!(nav.position.z > 20.0, "seek cursor must advance well ahead");
        assert!(current.centre.z.abs() < 1.0, "affinity must stay near the physical car: {current:?}");
        assert!(current.width() < nav.road_cross_section(&net).width(), "affinity must use inset steering edges");
        assert!(close::affinity(me.position, me.velocity, &current).x < 0.0, "right-edge approach must repel left");
        let result = snap(&mut ctx, &mut nav, &me, Vec3::new(100.0, 0.0, 30.0), Vec2::Y, &[], 1);
        assert!(result.point.is_none(), "off-road seek must retain free-form steering");
        assert_eq!(nav.half_width, me.half_width, "reset must retain this physical vehicle's width");
        assert!(result.current.unwrap().centre.z.abs() < 1.0, "reset affinity must use the car's nearby corridor");
    }
}
