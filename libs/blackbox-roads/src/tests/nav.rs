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
fn a_traffic_lane_transition_keeps_its_previous_endpoint() {
    let mut net = chain_road();
    net.nodes[2].position = Vec3::new(100.0, 0.0, 100.0);
    for (segment, node_ind, lane) in [(0, 1, 3), (1, 0, 3)] {
        let mut nav = RoadNav::traffic();
        nav.place(&net, segment, node_ind, lane, 1.0);
        nav.enable_trail(&net);
        let previous_end = nav.position;
        let previous_bounds = nav.cookie(&net);
        nav.advance(&net, 0.01, Vec3::ZERO, &mut SplitMix(1));
        assert_ne!(nav.segment, segment);
        assert!(
            nav.lane_line().start().distance(previous_end) < 1e-4,
            "lane jumped from {previous_end:?} to {:?}",
            nav.lane_line().start()
        );
        assert!(nav.position.distance(previous_end) < 0.02, "{:?}", nav.position);
        assert_eq!(nav.lane, lane, "keeps the lane counted from the centre");
        let bounds = nav.cookie(&net);
        assert!(bounds.left.distance(previous_bounds.left) < 0.02, "left bound jumped: {bounds:?}");
        assert!(bounds.right.distance(previous_bounds.right) < 0.02, "right bound jumped: {bounds:?}");
        for _ in 0..10 {
            let previous = nav.position;
            nav.advance(&net, 0.5, Vec3::ZERO, &mut SplitMix(1));
            assert!((nav.position.distance(previous) - 0.5).abs() < 1e-3, "{:?}", nav.position);
            assert_eq!(nav.lane_line().start(), previous_end);
            assert!(nav.forward.is_finite() && nav.forward.length() > 0.99);
        }
    }
}

#[test]
fn a_curved_traffic_join_scales_the_new_handles_from_the_carried_endpoint() {
    let mut net = chain_road();
    net.nodes[2].position = Vec3::new(100.0, 0.0, 100.0);
    net.segments[1].flags = crate::flags::CURVED;
    net.segments[1].start_handle = Vec3::new(30.0, 0.0, 0.0);
    net.segments[1].end_handle = Vec3::new(-30.0, 0.0, 0.0);
    let mut nav = traffic_in_forward_lane(&net);
    nav.place(&net, 0, 1, 3, 1.0);
    nav.enable_trail(&net);
    let previous_end = nav.position;
    nav.advance(&net, 0.01, Vec3::ZERO, &mut SplitMix(1));
    let line = nav.lane_line();
    let expected_end = Vec3::new(100.0, 0.0, 98.0);
    let scale = previous_end.distance(expected_end) / 100.0;
    assert_eq!(line.start(), previous_end);
    assert_eq!(line.end(), expected_end);
    assert!(line.points[1].distance(previous_end + Vec3::X * 30.0 * scale) < 1e-4);
    assert!(line.points[2].distance(expected_end - Vec3::X * 30.0 * scale) < 1e-4);
    for _ in 0..20 {
        let previous = nav.position;
        nav.advance(&net, 0.5, Vec3::ZERO, &mut SplitMix(1));
        let travelled = nav.position.distance(previous);
        assert!((0.1..1.0).contains(&travelled), "travelled {travelled}");
        assert!(nav.position.is_finite() && nav.forward.is_finite() && nav.curvature.is_finite());
    }
}

#[test]
fn a_traffic_transition_selects_the_lane_at_the_far_end_profile() {
    let mut net = chain_road();
    net.nodes[2].profile = 1;
    net.profiles.push(crate::RoadProfile {
        middle: 1,
        zones: vec![lane(crate::zone::TRAFFIC, 2.0), lane(crate::zone::SHOULDER, 1.0), lane(crate::zone::TRAFFIC, 4.0)],
    });
    let mut nav = traffic_in_forward_lane(&net);
    nav.place(&net, 0, 1, 3, 1.0);
    let previous_end = nav.position;
    nav.advance(&net, 0.01, Vec3::ZERO, &mut SplitMix(1));
    assert_eq!((nav.segment, nav.node_ind, nav.lane), (1, 1, 2));
    assert_eq!(nav.lane_line().start(), previous_end);
    assert_eq!(nav.lane_line().end(), Vec3::new(4.0, 0.0, 200.0));
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
