use glam::Vec3;

use super::synth::*;
use crate::{Goal, PathType, RouteBuilder, SegmentIndex, build_leg};

fn goal(z: f32) -> Goal {
    Goal { position: Vec3::new(1.0, 0.0, z), forward: Vec3::Z }
}

/// The chain road (two 100 m segments along +z) from z = 10 through goals at z = 150 and z = 190.
fn chain_route() -> (crate::RoadNetwork, crate::Route) {
    let net = chain_road();
    let index = SegmentIndex::build(&net);
    let mut builder = RouteBuilder::new(&net);
    let mut from = goal(10.0);
    for to in [goal(150.0), goal(190.0)] {
        let path = build_leg(&net, &index, from, to, PathType::RaceRoute).expect("a road");
        builder.push(from, to, Some(&path));
        from = to;
    }
    let route = builder.finish();
    (net, route)
}

#[test]
fn the_legs_follow_the_goals_in_order() {
    let (net, route) = chain_route();
    assert_eq!(route.legs.len(), 2);
    let segments: Vec<u16> = route.steps.iter().map(|s| s.segment).collect();
    assert_eq!(segments, vec![0, 1, 1], "start to the first goal, then on to the second");
    assert!(route.steps.iter().all(|s| s.node_ind == 1), "all along the stored direction");
    assert!((route.legs[0].length - 140.0).abs() < 1.0, "{}", route.legs[0].length);
    assert!((route.legs[1].length - 40.0).abs() < 1.0, "{}", route.legs[1].length);
    assert!((route.length - 180.0).abs() < 1.0);
    let (at, forward) = route.point_at(&net, 140.0).expect("a point");
    assert!((at.z - 150.0).abs() < 1.0 && forward.z > 0.99, "{at:?}");
    assert_eq!(route.leg_at(139.0), 0);
    assert_eq!(route.leg_at(141.0), 1);
    assert_eq!(route.window(0), vec![0, 1]);
    assert_eq!(route.window(1), vec![1]);
    assert_eq!(route.window_goal(0), route.legs[1].goal);
}

#[test]
fn a_position_is_located_along_the_route() {
    let (net, route) = chain_route();
    let found = route.locate(&net, Vec3::new(3.0, 0.5, 120.0), 0..2).expect("on the route");
    assert_eq!((found.leg, route.steps[found.step].segment), (0, 1));
    assert!((found.distance - 110.0).abs() < 1.0, "{}", found.distance);
    assert!((found.gap - 3.0).abs() < 0.1 && (found.rise - 0.5).abs() < 0.1);
    let later = route.locate(&net, Vec3::new(0.0, 0.0, 170.0), 1..2).expect("on the second leg");
    assert_eq!(later.leg, 1);
    assert!((later.distance - 160.0).abs() < 1.0, "{}", later.distance);
}

#[test]
fn a_route_under_a_bridge_is_not_located_on_the_bridge() {
    let net = stacked_roads();
    let index = SegmentIndex::build(&net);
    let (from, to) = (goal(10.0), goal(90.0));
    let path = build_leg(&net, &index, from, to, PathType::RaceRoute).expect("the lower road");
    assert_eq!(path.segments, vec![0], "the leg keeps to the road its goals are on");
    let mut builder = RouteBuilder::new(&net);
    builder.push(from, to, Some(&path));
    let route = builder.finish();
    let below = route.locate(&net, Vec3::new(1.0, 0.4, 50.0), 0..1).expect("on the road");
    assert!(below.rise.abs() < 0.5);
    let above = route.locate(&net, Vec3::new(1.0, 10.4, 50.0), 0..1).expect("still the nearest piece");
    assert!(above.rise > 9.0, "the bridge is 10 m over the route: {}", above.rise);
}

#[test]
fn a_missing_leg_is_a_straight_line_with_no_road() {
    let net = chain_road();
    let mut builder = RouteBuilder::new(&net);
    builder.push(goal(0.0), goal(5000.0), None);
    let route = builder.finish();
    assert!(route.legs[0].missing());
    assert_eq!(route.legs[0].goal, None);
    assert!((route.length - 5000.0).abs() < 1e-3);
    let (at, _) = route.point_at(&net, 2500.0).expect("a point");
    assert!((at.z - 2500.0).abs() < 1e-3);
    assert!(route.window(0).is_empty());
    assert!(route.locate(&net, Vec3::ZERO, 0..1).is_none());
}

#[test]
fn a_goal_off_the_map_has_no_leg() {
    let net = chain_road();
    let index = SegmentIndex::build(&net);
    assert!(build_leg(&net, &index, goal(10.0), goal(5000.0), PathType::RaceRoute).is_none());
}
