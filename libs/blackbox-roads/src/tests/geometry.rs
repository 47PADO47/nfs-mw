use glam::Vec3;

use super::synth::*;
use crate::{Bezier, SegmentFilter, SegmentIndex, closest_segment, flags, lane_line, travel_profile};

#[test]
fn bezier_line_is_straight_and_evenly_parameterised() {
    let line = Bezier::line(Vec3::ZERO, Vec3::new(0.0, 0.0, 90.0));
    assert!(line.position(0.5).distance(Vec3::new(0.0, 0.0, 45.0)) < 1e-4);
    assert!((line.length(8) - 90.0).abs() < 1e-3);
    assert!(line.curvature_xz(0.3).abs() < 1e-6);
    assert!((line.closest_t(Vec3::new(5.0, 0.0, 30.0), 90.0) - 1.0 / 3.0).abs() < 0.02);
}

#[test]
fn bezier_arc_turns_the_expected_way() {
    // A quarter-ish arc from the origin heading +z, bending towards +x.
    let arc = Bezier::with_handles(
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, 20.0),
        Vec3::new(-20.0, 0.0, 0.0),
        Vec3::new(40.0, 0.0, 40.0),
    );
    assert!(arc.curvature_xz(0.5) > 0.0, "turning right is positive");
    let left = Bezier::with_handles(
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, 20.0),
        Vec3::new(20.0, 0.0, 0.0),
        Vec3::new(-40.0, 0.0, 40.0),
    );
    assert!(left.curvature_xz(0.5) < 0.0);
}

#[test]
fn forward_lanes_sit_right_of_travel_in_both_directions() {
    let net = straight_road();
    // Heading to node 1 (+z): right of travel is +x; the first forward lane is zone 3, 2 m right.
    let forward = lane_line(&net, 0, 1, 3);
    assert!(forward.start().distance(Vec3::new(2.0, 0.0, 0.0)) < 1e-4, "{:?}", forward.start());
    assert!(forward.end().distance(Vec3::new(2.0, 0.0, 100.0)) < 1e-4);
    // Heading to node 0 (-z): right of travel is -x, so the same lane index lies at x = -2.
    let back = lane_line(&net, 0, 0, 3);
    assert!(back.start().distance(Vec3::new(-2.0, 0.0, 100.0)) < 1e-4, "{:?}", back.start());
    assert!(back.end().distance(Vec3::new(-2.0, 0.0, 0.0)) < 1e-4);
}

#[test]
fn inverted_ends_are_mirrored() {
    let mut net = straight_road();
    net.segments[0].flags |= flags::END_INVERTED;
    net.profiles[0].zones[5].offset = 12.0; // make the asymmetry visible
    let to_end = travel_profile(&net, 0, 1, true);
    assert_eq!(to_end.zones[0].offset, 12.0);
    assert_eq!(to_end.middle, 3);
}

#[test]
fn closest_segment_prefers_the_road_the_car_points_along() {
    let net = straight_road();
    let index = SegmentIndex::build(&net);
    let hit = closest_segment(&net, &index, Vec3::new(3.0, 0.0, 40.0), Vec3::Z, 1.0, SegmentFilter::default()).unwrap();
    assert_eq!(hit.segment, 0);
    assert!((hit.t - 0.4).abs() < 0.02);
    assert!(
        closest_segment(&net, &index, Vec3::new(900.0, 0.0, 40.0), Vec3::Z, 1.0, SegmentFilter::default()).is_none()
    );
}

#[test]
fn filters_exclude_segments() {
    let mut net = straight_road();
    net.segments[0].flags |= flags::NO_TRAFFIC;
    let index = SegmentIndex::build(&net);
    let at = Vec3::new(0.0, 0.0, 10.0);
    let traffic = SegmentFilter { traffic: true, ..SegmentFilter::default() };
    assert!(closest_segment(&net, &index, at, Vec3::Z, 0.0, traffic).is_none());
    // Cops may use exactly the segments traffic may not... when the xor flag is set.
    let cop = SegmentFilter { cop: true, ..SegmentFilter::default() };
    assert!(closest_segment(&net, &index, at, Vec3::Z, 0.0, cop).is_none());
    net.segments[0].flags |= flags::COPS_XOR_TRAFFIC;
    assert!(closest_segment(&net, &index, at, Vec3::Z, 0.0, cop).is_some());
}
