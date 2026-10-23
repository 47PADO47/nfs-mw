//! A racer on its route (`blackbox_roads::Route`): which road its navigator follows when it searches again. Instead of
//! the shortest road to the next gate, the racer follows the race's route (with the shortcuts it rolled for), so it
//! drives the white line through the barriers and over the bridges the route takes.
//! Spec: `docs/specs/ai-racers-route.md` section 2.

use blackbox_roads::{PathType, RoadNav, RoadNetwork, Route, SegmentIndex};
use glam::Vec3;

/// How far from the route, on the ground and up or down, a racer may be and still join it where it is.
const JOIN_GAP: f32 = 12.0;
const JOIN_RISE: f32 = 3.0;
/// How far along the route past its nearest point a racer off the route aims to join it again.
const REJOIN_AHEAD: f32 = 80.0;

/// How a racer took up its route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Follow {
    /// Its navigator was on the route already: it follows on from there.
    Window,
    /// It was near the route: its navigator is put on the nearest point and follows on.
    Located,
    /// It was off the route: a path to a point of the route ahead of it.
    Rejoin,
}

/// Points `nav` along `route` for a racer on leg `leg` whose car is at `position`. `None` when the leg has no road
/// (the caller searches its way to the gate, as before).
pub fn follow_route(
    (net, index): (&RoadNetwork, &SegmentIndex),
    route: &Route,
    leg: usize,
    nav: &mut RoadNav,
    position: Vec3,
) -> Option<Follow> {
    let current = route.legs.get(leg).filter(|l| !l.missing())?;
    let window = route.window(leg);
    let goal = route.window_goal(leg);
    if let Some(at) = window.iter().position(|&s| s == nav.segment) {
        nav.follow(net, window[at..].to_vec(), goal, PathType::RaceRoute);
        return Some(Follow::Window);
    }
    let nearest = route.locate(net, position, leg..leg + 2);
    if let Some(point) = nearest.filter(|p| p.gap <= JOIN_GAP && p.rise.abs() <= JOIN_RISE) {
        let step = route.steps[point.step];
        let along = (point.distance - step.start) / net.segment(step.segment).length.max(1e-3) + step.from_t;
        let stored = if step.node_ind == 1 { along } else { 1.0 - along };
        let trail = nav.trail().is_some();
        nav.init_at_segment(net, step.segment, stored.clamp(0.0, 1.0), position, point.forward, false);
        if trail {
            nav.enable_trail(net);
        }
        let at = window.iter().position(|&s| s == step.segment).unwrap_or(0);
        nav.follow(net, window[at..].to_vec(), goal, PathType::RaceRoute);
        return Some(Follow::Located);
    }
    let from = nearest.map_or(current.start, |p| p.distance);
    let (aim, forward) = route.point_at(net, from + REJOIN_AHEAD)?;
    nav.find_path_to(net, index, aim, Some(forward), PathType::Racer);
    Some(Follow::Rejoin)
}

#[cfg(test)]
mod tests {
    use blackbox_roads::{LaneType, NavKind, RouteBuilder, SegmentFilter};

    use super::*;
    use crate::scenes::world::race::test_roads::{goal, network, route};

    fn nav_on(net: &RoadNetwork, segment: u16, t: f32, heading: Vec3) -> RoadNav {
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
        let at = blackbox_roads::centre_line(net, segment, 1).position(t);
        nav.init_at_segment(net, segment, t, at, heading, false);
        nav
    }

    #[test]
    fn a_navigator_on_the_route_follows_the_window_from_its_segment() {
        let net = network();
        let index = SegmentIndex::build(&net);
        let route = route(&net, &index);
        let mut nav = nav_on(&net, 0, 0.5, Vec3::Z);
        let follow = follow_route((&net, &index), &route, 0, &mut nav, Vec3::new(1.0, 0.0, 40.0));
        assert_eq!(follow, Some(Follow::Window));
        assert_eq!(nav.path, vec![0, 1]);
        assert_eq!(nav.path_type, PathType::RaceRoute);
        let mut later = nav_on(&net, 1, 0.6, Vec3::Z);
        follow_route((&net, &index), &route, 1, &mut later, Vec3::new(1.0, 0.0, 160.0));
        assert_eq!(later.path, vec![1], "the last leg's window");
    }

    #[test]
    fn a_navigator_turned_the_wrong_way_on_the_route_turns_round() {
        let net = network();
        let index = SegmentIndex::build(&net);
        let route = route(&net, &index);
        let mut nav = nav_on(&net, 0, 0.5, Vec3::NEG_Z);
        assert_eq!(nav.node_ind, 0);
        follow_route((&net, &index), &route, 0, &mut nav, Vec3::new(1.0, 0.0, 50.0));
        assert_eq!(nav.node_ind, 1, "it heads along the route again");
    }

    #[test]
    fn a_racer_beside_the_route_joins_it_where_it_is() {
        let net = network();
        let index = SegmentIndex::build(&net);
        let route = route(&net, &index);
        // The navigator looks down the side street; the car is still on the main road, 8 m beside its centre.
        let mut nav = nav_on(&net, 2, 0.1, Vec3::X);
        let follow = follow_route((&net, &index), &route, 0, &mut nav, Vec3::new(8.0, 0.0, 90.0));
        assert_eq!(follow, Some(Follow::Located));
        assert_eq!((nav.segment, nav.node_ind), (0, 1));
        assert_eq!(nav.path, vec![0, 1]);
    }

    #[test]
    fn a_racer_far_from_the_route_finds_its_way_back_ahead() {
        let net = network();
        let index = SegmentIndex::build(&net);
        let route = route(&net, &index);
        let mut nav = nav_on(&net, 2, 0.6, Vec3::X);
        let follow = follow_route((&net, &index), &route, 0, &mut nav, Vec3::new(60.0, 0.0, 100.0));
        assert_eq!(follow, Some(Follow::Rejoin));
        assert_eq!(nav.path.last(), Some(&1), "a path back to the route ahead");
        assert_eq!(nav.path_type, PathType::Racer);
    }

    #[test]
    fn a_missing_leg_leaves_the_search_to_the_caller() {
        let net = network();
        let index = SegmentIndex::build(&net);
        let mut builder = RouteBuilder::new(&net);
        builder.push(goal(5.0), goal(150.0), None);
        let route = builder.finish();
        let mut nav = nav_on(&net, 0, 0.5, Vec3::Z);
        assert_eq!(follow_route((&net, &index), &route, 0, &mut nav, Vec3::ZERO), None);
        assert_eq!(follow_route((&net, &index), &route, 5, &mut nav, Vec3::ZERO), None, "no such leg");
    }
}
