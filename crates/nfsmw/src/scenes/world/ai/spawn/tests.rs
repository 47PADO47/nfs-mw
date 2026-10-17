use blackbox_roads::{Road, RoadNetwork, RoadNode, RoadProfile, RoadSegment, SegmentIndex, SplitMix, Zone, zone};

use super::*;

fn focus() -> Focus {
    Focus { position: [0.0, 0.0], heading: 0.0, speed: 0.0, body: None }
}

#[test]
fn pc_clearance_and_projected_speed_are_independent_of_height_and_wedge_angle() {
    assert!(!clear_spawn(Vec3::new(0.0, 500.0, 299.0), focus(), 300.0, 0.0, std::iter::empty()));
    assert!(clear_spawn(Vec3::new(0.0, 500.0, 300.0), focus(), 300.0, 0.0, std::iter::empty()));
    let moving = Focus { speed: 50.0, ..focus() };
    assert!((projected_speed(moving, WEDGE) - 50.0 * WEDGE.cos()).abs() < 1e-4);
    assert!(projected_speed(moving, std::f32::consts::PI) < 0.0);
}

#[test]
fn a_probe_outside_the_exclusion_radius_cannot_spawn_on_a_closer_road() {
    let lane = |offset| Zone { kind: zone::TRAFFIC, width: 4.0, offset };
    let node = |z| RoadNode { position: Vec3::new(0.0, 0.0, z), profile: 0, segments: vec![0] };
    let net = RoadNetwork {
        nodes: vec![node(0.0), node(140.0)],
        segments: vec![RoadSegment {
            nodes: [0, 1],
            length: 140.0,
            road: Some(0),
            flags: 0,
            start_handle: Vec3::ZERO,
            end_handle: Vec3::ZERO,
        }],
        profiles: vec![RoadProfile { middle: 1, zones: vec![lane(2.0), lane(2.0)] }],
        roads: vec![Road { scale: 1.0, speech_id: 0 }],
    };
    let probe = Vec3::new(0.0, 0.0, 160.0);
    let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
    assert!(nav.init_at_point(&net, &SegmentIndex::build(&net), probe, -Vec3::Z, false));
    assert!(nav.can_traffic_spawn(&net, &mut SplitMix(1)));
    assert!(probe.length() >= 150.0 && nav.position.length() < 150.0, "{:?}", nav.position);
    assert!(!clear_spawn(nav.position, focus(), 150.0, 0.0, std::iter::empty()));
}

#[test]
fn the_exclusion_radius_is_planar_even_on_an_elevated_road() {
    let above = Vec3::new(0.0, 200.0, 100.0);
    assert!(!clear_spawn(above, focus(), 150.0, 0.0, std::iter::empty()));
    let edge = Vec3::new(0.0, 0.0, 150.0);
    assert!(clear_spawn(edge, focus(), 150.0, 0.0, std::iter::empty()));
}

#[test]
fn spacing_from_existing_vehicles_is_planar_and_keeps_trailer_room() {
    let at = Vec3::new(0.0, 0.0, 200.0);
    assert!(!clear_spawn(at, focus(), 150.0, 0.0, [(at + Vec3::new(0.0, 30.0, 19.0), 0.0)].into_iter()));
    assert!(clear_spawn(at, focus(), 150.0, 0.0, [(at + Vec3::Z * 20.0, 0.0)].into_iter()));
    assert!(!clear_spawn(at, focus(), 150.0, 12.0, [(at + Vec3::Z * 29.0, 8.0)].into_iter()));
    assert!(clear_spawn(at, focus(), 150.0, 12.0, [(at + Vec3::Z * 30.0, 8.0)].into_iter()));
}

fn straight_road() -> RoadNetwork {
    let lane = |offset| Zone { kind: zone::TRAFFIC, width: 4.0, offset };
    let node = |z| RoadNode { position: Vec3::new(0.0, 0.0, z), profile: 0, segments: vec![0] };
    RoadNetwork {
        nodes: vec![node(0.0), node(140.0)],
        segments: vec![RoadSegment {
            nodes: [0, 1],
            length: 140.0,
            road: Some(0),
            flags: 0,
            start_handle: Vec3::ZERO,
            end_handle: Vec3::ZERO,
        }],
        profiles: vec![RoadProfile { middle: 1, zones: vec![lane(2.0), lane(2.0)] }],
        roads: vec![Road { scale: 1.0, speech_id: 0 }],
    }
}

#[test]
fn a_point_spawn_finds_a_traffic_lane_at_the_point() {
    let net = straight_road();
    let point = Vec3::new(0.0, 0.0, 50.0);
    let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
    assert!(nav.init_at_point(&net, &SegmentIndex::build(&net), point, -Vec3::Z, false));
    assert!(nav.can_traffic_spawn(&net, &mut SplitMix(1)));
    assert!((nav.position - point).length() < 10.0, "the lane is at the point: {:?}", nav.position);
}

#[test]
fn a_point_spawn_may_be_near_the_player_but_keeps_its_spacing_from_cars() {
    let at = Vec3::new(0.0, 0.0, 60.0);
    assert!(!clear_spawn(at, focus(), 150.0, 0.0, std::iter::empty()), "the manager keeps its cars out of view");
    assert!(clear_of_cars(at, 0.0, std::iter::empty()), "a point spawn only has to keep the spacing");
    assert!(!clear_of_cars(at, 0.0, [(at + Vec3::Z * 10.0, 0.0)].into_iter()));
}
