use blackbox_roads::{Road, RoadNetwork, RoadNode, RoadProfile, RoadSegment, SegmentIndex, SplitMix, Zone, zone};

use super::*;

fn focus() -> Focus {
    Focus { position: [0.0, 0.0], heading: 0.0, speed: 0.0, body: None }
}

#[test]
fn driven_probe_keeps_elevation_so_sloping_roads_do_not_snap_towards_sea_level() {
    let mut net = straight_road();
    net.nodes[0].position.y = 150.0;
    net.nodes[1].position.y = 170.0;
    let body = blackbox_roads::Body {
        position: Vec3::new(0.0, 160.0, 0.0),
        velocity: Vec3::ZERO,
        forward: glam::Vec2::Y,
        half_width: 1.0,
        half_length: 2.5,
    };
    let probe = probe_point(Focus { body: Some(body), ..focus() }, 70.0, 0.0);
    let physics = Vec3::from(space::to_physics(probe));
    assert_eq!(physics.y, 160.0);
    let curve = blackbox_roads::centre_line(&net, 0, 1);
    let correct = curve.position(curve.closest_t(physics, 140.0));
    let flattened = Vec3::new(physics.x, 0.0, physics.z);
    let wrong = curve.position(curve.closest_t(flattened, 140.0));
    assert!(correct.distance(physics) < 1.0);
    assert!(wrong.distance(correct) > 15.0, "fixture must expose the old elevation-induced snapping");
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

#[test]
fn the_patrol_probe_reaches_every_side_inside_the_cop_window() {
    let mut rng = SplitMix(7);
    let (mut ahead, mut behind, mut left, mut right) = (0, 0, 0, 0);
    for _ in 0..400 {
        let probe = patrol_probe(&mut rng, focus());
        assert!((COP_MIN_DISTANCE..=COP_MAX_DISTANCE).contains(&probe.distance), "{}", probe.distance);
        let planar = probe.at.x.hypot(probe.at.y);
        assert!((planar - probe.distance).abs() < 1e-2, "the point lies at the aimed distance");
        ahead += usize::from(probe.at.x > 0.0);
        behind += usize::from(probe.at.x < 0.0);
        left += usize::from(probe.at.y > 0.0);
        right += usize::from(probe.at.y < 0.0);
    }
    assert!([ahead, behind, left, right].iter().all(|&n| n > 120), "{ahead} {behind} {left} {right}");
}

#[test]
fn the_patrol_probe_is_pushed_along_by_the_player_velocity_but_keeps_its_window_on_the_road_point() {
    let body = blackbox_roads::Body {
        position: Vec3::ZERO,
        velocity: Vec3::new(0.0, 0.0, 60.0),
        forward: glam::Vec2::Y,
        half_width: 1.0,
        half_length: 2.5,
    };
    let moving = Focus { body: Some(body), ..focus() };
    let mut rng = SplitMix(11);
    let mean: f32 = (0..400).map(|_| patrol_probe(&mut rng, moving).at.x).sum::<f32>() / 400.0;
    assert!(mean > 30.0, "the disc shifts ahead with the velocity: {mean}");
}

#[test]
fn a_cop_spawn_point_must_lie_in_the_window_and_outside_the_no_spawn_zones() {
    let mut cops = crate::scenes::world::ai::cop_manager::CopManager::default();
    let at = |gap: f32| Vec3::new(0.0, 0.0, gap);
    assert!(!cop_spawn_point_valid(&cops, at(149.0), focus()));
    assert!(cop_spawn_point_valid(&cops, at(150.0), focus()));
    assert!(cop_spawn_point_valid(&cops, at(400.0), focus()));
    assert!(!cop_spawn_point_valid(&cops, at(401.0), focus()));
    cops.set_no_spawn_zones(vec![crate::scenes::world::ai::cop_manager::square_zone(500.0)]);
    assert!(!cop_spawn_point_valid(&cops, at(200.0), focus()), "inside a no-cop-spawn zone");
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
