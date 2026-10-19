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
