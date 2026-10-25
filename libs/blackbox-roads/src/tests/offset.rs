use glam::Vec3;

use super::synth::{chain_road, straight_road};
use crate::{LaneType, NavKind, RoadNav, SegmentFilter, SplitMix, flags};

fn cop() -> RoadNav {
    RoadNav::new(NavKind::Direction, LaneType::Cop, SegmentFilter::default())
}

#[test]
fn continuous_offset_survives_plain_joins_and_reversal() {
    let net = chain_road();
    let mut nav = cop();
    nav.init_at_segment(&net, 0, 0.5, Vec3::new(3.25, 0.0, 50.0), Vec3::Z, false);
    nav.preserve_offset_at(&net, Vec3::new(3.25, 0.0, 50.0));
    nav.enable_trail(&net);
    assert!((nav.position.x - 3.25).abs() < 1e-4);
    nav.advance(&net, 75.0, Vec3::Z, &mut SplitMix(1));
    assert_eq!(nav.segment, 1);
    assert!((nav.position.x - 3.25).abs() < 1e-4);
    let at = nav.position;
    nav.reverse(&net);
    assert!(nav.position.distance(at) < 1e-4);
    assert!(nav.forward.dot(-Vec3::Z) > 0.99);
    nav.advance(&net, 50.0, -Vec3::Z, &mut SplitMix(1));
    assert_eq!(nav.segment, 0);
    assert!((nav.position.x - 3.25).abs() < 1e-4);
}

#[test]
fn explicit_lane_placement_restores_indexed_geometry() {
    let net = straight_road();
    let mut nav = cop();
    nav.place(&net, 0, 1, 3, 0.5);
    nav.preserve_offset_at(&net, Vec3::new(3.25, 0.0, 50.0));
    nav.place(&net, 0, 1, 3, 0.5);
    assert!((nav.position.x - 2.0).abs() < 1e-4);
}

#[test]
fn heading_initialization_keeps_wrong_way_direction_but_traffic_admission_rejects_it() {
    let mut net = straight_road();
    net.segments[0].flags |= flags::ONE_WAY;
    let mut nav = cop();
    nav.init_at_segment(&net, 0, 0.5, Vec3::Z * 50.0, -Vec3::Z, false);
    assert_eq!(nav.node_ind, 0);
    assert!(!nav.can_traffic_spawn(&net, &mut SplitMix(1)));
    nav.init_in_lane(&net, 0, 2, 0.5);
    assert_eq!(nav.node_ind, 1);
}

#[test]
fn continuous_geometry_is_part_of_deterministic_state() {
    let net = straight_road();
    let mut a = cop();
    a.place(&net, 0, 1, 3, 0.5);
    let mut b = a.clone();
    b.preserve_offset_at(&net, a.position);
    let words = |nav: &RoadNav| {
        let mut words = Vec::new();
        nav.visit_state(&mut |word| words.push(word));
        words
    };
    assert_eq!(a.position, b.position);
    assert_ne!(words(&a), words(&b));
}

#[test]
fn selectable_offset_keeps_within_lane_position_and_existing_trail() {
    let net = chain_road();
    let mut nav = cop();
    nav.place(&net, 0, 1, 3, 0.2);
    nav.enable_trail(&net);
    nav.advance(&net, 20.0, Vec3::Z, &mut SplitMix(1));
    let count = nav.trail().unwrap().cookies().len();
    assert!(count > 2);
    let correction = nav.snap_offset_at(&net, Vec3::new(3.25, 0.0, nav.position.z)).unwrap();
    assert!(correction < 1e-4);
    assert!((nav.position.x - 3.25).abs() < 1e-4);
    assert_eq!(nav.trail().unwrap().cookies().len(), count);
}

#[test]
fn selectable_offset_uses_the_heading_end_profile_on_a_taper() {
    let mut net = straight_road();
    let mut far = net.profiles[0].clone();
    for lane in &mut far.zones {
        lane.offset += 4.0;
    }
    net.profiles.push(far);
    net.nodes[1].profile = 1;
    let mut nav = cop();
    nav.place(&net, 0, 1, 3, 0.1);
    let correction = nav.snap_offset_at(&net, Vec3::new(3.25, 0.0, 10.0)).unwrap();
    assert!((correction - 0.75).abs() < 1e-4);
    assert!((nav.position.x - 4.0).abs() < 1e-4);
}
