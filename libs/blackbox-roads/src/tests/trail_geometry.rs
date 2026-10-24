//! A trail's centre and bounds share independently constructed endpoint corridors.

use glam::Vec3;

use super::synth::straight_road;
use crate::{LaneType, NavKind, RoadNav, RoadProfile, SegmentFilter, Zone, zone};

fn profile(lane_width: f32) -> RoadProfile {
    let lane = |offset| Zone { kind: zone::TRAFFIC, width: lane_width, offset };
    RoadProfile {
        middle: 2,
        zones: vec![lane(lane_width * 1.5), lane(lane_width * 0.5), lane(lane_width * 0.5), lane(lane_width * 1.5)],
    }
}

fn navigator() -> RoadNav {
    RoadNav::new(NavKind::Direction, LaneType::CopReckless, SegmentFilter::default())
}

fn taper() -> crate::RoadNetwork {
    let mut net = straight_road();
    net.profiles = vec![profile(4.0), profile(6.0)];
    net.nodes[1].profile = 1;
    net
}

#[test]
fn trail_geometry_clamps_an_outer_lane_centre_inside_the_inset_corridor_in_both_directions() {
    let mut net = straight_road();
    net.profiles = vec![profile(4.0)];
    for (node_ind, sign) in [(1, 1.0), (0, -1.0)] {
        let mut nav = navigator();
        nav.half_width = 1.0;
        nav.place(&net, 0, node_ind, 3, 0.25);
        assert!((nav.position.x - sign * 6.0).abs() < 1e-5, "without a trail the requested lane is unchanged");
        nav.enable_trail(&net);
        let cookie = nav.cookie(&net);
        // Road edges are +/-8 m. The unchanged half-width + 1.5 m safety margin gives +/-5.5 m bounds.
        // The centre is another 0.1 m inside the outer bound, at 5.4 m right of travel.
        assert!((nav.position.x - sign * 5.4).abs() < 1e-5, "trail centre must respect its bounds: {nav:?}");
        assert!((cookie.right_offset - 0.1).abs() < 1e-5, "the centre has a separate 0.1 m inset: {cookie:?}");
        assert!((cookie.left_offset + 10.9).abs() < 1e-5, "the 1.5 m safety margin must remain intact: {cookie:?}");
        assert_eq!(nav.trail().unwrap().cookies()[0], cookie, "the first cookie uses the rebuilt centre");
    }
}

#[test]
fn trail_geometry_taper_bounds_interpolate_between_independent_endpoint_profiles() {
    let net = taper();
    let mut nav = navigator();
    nav.half_width = 1.0;
    nav.place(&net, 0, 1, 2, 0.0);
    nav.enable_trail(&net);
    for t in [0.0, 0.25, 0.75, 1.0] {
        nav.place(&net, 0, 1, 2, t);
        let cookie = nav.cookie(&net);
        // Start edges +/-8 m and end edges +/-12 m each keep the same 2.5 m margin.
        let edge = 5.5 + 4.0 * t;
        assert!((cookie.left.x + edge).abs() < 1e-4, "left bound at t={t}: {cookie:?}");
        assert!((cookie.right.x - edge).abs() < 1e-4, "right bound at t={t}: {cookie:?}");
        assert!((nav.position.x - (2.0 + t)).abs() < 1e-4, "the inner lane stays inside both endpoint bounds");
    }
}

#[test]
fn trail_geometry_taper_does_not_switch_its_bounds_at_the_segment_midpoint() {
    let net = taper();
    let mut nav = navigator();
    nav.place(&net, 0, 1, 2, 0.4999);
    nav.enable_trail(&net);
    let before = nav.cookie(&net);
    nav.place(&net, 0, 1, 2, 0.5001);
    let after = nav.cookie(&net);
    assert!((after.left.x - before.left.x).abs() < 0.01, "left edge must be continuous: {before:?} -> {after:?}");
    assert!((after.right.x - before.right.x).abs() < 0.01, "right edge must be continuous: {before:?} -> {after:?}");
    assert!((after.centre - before.centre).dot(Vec3::Z) > 0.0, "the cursor still advances along the segment");
}
