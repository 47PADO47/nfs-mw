//! Matching prop poses to the signal approaches: reach, facing, side, mast arms and tie-breaks.

use glam::Vec3;

use crate::{Approach, MatchParams, PropPose, STOP_LINE_DISTANCE, match_props, right_of};

/// An approach to the junction at the origin, driving along `heading`, stopping on the centre line.
fn approach(junction: usize, heading: Vec3) -> Approach {
    Approach {
        junction,
        segment: 0,
        node: 0,
        node_ind: 0,
        stop_position: -heading * STOP_LINE_DISTANCE,
        heading,
        phase: 0,
        kerb: 4.0,
    }
}

/// The four approaches of a four-way junction: driving north, east, south and west.
fn four_way() -> [Approach; 4] {
    [Vec3::Z, Vec3::X, Vec3::NEG_Z, Vec3::NEG_X].map(|heading| approach(0, heading))
}

/// A post `along` metres from the stop point along the heading (negative is back from it), 4.5 m right of the lane,
/// facing back along the approach.
fn right_post(road: &Approach, along: f32) -> PropPose {
    let heading = road.heading;
    PropPose {
        position: road.stop_position + right_of(heading) * 4.5 + heading * along,
        forward: -heading,
        kind: 7,
        mast_arm: false,
    }
}

/// A unit vector `degrees` away from the direction back along a northbound approach, toward the east.
fn facing_off(degrees: f32) -> Vec3 {
    let turn = degrees.to_radians();
    Vec3::new(turn.sin(), 0.0, -turn.cos())
}

#[test]
fn each_approach_of_a_four_way_junction_gets_its_own_post() {
    let roads = four_way();
    // The posts come in the reverse order of the approaches, so the matcher cannot rely on order.
    let posts: Vec<PropPose> = roads.iter().rev().map(|road| right_post(road, -1.0)).collect();
    let matched = match_props(&roads, &posts, &MatchParams::default());
    assert_eq!(matched, vec![Some(3), Some(2), Some(1), Some(0)]);
}

#[test]
fn a_post_facing_with_the_traffic_is_rejected() {
    let roads = [approach(0, Vec3::Z)];
    let mut post = right_post(&roads[0], -1.0);
    post.forward = roads[0].heading;
    assert_eq!(match_props(&roads, &[post], &MatchParams::default()), vec![None]);
}

#[test]
fn a_post_on_the_left_of_the_lane_is_rejected() {
    let roads = [approach(0, Vec3::Z)];
    let mut post = right_post(&roads[0], -1.0);
    post.position = roads[0].stop_position - right_of(roads[0].heading) * 4.5 - roads[0].heading;
    assert_eq!(match_props(&roads, &[post], &MatchParams::default()), vec![None]);
}

#[test]
fn a_far_side_mast_arm_serves_two_approaches_of_one_group() {
    let roads = [approach(0, Vec3::Z), {
        let mut second = approach(0, Vec3::Z);
        second.stop_position.x = -4.0;
        second
    }];
    // Across the junction, 9 m past the corner, facing back toward both approaches.
    let mast = PropPose { position: Vec3::new(-2.0, 6.0, 9.0), forward: Vec3::NEG_Z, kind: 1, mast_arm: true };
    assert_eq!(match_props(&roads, &[mast], &MatchParams::default()), vec![Some(0), Some(0)]);
}

#[test]
fn a_post_that_is_not_a_mast_arm_serves_only_one_approach() {
    let roads = [approach(0, Vec3::Z), {
        let mut second = approach(0, Vec3::Z);
        second.stop_position.x = -4.0;
        second
    }];
    let mast = PropPose { position: Vec3::new(-2.0, 6.0, 9.0), forward: Vec3::NEG_Z, kind: 1, mast_arm: false };
    assert_eq!(match_props(&roads, &[mast], &MatchParams::default()), vec![Some(0), None]);
}

#[test]
fn a_mast_arm_does_not_serve_approaches_of_another_junction() {
    let roads = [approach(0, Vec3::Z), {
        let mut second = approach(1, Vec3::Z);
        second.stop_position.x = -4.0;
        second
    }];
    let mast = PropPose { position: Vec3::new(-2.0, 6.0, 9.0), forward: Vec3::NEG_Z, kind: 1, mast_arm: true };
    assert_eq!(match_props(&roads, &[mast], &MatchParams::default()), vec![Some(0), None]);
}

#[test]
fn a_post_beyond_the_radius_is_rejected_and_the_radius_is_configurable() {
    let roads = [approach(0, Vec3::Z)];
    let near = right_post(&roads[0], -15.0);
    let far = right_post(&roads[0], -30.0);
    let default = MatchParams::default();
    assert_eq!(match_props(&roads, &[far], &default), vec![None]);
    assert_eq!(match_props(&roads, &[near], &default), vec![Some(0)]);
    let tight = MatchParams { max_distance: 10.0, ..default };
    assert_eq!(match_props(&roads, &[near], &tight), vec![None]);
}

#[test]
fn facing_is_judged_against_the_facing_tolerance() {
    let roads = [approach(0, Vec3::Z)];
    let default = MatchParams::default();
    let mut post = right_post(&roads[0], -1.0);
    post.forward = facing_off(50.0);
    assert_eq!(match_props(&roads, &[post], &default), vec![Some(0)]);
    post.forward = facing_off(70.0);
    assert_eq!(match_props(&roads, &[post], &default), vec![None]);
    post.forward = Vec3::ZERO;
    assert_eq!(match_props(&roads, &[post], &default), vec![None]);
}

#[test]
fn duplicate_approaches_share_one_post_by_approach_index() {
    let roads = [approach(0, Vec3::Z), approach(0, Vec3::Z)];
    let post = right_post(&roads[0], -1.0);
    assert_eq!(match_props(&roads, &[post], &MatchParams::default()), vec![Some(0), None]);
}

#[test]
fn duplicate_posts_are_taken_by_the_lower_index() {
    let roads = [approach(0, Vec3::Z)];
    let post = right_post(&roads[0], -1.0);
    let matched = match_props(&roads, &[post, post], &MatchParams::default());
    assert_eq!(matched, vec![Some(0)]);
}

#[test]
fn matching_is_deterministic() {
    let roads = [approach(0, Vec3::Z), approach(0, Vec3::Z), approach(0, Vec3::X)];
    let posts = [right_post(&roads[0], -1.0), right_post(&roads[0], -2.0), right_post(&roads[2], -1.0)];
    let params = MatchParams::default();
    let first = match_props(&roads, &posts, &params);
    for _ in 0..10 {
        assert_eq!(match_props(&roads, &posts, &params), first);
    }
}
