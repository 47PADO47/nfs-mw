//! A shortcut must use the same active-wall context as its base route.

use blackbox_roads::{
    ActiveBarrier, Goal, PathType, RoadNetwork, RoadNode, RoadSegment, Route, RouteBuilder, build_leg, to_zone_space,
};
use glam::Vec3;

use super::TrafficWorld;
use crate::scenes::world::race::test_roads::{goal, network};

fn course() -> (TrafficWorld, Route, Goal) {
    let mut net = network();
    net.nodes[3].position.x = 20.0;
    net.nodes[3].segments.push(3);
    net.nodes[2].segments.extend([4, 5]);
    net.nodes.push(RoadNode { position: Vec3::new(20.0, 0.0, 150.0), profile: 0, segments: vec![3, 4] });
    net.nodes.push(RoadNode { position: Vec3::new(0.0, 0.0, 300.0), profile: 0, segments: vec![5] });
    net.roads.push(net.roads[0].clone());
    net.segments[2].length = 20.0;
    net.segments[2].road = Some(1);
    let segment = |nodes, length, road| RoadSegment {
        nodes,
        length,
        road: Some(road),
        flags: 0,
        start_handle: Vec3::ZERO,
        end_handle: Vec3::ZERO,
    };
    net.segments.extend([segment([3, 4], 50.0, 1), segment([4, 2], 54.0, 1), segment([2, 5], 100.0, 0)]);
    let traffic = TrafficWorld::new(net);
    let from = goal(5.0);
    let to = goal(250.0);
    let path = build_leg(&traffic.network, &traffic.index, from, to, PathType::RaceRoute).unwrap();
    assert_eq!(path.segments, vec![0, 1, 5]);
    let mut builder = RouteBuilder::new(&traffic.network);
    builder.push(from, to, Some(&path));
    let route = builder.finish();
    (traffic, route, Goal { position: Vec3::new(20.0, 0.0, 140.0), forward: Vec3::Z })
}

fn wall(z: f32) -> ActiveBarrier {
    ActiveBarrier {
        a: to_zone_space(Vec3::new(10.0, 0.0, z)),
        b: to_zone_space(Vec3::new(30.0, 0.0, z)),
        drive_through: false,
    }
}

fn flags(net: &RoadNetwork) -> Vec<u16> {
    net.segments.iter().map(|segment| segment.flags).collect()
}

#[test]
fn an_active_wall_closes_the_shortcut_without_changing_the_shared_road_flags() {
    let (traffic, route, via) = course();
    let before = flags(&traffic.network);
    let (_, open) = traffic.route_through(&route, via, &[]).expect("the open side road is a usable shortcut");
    assert!(open.segments.contains(&3));
    assert!(traffic.route_through(&route, via, &[wall(125.0)]).is_none(), "the marker is beyond a closed barrier");
    assert_eq!(flags(&traffic.network), before, "the police/free-roam graph keeps its previous flags");
}

#[test]
fn an_unrelated_wall_does_not_disable_an_open_shortcut() {
    let (traffic, route, via) = course();
    assert!(traffic.route_through(&route, via, &[wall(500.0)]).is_some());
}
