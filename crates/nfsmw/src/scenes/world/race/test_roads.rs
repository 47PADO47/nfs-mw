//! A small road network and a route on it, for the tests of the racers' route and the reset.

use blackbox_roads::{
    Goal, PathType, Road, RoadNetwork, RoadNode, RoadProfile, RoadSegment, Route, RouteBuilder, SegmentIndex, Zone,
    build_leg, zone,
};
use glam::Vec3;

fn lane(kind: u8, offset: f32) -> Zone {
    Zone { kind, width: 4.0, offset }
}

/// A road along +z from 0 to 200 m in two segments, and a side street from the middle node to +x (physics space).
pub fn network() -> RoadNetwork {
    let profile = RoadProfile {
        middle: 2,
        zones: vec![
            lane(zone::TRAFFIC, 6.0),
            lane(zone::TRAFFIC, 2.0),
            lane(zone::TRAFFIC, 2.0),
            lane(zone::TRAFFIC, 6.0),
        ],
    };
    let node = |x: f32, z: f32, segments: Vec<u16>| RoadNode { position: Vec3::new(x, 0.0, z), profile: 0, segments };
    let segment = |nodes: [u16; 2], length: f32| RoadSegment {
        nodes,
        length,
        road: Some(0),
        flags: 0,
        start_handle: Vec3::ZERO,
        end_handle: Vec3::ZERO,
    };
    RoadNetwork {
        nodes: vec![
            node(0.0, 0.0, vec![0]),
            node(0.0, 100.0, vec![0, 1, 2]),
            node(0.0, 200.0, vec![1]),
            node(100.0, 100.0, vec![2]),
        ],
        segments: vec![segment([0, 1], 100.0), segment([1, 2], 100.0), segment([1, 3], 100.0)],
        profiles: vec![profile],
        roads: vec![Road { scale: 1.0, speech_id: 0 }],
    }
}

/// A goal on the main road at `z`, crossed along +z.
pub fn goal(z: f32) -> Goal {
    Goal { position: Vec3::new(1.0, 0.0, z), forward: Vec3::Z }
}

/// The route from z = 5 through goals at z = 150 and z = 190.
pub fn route(net: &RoadNetwork, index: &SegmentIndex) -> Route {
    let mut builder = RouteBuilder::new(net);
    let mut from = goal(5.0);
    for to in [goal(150.0), goal(190.0)] {
        builder.push(from, to, build_leg(net, index, from, to, PathType::RaceRoute).as_ref());
        from = to;
    }
    builder.finish()
}
