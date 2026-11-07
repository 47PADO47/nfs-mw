//! Non-pursuit off-path recovery: `docs/specs/ai-driver-speed-skill.md`, section 1.

use blackbox_roads::{Body, NavKind, RoadNav, RoadNetwork, SegmentIndex, update_occluded_position};
use glam::Vec3;

/// A closer road outside the old path and trail, only after the car has left the old corridor.
pub(super) fn candidate(net: &RoadNetwork, index: &SegmentIndex, nav: &RoadNav, car: &Body) -> Option<RoadNav> {
    let trail = nav.trail()?;
    let current =
        update_occluded_position(trail, car, nav.position, nav.forward, [] as [Body; 0], false, nav.half_width)?;
    if current.out_of_bounds <= 2.0 {
        return None;
    }
    let mut fresh = RoadNav::new(NavKind::Direction, nav.lane_type, nav.filter);
    fresh.half_width = nav.half_width;
    let heading = Vec3::new(car.forward.x, 0.0, car.forward.y);
    if !fresh.init_at_point(net, index, car.position, heading, false)
        || nav.path.contains(&fresh.segment)
        || trail.cookies().iter().any(|cookie| cookie.segment == fresh.segment)
        || fresh.position.distance_squared(car.position) >= current.current.centre.distance_squared(car.position)
    {
        return None;
    }
    fresh.enable_trail(net);
    Some(fresh)
}

#[cfg(test)]
mod tests {
    use blackbox_roads::{LaneType, PathType, SegmentFilter, SplitMix};
    use glam::Vec2;

    use super::*;
    use crate::scenes::world::race::test_roads::network;

    fn car(position: Vec3) -> Body {
        Body { position, velocity: Vec3::Z * 10.0, forward: Vec2::Y, half_width: 1.0, half_length: 2.0 }
    }

    fn road(net: &RoadNetwork) -> RoadNav {
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
        nav.init_at_segment(net, 0, 0.5, Vec3::new(1.0, 0.0, 50.0), Vec3::Z, false);
        nav.enable_trail(net);
        nav.follow(net, vec![0, 1], None, PathType::RaceRoute);
        nav.advance(net, 20.0, Vec3::Z, &mut SplitMix(1));
        nav
    }

    #[test]
    fn a_car_on_the_side_street_reacquires_its_closer_road() {
        let net = network();
        let index = SegmentIndex::build(&net);
        let nav = road(&net);
        let body = car(Vec3::new(60.0, 0.0, 100.0));
        let fresh = candidate(&net, &index, &nav, &body).expect("the car left the main-road corridor");
        assert_eq!(fresh.segment, 2);
        assert!(fresh.position.distance(body.position) < 6.0);
        assert!(fresh.trail().is_some());
    }

    #[test]
    fn leaving_the_centre_line_does_not_replace_a_segment_already_on_the_path() {
        let net = network();
        let index = SegmentIndex::build(&net);
        let nav = road(&net);
        assert!(candidate(&net, &index, &nav, &car(Vec3::new(20.0, 0.0, 60.0))).is_none());
    }

    #[test]
    fn a_recent_trail_segment_is_not_reacquired_even_after_the_path_moves_on() {
        let net = network();
        let index = SegmentIndex::build(&net);
        let mut nav = road(&net);
        nav.advance(&net, 70.0, Vec3::Z, &mut SplitMix(1));
        nav.path = vec![1];
        assert!(nav.trail().unwrap().cookies().iter().any(|cookie| cookie.segment == 0));
        assert!(candidate(&net, &index, &nav, &car(Vec3::new(12.0, 0.0, 80.0))).is_none());
    }
}
