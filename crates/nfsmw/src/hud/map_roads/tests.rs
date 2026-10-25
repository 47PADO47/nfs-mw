//! Tests for the map's roads: the simplification, the classes, the grid and the shared handle, on small hand-made
//! networks (physics space: x right, z forward), with no game data.

use blackbox_roads::{Bezier, RoadNetwork, RoadNode, RoadProfile, RoadSegment, Zone, flags, zone};
use glam::Vec3;

use super::classify::{MAIN_LANES, class_of};
use super::grid::{CELL_M, RoadGrid};
use super::simplify::{distance_to_segment, douglas_peucker, polyline};
use super::{MapRoad, MapRoads, MapRoadsHandle, RoadClass, TOLERANCE_M};

/// Physics to render (x, y), as `scenes/world/space.rs` converts.
fn to_render(p: Vec3) -> [f32; 2] {
    [p.z, -p.x]
}

fn profile(kinds: &[u8]) -> RoadProfile {
    let zones = kinds.iter().map(|&kind| Zone { kind, width: 4.0, offset: 2.0 }).collect();
    RoadProfile { middle: (kinds.len() / 2) as u8, zones }
}

fn node(x: f32, z: f32, profile: u16) -> RoadNode {
    RoadNode { position: Vec3::new(x, 0.0, z), profile, segments: Vec::new() }
}

fn segment(nodes: [u16; 2], flags: u16, handles: [Vec3; 2]) -> RoadSegment {
    RoadSegment { nodes, length: 0.0, road: None, flags, start_handle: handles[0], end_handle: handles[1] }
}

/// A quarter circle of radius 50 m from the origin heading +z to (50, 0, 50) heading +x.
fn quarter() -> [Vec3; 2] {
    let k = 0.5523 * 50.0;
    [Vec3::new(0.0, 0.0, k), Vec3::new(-k, 0.0, 0.0)]
}

/// Profiles: 0 two lanes, 1 four lanes, 2 two lanes and a median. Nodes 0 to 3 on profile 0, 4 and 5 on profile 1,
/// 6 and 7 on profile 2.
fn network(segments: Vec<RoadSegment>) -> RoadNetwork {
    let traffic = zone::TRAFFIC;
    RoadNetwork {
        profiles: vec![
            profile(&[traffic, traffic]),
            profile(&[traffic, traffic, traffic, traffic]),
            profile(&[traffic, zone::MEDIAN, traffic]),
        ],
        nodes: vec![
            node(0.0, 0.0, 0),
            node(0.0, 100.0, 0),
            node(50.0, 50.0, 0),
            node(400.0, 0.0, 0),
            node(0.0, 1000.0, 1),
            node(0.0, 1200.0, 1),
            node(600.0, 600.0, 2),
            node(700.0, 600.0, 2),
        ],
        segments,
        roads: Vec::new(),
    }
}

#[test]
fn a_straight_segment_is_its_two_ends_in_render_space() {
    let net = network(vec![segment([0, 1], 0, [Vec3::ZERO; 2])]);
    let roads = MapRoads::build(&net, to_render);
    assert_eq!(roads.roads().len(), 1);
    assert_eq!(roads.roads()[0].points, vec![[0.0, 0.0], [100.0, 0.0]], "physics +z is render +x");
}

#[test]
fn a_curve_is_sampled_within_the_tolerance_and_keeps_few_points() {
    let curve = Bezier::with_handles(Vec3::ZERO, quarter()[0], quarter()[1], Vec3::new(50.0, 0.0, 50.0));
    let points = polyline(&curve, true, TOLERANCE_M, &to_render);
    assert!(points.len() > 2, "a bend is more than its chord");
    assert!(points.len() < 20, "a quarter circle of 50 m needs few points, got {}", points.len());
    assert_eq!(points[0], to_render(curve.start()));
    assert_eq!(*points.last().unwrap(), to_render(curve.end()));
    for i in 0..=100 {
        let on_curve = to_render(curve.position(i as f32 / 100.0));
        let gap = points.windows(2).map(|w| distance_to_segment(on_curve, w[0], w[1])).fold(f32::INFINITY, f32::min);
        // The sampling and the thinning each allow the tolerance, so the curve is within twice it of the line.
        assert!(gap <= 2.0 * TOLERANCE_M, "point {i} is {gap} m off the polyline");
    }
}

#[test]
fn douglas_peucker_drops_points_on_the_line_and_keeps_the_corners() {
    let line = [[0.0, 0.0], [10.0, 0.1], [20.0, -0.2], [30.0, 0.0]];
    assert_eq!(douglas_peucker(&line, 1.5), vec![[0.0, 0.0], [30.0, 0.0]]);
    let corner = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
    assert_eq!(douglas_peucker(&corner, 1.5), corner.to_vec());
    assert_eq!(douglas_peucker(&[[1.0, 1.0]], 1.5), vec![[1.0, 1.0]], "too short to thin");
}

#[test]
fn roads_are_classed_by_their_lanes_median_and_traffic_flag() {
    let net = network(vec![
        segment([0, 1], 0, [Vec3::ZERO; 2]),
        segment([4, 5], 0, [Vec3::ZERO; 2]),
        segment([6, 7], 0, [Vec3::ZERO; 2]),
        segment([0, 3], flags::NO_TRAFFIC, [Vec3::ZERO; 2]),
    ]);
    assert_eq!(MAIN_LANES, 4);
    assert_eq!(class_of(&net, 0), Some(RoadClass::Side), "two lanes");
    assert_eq!(class_of(&net, 1), Some(RoadClass::Main), "four lanes");
    assert_eq!(class_of(&net, 2), Some(RoadClass::Main), "a divided road");
    assert_eq!(class_of(&net, 3), Some(RoadClass::Path), "traffic never drives it");
}

#[test]
fn a_junction_keeps_its_through_connectors_and_drops_the_turns() {
    let decision = flags::DECISION | flags::INTERSECTION;
    let net =
        network(vec![segment([0, 1], decision, [Vec3::ZERO; 2]), segment([0, 2], decision | flags::CURVED, quarter())]);
    assert_eq!(class_of(&net, 0), Some(RoadClass::Side), "straight through the junction");
    assert_eq!(class_of(&net, 1), None, "a right turn inside the junction");
    assert_eq!(MapRoads::build(&net, to_render).roads().len(), 1);
}

fn road(points: Vec<[f32; 2]>) -> MapRoad {
    MapRoad { class: RoadClass::Side, points }
}

#[test]
fn the_grid_finds_the_roads_near_a_point_and_nothing_far_away() {
    let roads = vec![
        road(vec![[0.0, 0.0], [100.0, 0.0]]),
        road(vec![[2000.0, 2000.0], [2100.0, 2000.0]]),
        road(vec![[0.0, 50.0], [0.0, 900.0]]),
    ];
    let grid = RoadGrid::new(&roads);
    assert_eq!(grid.size(), ((2100.0 / CELL_M) as usize + 1, (2000.0 / CELL_M) as usize + 1));
    assert_eq!(grid.query([50.0, 10.0], 60.0), vec![0, 2], "sorted, each once");
    assert_eq!(grid.query([2050.0, 2000.0], 30.0), vec![1]);
    assert_eq!(grid.query([0.0, 700.0], 20.0), vec![2], "a long road is in every cell it crosses");
    assert!(grid.query([-5000.0, -5000.0], 100.0).is_empty(), "a circle off the grid");
    assert!(grid.query([0.0, 0.0], f32::NAN).is_empty());
    assert!(RoadGrid::new(&[]).query([0.0, 0.0], 1e6).is_empty(), "no roads, nothing found");
}

#[test]
fn map_roads_drop_a_road_of_one_point_and_query_through_the_grid() {
    let roads = MapRoads::from_roads(vec![road(vec![[5.0, 5.0]]), road(vec![[0.0, 0.0], [10.0, 0.0]])]);
    assert_eq!(roads.roads().len(), 1);
    assert_eq!(roads.near([5.0, 0.0], 10.0), vec![0]);
}

#[test]
fn handles_are_equal_only_when_they_share_the_roads() {
    let a = MapRoadsHandle::new(MapRoads::default());
    let b = MapRoadsHandle::new(MapRoads::default());
    assert_eq!(a, a.clone());
    assert_ne!(a, b, "the same content from another load is another network");
    assert!(a.same(&a.clone()));
}
