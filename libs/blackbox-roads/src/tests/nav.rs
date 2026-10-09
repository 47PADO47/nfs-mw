use glam::Vec3;

use super::synth::*;
use crate::{LaneType, NavKind, RoadNav, SegmentFilter, SplitMix};

fn traffic_in_forward_lane(net: &crate::RoadNetwork) -> RoadNav {
    let mut nav = RoadNav::traffic();
    nav.init_in_lane(net, 0, 3, 0.0);
    nav
}

#[test]
fn a_cursor_follows_its_lane_across_a_chain_node() {
    let net = chain_road();
    let mut nav = traffic_in_forward_lane(&net);
    let mut rng = SplitMix(1);
    nav.advance(&net, 150.0, Vec3::ZERO, &mut rng);
    assert_eq!((nav.segment, nav.node_ind), (1, 1));
    assert!(nav.position.distance(Vec3::new(2.0, 0.0, 150.0)) < 0.5, "{:?}", nav.position);
    assert!(!nav.dead_end);
}

#[test]
fn a_traffic_cursor_stops_at_a_dead_end() {
    let net = chain_road();
    let mut nav = traffic_in_forward_lane(&net);
    nav.advance(&net, 500.0, Vec3::ZERO, &mut SplitMix(1));
    assert!(nav.dead_end);
    assert!(nav.position.z > 199.0 && nav.position.z <= 200.1);
}

#[test]
fn lanes_against_the_stored_direction_run_backwards() {
    let net = chain_road();
    let mut nav = RoadNav::traffic();
    // Zone 2 is the lane left of the middle, 2 m left of the centre line in the stored direction.
    nav.init_in_lane(&net, 1, 2, 0.0);
    assert_eq!(nav.node_ind, 0);
    assert!(nav.forward.z < -0.99);
    assert!(nav.position.distance(Vec3::new(-2.0, 0.0, 200.0)) < 0.5, "{:?}", nav.position);
}

#[test]
fn init_at_point_picks_the_lane_and_direction() {
    let net = chain_road();
    let index = crate::SegmentIndex::build(&net);
    let mut nav = RoadNav::traffic();
    assert!(nav.init_at_point(&net, &index, Vec3::new(1.0, 0.0, 30.0), Vec3::Z, true));
    assert_eq!((nav.segment, nav.node_ind), (0, 1));
    let mut oncoming = RoadNav::traffic();
    assert!(oncoming.init_at_point(&net, &index, Vec3::new(-1.0, 0.0, 30.0), -Vec3::Z, true));
    assert_eq!(oncoming.node_ind, 0);
    assert!(!RoadNav::traffic().init_at_point(&net, &index, Vec3::new(500.0, 0.0, 0.0), Vec3::Z, true));
}

#[test]
fn a_direction_cursor_turns_around_at_a_dead_end() {
    let net = chain_road();
    let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
    nav.place(&net, 1, 1, 3, 0.5);
    nav.advance(&net, 120.0, Vec3::Z, &mut SplitMix(1));
    assert!(!nav.dead_end);
    assert_eq!(nav.node_ind, 0, "heads back after the dead end");
}

#[test]
fn lane_types_mask_the_zone_types() {
    use crate::zone;
    assert_eq!(LaneType::Traffic.drivable_mask(), 1 << zone::TRAFFIC);
    assert_eq!(LaneType::Cop.drivable_mask() & (1 << zone::SIDEWALK), 0);
    assert_ne!(LaneType::Racing.drivable_mask() & (1 << zone::SIDEWALK), 0);
    assert_eq!(LaneType::Racing.drivable_mask() & (1 << zone::BARRIER), 0);
    assert_ne!(LaneType::Any.drivable_mask() & (1 << zone::BARRIER), 0);
}
