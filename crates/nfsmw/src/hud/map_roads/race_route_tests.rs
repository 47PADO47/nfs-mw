//! Tests for the race's route on the map: one lap of a hand-made route, closed or open, its missing legs, and the split
//! where the player is.

use blackbox_roads::{Goal, Leg, RoadNetwork, RoadNode, RoadProfile, RoadSegment, Route, RouteStep};
use glam::Vec3;

use super::{RaceRoute, RaceRouteHandle};

fn to_render(p: Vec3) -> [f32; 2] {
    [p.z, -p.x]
}

/// Two straight 100 m segments: north along +z, then east along +x (physics space).
fn network() -> RoadNetwork {
    let node = |x: f32, z: f32| RoadNode { position: Vec3::new(x, 0.0, z), profile: 0, segments: Vec::new() };
    let segment = |nodes: [u16; 2]| RoadSegment {
        nodes,
        length: 100.0,
        road: None,
        flags: 0,
        start_handle: Vec3::ZERO,
        end_handle: Vec3::ZERO,
    };
    RoadNetwork {
        nodes: vec![node(0.0, 0.0), node(0.0, 100.0), node(100.0, 100.0)],
        segments: vec![segment([0, 1]), segment([1, 2])],
        profiles: vec![RoadProfile { middle: 0, zones: Vec::new() }],
        roads: Vec::new(),
    }
}

fn goal(x: f32, z: f32) -> Goal {
    Goal { position: Vec3::new(x, 0.0, z), forward: Vec3::Z }
}

fn step(segment: u16, start: f32, leg: usize) -> RouteStep {
    RouteStep { segment, node_ind: 1, from_t: 0.0, to_t: 1.0, start, leg }
}

/// The two legs of one lap, driven twice: 400 m in all.
fn two_laps() -> Route {
    Route {
        steps: vec![step(0, 0.0, 0), step(1, 100.0, 1)],
        legs: vec![
            Leg { from: goal(0.0, 0.0), to: goal(0.0, 100.0), steps: 0..1, start: 0.0, length: 100.0, goal: None },
            Leg {
                from: goal(0.0, 100.0),
                to: goal(100.0, 100.0),
                steps: 1..2,
                start: 100.0,
                length: 100.0,
                goal: None,
            },
        ],
        length: 400.0,
    }
}

#[test]
fn one_lap_of_the_route_is_drawn_open_or_closed() {
    let open = RaceRoute::build(&network(), &two_laps(), (2, false), to_render).unwrap();
    assert_eq!(open.points(), [[0.0, 0.0], [100.0, 0.0], [100.0, -100.0]], "the corner is kept, the repeat is not");
    let closed = RaceRoute::build(&network(), &two_laps(), (2, true), to_render).unwrap();
    assert_eq!(closed.points().len(), 4);
    assert_eq!(closed.points()[3], closed.points()[0], "a lap race runs on to its start");
}

#[test]
fn a_missing_leg_is_the_straight_line_between_its_goals() {
    let route = Route {
        steps: Vec::new(),
        legs: vec![Leg {
            from: goal(0.0, 0.0),
            to: goal(0.0, 50.0),
            steps: 0..0,
            start: 0.0,
            length: 50.0,
            goal: None,
        }],
        length: 50.0,
    };
    let line = RaceRoute::build(&network(), &route, (1, false), to_render).unwrap();
    assert_eq!(line.points(), [[0.0, 0.0], [50.0, 0.0]]);
    assert_eq!(RaceRoute::build(&network(), &Route::default(), (1, false), to_render), None, "no leg, no line");
}

#[test]
fn the_player_is_placed_on_the_lap_they_drive() {
    let route = RaceRoute::build(&network(), &two_laps(), (2, false), to_render).unwrap();
    assert_eq!(route.along(300.0), Some(100.0), "first lap, 100 m in");
    assert_eq!(route.along(100.0), Some(100.0), "second lap, 100 m in");
    assert_eq!(route.along(0.0), Some(200.0), "finished: all driven");
    assert_eq!(route.along(f32::NAN), None);
}

#[test]
fn the_route_splits_where_the_player_is() {
    let route = RaceRoute::build(&network(), &two_laps(), (2, false), to_render).unwrap();
    let (driven, ahead) = route.split(150.0);
    assert_eq!(driven, vec![[0.0, 0.0], [100.0, 0.0], [100.0, -50.0]]);
    assert_eq!(ahead, vec![[100.0, -50.0], [100.0, -100.0]]);
    assert!(route.split(-1.0).0.is_empty(), "before the start all is ahead");
    assert!(route.split(500.0).1.is_empty(), "past the end all is driven");
}

#[test]
fn race_route_handles_are_equal_only_when_shared() {
    let route = RaceRoute::build(&network(), &two_laps(), (2, false), to_render).unwrap();
    let a = RaceRouteHandle::new(route.clone());
    assert_eq!(a, a.clone());
    assert_ne!(a, RaceRouteHandle::new(route));
}
