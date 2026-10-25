use super::*;
use crate::{Road, RoadNode, RoadProfile, RoadSegment, SegmentFilter, Zone, closest_segment, flags, zone};
use glam::Vec2;

fn network(points: &[[f32; 3]], edges: &[[u16; 2]]) -> RoadNetwork {
    let nodes = points
        .iter()
        .enumerate()
        .map(|(index, point)| RoadNode {
            position: Vec3::from_array(*point),
            profile: 0,
            segments: edges
                .iter()
                .enumerate()
                .filter_map(|(i, ends)| ends.contains(&(index as u16)).then_some(i as u16))
                .collect(),
        })
        .collect();
    let segments = edges
        .iter()
        .map(|ends| RoadSegment {
            nodes: *ends,
            length: Vec3::from_array(points[usize::from(ends[0])])
                .distance(Vec3::from_array(points[usize::from(ends[1])])),
            road: Some(0),
            flags: 0,
            start_handle: Vec3::ZERO,
            end_handle: Vec3::ZERO,
        })
        .collect();
    RoadNetwork {
        nodes,
        segments,
        profiles: vec![RoadProfile { middle: 0, zones: vec![Zone { kind: zone::TRAFFIC, width: 4.0, offset: 0.0 }] }],
        roads: vec![Road { scale: 1.0, speech_id: 0 }],
    }
}

fn body(z: f32, speed: f32) -> Body {
    Body { position: Vec3::Z * z, velocity: Vec3::Z * speed, forward: Vec2::Y, half_width: 1.0, half_length: 2.0 }
}

fn straight() -> RoadNetwork {
    network(&[[0.0, 0.0, -100.0], [0.0, 0.0, 1000.0]], &[[0, 1]])
}

fn words(tracker: &RoadTracker) -> Vec<u32> {
    let mut words = Vec::new();
    tracker.visit_state(&mut |word| words.push(word));
    words
}

#[test]
fn target_prediction_keeps_lateral_position_inside_a_selectable_lane() {
    let mut net = straight();
    net.profiles[0] = RoadProfile { middle: 1, zones: vec![Zone { kind: zone::TRAFFIC, width: 10.0, offset: 5.0 }; 2] };
    let index = SegmentIndex::build(&net);
    let mut tracker = RoadTracker::default();
    let target = Body { position: Vec3::new(3.25, 0.0, 0.0), ..body(0.0, 20.0) };
    tracker.update(&net, &index, Some(target), 1.0 / 60.0);
    let observed = tracker.snapshot().unwrap();
    assert!((observed.current.x - 3.25).abs() < 1e-4, "{observed:?}");
    assert!((observed.future.x - 3.25).abs() < 1e-4, "{observed:?}");
    let seek = tracker.seek_ahead(&net, 20.0, 7.8, false).unwrap();
    assert!((seek.x - 3.25).abs() < 1e-4, "{seek:?}");
    tracker.update(&net, &index, Some(Body { position: Vec3::new(3.5, 0.0, 0.5), ..target }), 0.025);
    assert!((tracker.snapshot().unwrap().current.x - 3.5).abs() < 1e-4);
}

#[test]
fn tracker_keeps_the_nearby_continuation_instead_of_a_distant_collinear_endpoint() {
    let net = network(&[[0.0, 0.0, -80.0], [0.0, 0.0, -40.0], [0.0, 0.0, 0.0], [0.0, 0.0, 600.0]], &[[0, 1], [2, 3]]);
    let index = SegmentIndex::build(&net);
    let first = body(5.0, 20.0);
    let weighted = closest_segment(&net, &index, first.position, Vec3::Z, 1.0, SegmentFilter::default()).unwrap();
    assert_eq!(weighted.segment, 0, "fixture exposes the old weighted endpoint association");
    assert!(weighted.point.distance(first.position) > 40.0);
    let mut tracker = RoadTracker::default();
    tracker.update(&net, &index, Some(first), 1.0 / 60.0);
    assert_eq!(tracker.current().unwrap().segment, 1);
    assert!(tracker.snapshot().unwrap().current.distance(first.position) < 0.5);
    tracker.update(&net, &index, Some(body(6.0, 20.0)), 1.0 / 60.0);
    assert_eq!(tracker.current().unwrap().segment, 1);
    assert!(tracker.snapshot().unwrap().current.z > 5.5, "retained current cursor moves forward");
    assert!(tracker.snapshot().unwrap().future.z > 24.0);
}

#[test]
fn twenty_metre_displacement_forces_a_rethink_before_the_periodic_timeout() {
    let net = straight();
    let index = SegmentIndex::build(&net);
    let mut tracker = RoadTracker::default();
    tracker.update(&net, &index, Some(body(0.0, 20.0)), 1.0 / 60.0);
    let before = tracker.last_rethink;
    tracker.update(&net, &index, Some(body(30.0, 20.0)), 1.0 / 60.0);
    assert!(tracker.last_rethink > before);
    assert!(tracker.snapshot().unwrap().current.z > 29.5);
    assert!(tracker.clock - before < 0.33);
}

#[test]
fn incremental_clock_keeps_repeated_queries_from_advancing_the_same_cursor() {
    let net = straight();
    let index = SegmentIndex::build(&net);
    let mut tracker = RoadTracker::default();
    tracker.update(&net, &index, Some(body(0.0, 20.0)), 0.01);
    tracker.update(&net, &index, Some(body(1.0, 20.0)), 0.01);
    let current = tracker.snapshot().unwrap();
    tracker.update(&net, &index, Some(body(2.0, 20.0)), 0.005);
    assert_eq!(tracker.snapshot().unwrap(), current);
    tracker.update(&net, &index, Some(body(2.0, 20.0)), 0.02);
    assert!(tracker.snapshot().unwrap().current.z > current.current.z + 0.5);
}

#[test]
fn absent_observations_age_the_seek_cache_without_exposing_stale_snapshots() {
    let net = straight();
    let index = SegmentIndex::build(&net);
    let mut tracker = RoadTracker::default();
    tracker.update(&net, &index, Some(body(0.0, 20.0)), 0.02);
    let first = tracker.seek_ahead(&net, 20.0, 7.8, true).unwrap();
    tracker.update(&net, &index, Some(body(1.0, 20.0)), 0.02);
    assert_eq!(tracker.seek_ahead(&net, 80.0, 3.12, true), Some(first), "cache is shared across callers");
    tracker.update(&net, &index, None, 1.0);
    assert!(tracker.current().is_none() && tracker.snapshot().is_none());
    assert!(tracker.seek_ahead(&net, 80.0, 3.12, true).is_none());
    tracker.update(&net, &index, Some(body(10.0, 40.0)), 0.02);
    let next = tracker.seek_ahead(&net, 40.0, 7.8, true).unwrap();
    assert!(next.z > first.z + 100.0, "inactive time expires the old seek cache");
}

#[test]
fn fork_search_prefers_forward_motion_over_the_first_branch() {
    let mut net = network(
        &[[0.0, 0.0, -100.0], [0.0, 0.0, 20.0], [100.0, 0.0, 20.0], [0.0, 0.0, 500.0]],
        &[[0, 1], [1, 2], [1, 3]],
    );
    net.segments[1].flags = flags::DECISION;
    net.segments[2].flags = flags::DECISION;
    let mut tracker = RoadTracker::default();
    tracker.update(&net, &SegmentIndex::build(&net), Some(body(0.0, 40.0)), 0.02);
    let observed = tracker.snapshot().unwrap();
    assert_eq!(tracker.current().unwrap().segment, 0);
    assert_eq!(tracker.future().unwrap().segment, 2);
    assert!(observed.future.x.abs() < 0.1 && observed.future.z > 39.0);
    assert!(tracker.far.z > 89.0);
}

#[test]
fn blocked_branch_is_rejected_even_when_it_points_forward() {
    let mut net = network(
        &[[0.0, 0.0, -100.0], [0.0, 0.0, 20.0], [50.0, 0.0, 150.0], [0.0, 0.0, 500.0]],
        &[[0, 1], [1, 2], [1, 3]],
    );
    net.segments[1].flags = flags::DECISION;
    net.segments[2].flags = flags::DECISION | flags::CROSSES_BARRIER;
    let mut tracker = RoadTracker::default();
    tracker.update(&net, &SegmentIndex::build(&net), Some(body(0.0, 40.0)), 0.02);
    assert_eq!(tracker.future().unwrap().segment, 1);
}

#[test]
fn malformed_zero_length_cycle_has_bounded_work_and_no_invented_road() {
    let mut net = network(&[[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]], &[[0, 1], [1, 0]]);
    for segment in &mut net.segments {
        segment.length = 0.0;
    }
    let mut tracker = RoadTracker::default();
    tracker.update(&net, &SegmentIndex::build(&net), Some(body(0.0, 20.0)), 0.02);
    assert!(tracker.snapshot().is_none());
}

#[test]
fn state_words_include_future_route_identity_clocks_and_cached_seek() {
    let net = straight();
    let index = SegmentIndex::build(&net);
    let mut tracker = RoadTracker::default();
    tracker.update(&net, &index, Some(body(0.0, 20.0)), 0.02);
    let mut cached = tracker.clone();
    cached.seek_ahead(&net, 20.0, 7.8, true).unwrap();
    assert_eq!(tracker.snapshot(), cached.snapshot());
    assert_ne!(words(&tracker), words(&cached));
    let mut previous = tracker.clone();
    previous.previous = Some((0, 0));
    assert_ne!(words(&tracker), words(&previous));
    let mut clock = tracker.clone();
    clock.clock += 0.001;
    assert_eq!(tracker.snapshot(), clock.snapshot());
    assert_ne!(words(&tracker), words(&clock));
    assert_eq!(words(&tracker), words(&tracker.clone()));
}
