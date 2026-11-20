//! The carriageway lookup on authored roads: no game data.

use super::*;

/// A straight road along x at y = 0 from 0 to `length`, `half` metres half width, at `level`.
fn straight(length: f32, half: f32, level: f32) -> Line {
    Line { points: vec![([0.0, 0.0], level, half), ([length / 2.0, 0.0], level, half), ([length, 0.0], level, half)] }
}

#[test]
fn a_point_on_the_road_is_inside_its_carriageway() {
    let roads = Roads::from_lines(&[straight(100.0, 5.0, 10.0)]);
    let near = roads.nearest([40.0, 2.0], 10.0, 5.0, 15.0).unwrap();
    assert!((near.gap - -3.0).abs() < 1e-4, "2 m from the middle of a 5 m half width: {near:?}");
    assert_eq!(near.level, 10.0);
}

#[test]
fn a_point_beside_the_road_is_its_distance_from_the_edge() {
    let roads = Roads::from_lines(&[straight(100.0, 5.0, 10.0)]);
    let near = roads.nearest([40.0, -9.0], 10.0, 5.0, 15.0).unwrap();
    assert!((near.gap - 4.0).abs() < 1e-4, "{near:?}");
    assert!((near.away[1] - -1.0).abs() < 1e-4, "away from the road is to the side the point is on: {near:?}");
    assert!(near.away[0].abs() < 1e-4);
}

#[test]
fn a_point_further_than_the_reach_is_not_near_a_road() {
    let roads = Roads::from_lines(&[straight(100.0, 5.0, 10.0)]);
    assert!(roads.nearest([40.0, 30.0], 10.0, 5.0, 15.0).is_none());
    assert!(roads.nearest([40.0, 14.0], 10.0, 5.0, 15.0).is_some(), "9 m from the edge is inside a reach of 10");
}

#[test]
fn a_bridge_over_the_point_or_a_road_under_it_is_another_road() {
    let roads = Roads::from_lines(&[straight(100.0, 5.0, 10.0)]);
    assert!(roads.nearest([40.0, 0.0], 10.0, 0.0, 6.0).is_none(), "the road is at 10, above the window 0 to 6");
    assert!(roads.nearest([40.0, 0.0], 10.0, 12.0, 30.0).is_none(), "and below the window 12 to 30");
}

#[test]
fn the_nearest_of_two_roads_is_taken() {
    let near_road = Line { points: vec![([0.0, 20.0], 0.0, 4.0), ([100.0, 20.0], 0.0, 4.0)] };
    let roads = Roads::from_lines(&[straight(100.0, 5.0, 0.0), near_road]);
    let near = roads.nearest([50.0, 14.0], 30.0, -1.0, 1.0).unwrap();
    assert!((near.gap - 2.0).abs() < 1e-4, "2 m from the edge of the road at y = 20: {near:?}");
}

#[test]
fn a_point_on_the_centre_line_is_sent_to_the_left_of_it() {
    let roads = Roads::from_lines(&[straight(100.0, 5.0, 0.0)]);
    let near = roads.nearest([40.0, 0.0], 10.0, -1.0, 1.0).unwrap();
    assert!((near.gap - -5.0).abs() < 1e-4);
    assert!((near.away[1] - 1.0).abs() < 1e-4, "{near:?}");
}

#[test]
fn a_road_past_a_cell_edge_is_found_from_both_sides() {
    let line = Line { points: vec![([0.0, 31.0], 0.0, 3.0), ([100.0, 31.0], 0.0, 3.0)] };
    let roads = Roads::from_lines(&[line]);
    assert!(roads.nearest([50.0, 33.0], 5.0, -1.0, 1.0).is_some());
    assert!(roads.nearest([50.0, 29.0], 5.0, -1.0, 1.0).is_some());
    assert!(roads.nearest([50.0, 60.0], 5.0, -1.0, 1.0).is_none());
}

#[test]
fn no_roads_is_empty() {
    assert!(Roads::from_lines(&[]).is_empty());
    assert!(Roads::default().nearest([0.0, 0.0], 100.0, -100.0, 100.0).is_none());
}

#[test]
fn only_the_lanes_a_vehicle_drives_on_are_carriageway() {
    assert!(CARRIAGEWAY.contains(&zone::TRAFFIC));
    assert!(CARRIAGEWAY.contains(&zone::SHOULDER));
    for kind in [zone::SIDEWALK, zone::MEDIAN, zone::GRASS_MEDIAN, zone::BARRIER, zone::UNDRIVABLE] {
        assert!(!CARRIAGEWAY.contains(&kind), "zone {kind}");
    }
}
