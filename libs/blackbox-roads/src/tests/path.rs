use glam::Vec3;

use super::synth::*;
use crate::{PathState, PathType, RoadNav, SegmentIndex, SplitMix};

#[test]
fn a_path_along_a_chain_visits_each_segment() {
    let net = chain_road();
    let index = SegmentIndex::build(&net);
    let mut nav = RoadNav::traffic();
    nav.init_in_lane(&net, 0, 3, 0.1);
    let state = nav.find_path_to(&net, &index, Vec3::new(0.0, 0.0, 170.0), None, PathType::Cop);
    assert_eq!(state, PathState::Full);
    assert_eq!(nav.path, vec![0, 1]);
    // The cursor follows it across the chain node and ends up on the goal segment.
    nav.advance(&net, 120.0, Vec3::ZERO, &mut SplitMix(1));
    assert_eq!(nav.segment, 1);
    assert!(nav.on_path(&net));
}

#[test]
fn the_distance_remaining_counts_down() {
    let net = chain_road();
    let index = SegmentIndex::build(&net);
    let mut nav = RoadNav::traffic();
    nav.init_in_lane(&net, 0, 3, 0.0);
    nav.find_path_to(&net, &index, Vec3::new(0.0, 0.0, 150.0), None, PathType::Cop);
    let start = nav.distance_remaining(&net);
    assert!((start - 150.0).abs() < 5.0, "{start}");
    nav.advance(&net, 60.0, Vec3::ZERO, &mut SplitMix(1));
    let later = nav.distance_remaining(&net);
    assert!((start - later - 60.0).abs() < 6.0, "{start} -> {later}");
}

#[test]
fn a_goal_behind_makes_the_cursor_turn_round() {
    let net = chain_road();
    let index = SegmentIndex::build(&net);
    let mut nav = RoadNav::traffic();
    nav.init_in_lane(&net, 1, 3, 0.5);
    assert_eq!(nav.node_ind, 1);
    let state = nav.find_path_to(&net, &index, Vec3::new(0.0, 0.0, 20.0), None, PathType::Cop);
    assert_eq!(state, PathState::Full);
    assert_eq!(nav.path, vec![1, 0]);
    assert_eq!(nav.node_ind, 0, "now heading back along the stored direction");
}

#[test]
fn no_goal_on_the_map_means_no_way() {
    let net = chain_road();
    let index = SegmentIndex::build(&net);
    let mut nav = RoadNav::traffic();
    nav.init_in_lane(&net, 0, 3, 0.1);
    let state = nav.find_path_to(&net, &index, Vec3::new(5000.0, 0.0, 0.0), None, PathType::Cop);
    assert_eq!(state, PathState::NoWay);
    assert!(nav.path.is_empty());
}
