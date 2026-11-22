use glam::Vec3;

use super::synth::*;
use crate::{Gps, GpsEnd, GpsState, SegmentIndex};

const FRAME: f32 = 1.0 / 60.0;

#[test]
fn the_arrow_points_a_look_ahead_distance_down_the_route() {
    let net = chain_road();
    let index = SegmentIndex::build(&net);
    let car = Vec3::new(0.0, 0.0, 10.0);
    let mut gps = Gps::engage(&net, &index, car, Vec3::Z, Vec3::new(0.0, 0.0, 150.0), 0.0).expect("a route");
    assert_eq!(gps.state(), GpsState::Tracking);
    assert_eq!(gps.update(&net, &index, car, Vec3::Z, 20.0, FRAME), None);
    let arrow = gps.arrow_point();
    assert!((arrow.z - car.z - 20.0).abs() < 2.0, "the look-ahead starts at 20 m: {arrow:?}");
    // The navigator takes the lane centre (here 2 m to one side of the road's midline), not the car's exact x.
    assert!((arrow.x - car.x).abs() < 2.5, "the arrow stays on the road lane: {arrow:?}");
    let direction = gps.arrow_direction(car).expect("the arrow is ahead");
    assert!(direction.dot(Vec3::Z) > 0.99, "{direction:?}");
}

#[test]
fn the_look_ahead_grows_with_speed_up_to_its_limit() {
    let net = chain_road();
    let index = SegmentIndex::build(&net);
    let car = Vec3::new(0.0, 0.0, 10.0);
    let mut gps = Gps::engage(&net, &index, car, Vec3::Z, Vec3::new(0.0, 0.0, 150.0), 0.0).expect("a route");
    for _ in 0..600 {
        gps.update(&net, &index, car, Vec3::Z, 60.0, FRAME);
    }
    // 60 m/s is 180 m of look-ahead, capped at 80.
    assert!((gps.look_ahead() - 80.0).abs() < 0.5, "{}", gps.look_ahead());
}

#[test]
fn the_route_finishes_within_arrival_distance_of_the_target() {
    let net = chain_road();
    let index = SegmentIndex::build(&net);
    let target = Vec3::new(0.0, 0.0, 150.0);
    let mut gps = Gps::engage(&net, &index, Vec3::new(0.0, 0.0, 10.0), Vec3::Z, target, 0.0).expect("a route");
    let near = Vec3::new(0.0, 0.0, 125.0);
    assert_eq!(gps.update(&net, &index, near, Vec3::Z, 10.0, FRAME), Some(GpsEnd::Finished));
}

#[test]
fn the_route_is_lost_when_the_car_strays_too_far() {
    let net = chain_road();
    let index = SegmentIndex::build(&net);
    let mut gps = Gps::engage(&net, &index, Vec3::new(0.0, 0.0, 10.0), Vec3::Z, Vec3::new(0.0, 0.0, 150.0), 0.0)
        .expect("a route");
    let far = Vec3::new(400.0, 0.0, 10.0);
    assert_eq!(gps.update(&net, &index, far, Vec3::Z, 10.0, FRAME), Some(GpsEnd::Lost));
}

#[test]
fn no_route_means_no_gps() {
    let net = chain_road();
    let index = SegmentIndex::build(&net);
    let gps = Gps::engage(&net, &index, Vec3::new(0.0, 0.0, 10.0), Vec3::Z, Vec3::new(5000.0, 0.0, 0.0), 0.0);
    assert!(gps.is_none());
}

#[test]
fn the_first_aim_point_is_ahead_before_any_update() {
    let net = chain_road();
    let index = SegmentIndex::build(&net);
    let car = Vec3::new(0.0, 0.0, 10.0);
    let gps = Gps::engage(&net, &index, car, Vec3::Z, Vec3::new(0.0, 0.0, 150.0), 0.0).expect("a route");
    let ahead = gps.arrow_point().z - car.z;
    assert!((ahead - 20.0).abs() < 2.0, "the aim starts a look-ahead ahead, not on the car: {ahead}");
}

/// A 100 m segment along +z, then a 100 m segment along +x from its end: a corner at (0, 0, 100).
fn corner_road() -> crate::RoadNetwork {
    let node =
        |x: f32, z: f32, segments: Vec<u16>| crate::RoadNode { position: Vec3::new(x, 0.0, z), profile: 0, segments };
    crate::RoadNetwork {
        nodes: vec![node(0.0, 0.0, vec![0]), node(0.0, 100.0, vec![0, 1]), node(100.0, 100.0, vec![1])],
        segments: vec![segment([0, 1], 100.0, 0), segment([1, 2], 100.0, 0)],
        profiles: vec![two_way_profile()],
        roads: vec![crate::Road { scale: 1.0, speech_id: 0 }],
    }
}

#[test]
fn following_the_route_keeps_it_from_the_car_to_the_target() {
    // A car driving up to the corner at 40 m/s: its look-ahead turns the corner long before the car does. The car must stay
    // on its route, which still holds the segment the car is on (the aim walks a copy of the navigator).
    let net = corner_road();
    let index = SegmentIndex::build(&net);
    let target = Vec3::new(90.0, 0.0, 100.0);
    let mut gps = Gps::engage(&net, &index, Vec3::new(0.0, 0.0, 5.0), Vec3::Z, target, 0.0).expect("a route");
    assert_eq!(gps.path(), &[0, 1]);
    let mut z = 5.0;
    while z < 90.0 {
        z += 40.0 * FRAME;
        assert_eq!(gps.update(&net, &index, Vec3::new(0.0, 0.0, z), Vec3::Z, 40.0, FRAME), None);
        assert_eq!(gps.state(), GpsState::Tracking, "the car stays on its route at z = {z}");
        assert_eq!(gps.path(), &[0, 1], "the route keeps the car's segment at z = {z}");
    }
    let aim = gps.arrow_point();
    assert!(aim.x > 5.0 && (aim.z - 100.0).abs() < 5.0, "near the corner the arrow aims round it: {aim:?}");
    let mut x = 2.0;
    for _ in 0..20 {
        x += 40.0 * FRAME;
        gps.update(&net, &index, Vec3::new(x, 0.0, 100.0), Vec3::X, 40.0, FRAME);
    }
    assert_eq!(gps.state(), GpsState::Tracking);
    assert_eq!(gps.path(), &[1], "past the corner the segment behind the car is dropped");
}
