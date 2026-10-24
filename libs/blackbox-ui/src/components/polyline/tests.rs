//! Tests of `Polyline` and `Polylines`: each is built into a draw list and checked node by node.

use glam::{Vec2, Vec3};

use super::{Clip, LineCap, LineJoin, Polyline, Polylines};
use crate::component::Component;
use crate::components::testing::drawn;
use crate::geom::Rect;
use crate::tree::{NodeKind, UiNode};

const COLOUR: [u8; 4] = [40, 80, 120, 200];

fn nodes(c: &dyn Component) -> Vec<UiNode> {
    drawn(c).iter().cloned().collect()
}

fn line(points: &[(f32, f32)], width: f32) -> Polyline {
    Polyline::new(points.iter().map(|&(x, y)| Vec2::new(x, y)).collect(), width, COLOUR)
}

/// The ends of a quad's centre line: where the middles of its short sides land.
fn ends(node: &UiNode) -> (Vec2, Vec2) {
    let at = |x: f32| node.world.transform_point3(Vec3::new(x, 0.0, 0.0)).truncate();
    (at(-0.5), at(0.5))
}

/// How wide a quad is across its centre line.
fn width_of(node: &UiNode) -> f32 {
    node.world.y_axis.truncate().length()
}

fn close(a: Vec2, b: Vec2) -> bool {
    a.distance(b) < 1e-3
}

#[test]
fn no_points_one_point_or_only_repeats_draw_nothing() {
    assert!(nodes(&line(&[], 2.0)).is_empty());
    assert!(nodes(&line(&[(5.0, 5.0)], 2.0)).is_empty());
    assert!(nodes(&line(&[(5.0, 5.0), (5.0, 5.0), (5.0, 5.0)], 2.0)).is_empty());
    assert!(nodes(&Polylines::new()).is_empty());
}

#[test]
fn a_zero_width_or_transparent_line_draws_nothing() {
    assert!(nodes(&line(&[(0.0, 0.0), (10.0, 0.0)], 0.0)).is_empty());
    let mut clear = line(&[(0.0, 0.0), (10.0, 0.0)], 2.0);
    clear.set_colour([255, 255, 255, 0]);
    assert!(nodes(&clear).is_empty());
}

#[test]
fn a_segment_is_one_untextured_rectangle_turned_along_it() {
    let built = nodes(&line(&[(0.0, 0.0), (30.0, 40.0)], 4.0).with_z(3.0));
    assert_eq!(built.len(), 1);
    let node = &built[0];
    assert!(matches!(node.kind, NodeKind::Image { texture: 0, mask: None, .. }));
    assert_eq!(node.colour, COLOUR);
    assert_eq!(node.world_colour, COLOUR);
    assert_eq!(node.z, 3.0);
    assert_eq!(node.world.w_axis.z, 3.0);
    assert!(node.visible);
    let (a, b) = ends(node);
    assert!(close(a, Vec2::ZERO) && close(b, Vec2::new(30.0, 40.0)), "{a} {b}");
    assert!((width_of(node) - 4.0).abs() < 1e-5);
    assert!(node.world.x_axis.truncate().dot(node.world.y_axis.truncate()).abs() < 1e-3, "a rectangle, not a slant");
}

#[test]
fn zero_length_segments_are_skipped() {
    let built = nodes(&line(&[(0.0, 0.0), (0.0, 0.0), (10.0, 0.0), (10.0, 0.0), (10.0, 0.0)], 2.0));
    assert_eq!(built.len(), 1);
    let (a, b) = ends(&built[0]);
    assert!(close(a, Vec2::ZERO) && close(b, Vec2::new(10.0, 0.0)), "{a} {b}");
}

#[test]
fn the_width_is_the_same_whatever_the_direction_or_length() {
    for k in 0..24 {
        let angle = (k as f32 * 15.0).to_radians();
        let tip = Vec2::from_angle(angle) * (1.0 + k as f32 * 7.0);
        let start = Vec2::new(5.0, -5.0);
        let built = nodes(&Polyline::new(vec![start, start + tip], 2.5, COLOUR));
        assert_eq!(built.len(), 1);
        assert!((width_of(&built[0]) - 2.5).abs() < 1e-4, "{angle}");
        assert!((built[0].size.y - 2.5).abs() < 1e-4, "{angle}");
    }
    // Every piece of a faded, mitred zigzag keeps the width too: a mitre adds no node of another shape.
    let zigzag = line(&[(-60.0, 0.0), (-20.0, 20.0), (20.0, -20.0), (60.0, 0.0)], 3.0)
        .with_join(LineJoin::Mitre { limit: 4.0 })
        .with_clip(Clip::Circle { centre: Vec2::ZERO, radius: 50.0 }, 15.0);
    let built = nodes(&zigzag);
    assert!(!built.is_empty());
    for node in &built {
        assert!((width_of(node) - 3.0).abs() < 1e-4, "{}", width_of(node));
    }
}

#[test]
fn a_bevel_adds_one_node_per_turn_and_a_mitre_none() {
    let zigzag = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (20.0, 10.0), (20.0, 20.0)];
    assert_eq!(nodes(&line(&zigzag, 2.0)).len(), 4 + 3);
    assert_eq!(nodes(&line(&zigzag, 2.0).with_join(LineJoin::Mitre { limit: 2.0 })).len(), 4);
    let straight = [(0.0, 0.0), (5.0, 0.0), (10.0, 0.0), (15.0, 0.0)];
    assert_eq!(nodes(&line(&straight, 2.0)).len(), 3, "points in a line need no join");
}

#[test]
fn a_mitre_runs_both_segments_on_to_the_corner() {
    let built = nodes(&line(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], 2.0).with_join(LineJoin::Mitre { limit: 2.0 }));
    assert_eq!(built.len(), 2);
    let (a, b) = ends(&built[0]);
    assert!(close(a, Vec2::ZERO) && close(b, Vec2::new(11.0, 0.0)), "{a} {b}");
    let (a, b) = ends(&built[1]);
    assert!(close(a, Vec2::new(10.0, -1.0)) && close(b, Vec2::new(10.0, 10.0)), "{a} {b}");
}

#[test]
fn a_bevel_sits_on_the_outer_side_of_the_turn() {
    let built = nodes(&line(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], 2.0));
    assert_eq!(built.len(), 3);
    // The bevel is built between the two segments, on the side the path turns away from: right of and above the corner.
    let centre = built[1].world.w_axis.truncate().truncate();
    assert!(centre.x > 10.0 && centre.y < 0.0, "{centre}");
    assert!(centre.distance(Vec2::new(10.0, 0.0)) < 1.0, "{centre}");
    assert!(width_of(&built[1]) < 2.0, "the bevel is no deeper than the half width");
}

#[test]
fn a_square_cap_runs_the_ends_on_by_half_the_width() {
    let built = nodes(&line(&[(0.0, 0.0), (10.0, 0.0)], 4.0).with_cap(LineCap::Square));
    assert_eq!(built.len(), 1);
    let (a, b) = ends(&built[0]);
    assert!(close(a, Vec2::new(-2.0, 0.0)) && close(b, Vec2::new(12.0, 0.0)), "{a} {b}");
}

#[test]
fn a_clip_cuts_segments_at_its_edge_and_drops_what_is_outside() {
    let circle = Clip::Circle { centre: Vec2::ZERO, radius: 50.0 };
    let crossing = nodes(&line(&[(-100.0, 0.0), (100.0, 0.0)], 2.0).with_cap(LineCap::Square).with_clip(circle, 0.0));
    assert_eq!(crossing.len(), 1);
    let (a, b) = ends(&crossing[0]);
    assert!(close(a, Vec2::new(-50.0, 0.0)) && close(b, Vec2::new(50.0, 0.0)), "a clipped end has no cap: {a} {b}");
    assert!(nodes(&line(&[(60.0, 60.0), (100.0, 0.0)], 2.0).with_clip(circle, 0.0)).is_empty());
    // The corner is outside the clip, so it gets no bevel; both segments are cut where they leave.
    let out_and_back = nodes(&line(&[(0.0, 0.0), (80.0, 0.0), (0.0, 10.0)], 2.0).with_clip(circle, 0.0));
    assert_eq!(out_and_back.len(), 2);
    for node in &out_and_back {
        let (a, b) = ends(node);
        assert!(a.length() <= 50.0 + 1e-3 && b.length() <= 50.0 + 1e-3, "{a} {b}");
    }
}

#[test]
fn the_fade_lowers_the_alpha_towards_the_edge() {
    let circle = Clip::Circle { centre: Vec2::ZERO, radius: 50.0 };
    let built = nodes(&line(&[(0.0, 0.0), (100.0, 0.0)], 2.0).with_clip(circle, 20.0));
    assert!(built.len() >= 4, "the band is cut into pieces: {}", built.len());
    assert_eq!(built[0].colour[3], COLOUR[3], "the part inside the band is at full opacity");
    // The pieces run outwards in order, end to end, each no more opaque than the one before.
    for pair in built.windows(2) {
        let (end, start) = (ends(&pair[0]).1, ends(&pair[1]).0);
        assert!(close(end, start), "the pieces meet: {end} {start}");
        assert!(pair[1].colour[3] <= pair[0].colour[3], "{} after {}", pair[1].colour[3], pair[0].colour[3]);
    }
    let last = built.last().expect("a piece");
    assert!(last.colour[3] < COLOUR[3] / 4, "{}", last.colour[3]);
    assert!(ends(last).1.x <= 50.0 + 1e-3);
    assert!(built.iter().all(|n| n.colour[..3] == COLOUR[..3]));
}

#[test]
fn a_bevel_takes_the_fade_at_its_joint() {
    // The corner is 5 units inside the right edge of the clip: half way into a 10-unit band.
    let rect = Clip::Rect(Rect::new(0.0, 0.0, 100.0, 100.0));
    let built = nodes(&line(&[(0.0, 0.0), (45.0, 0.0), (45.0, -20.0)], 2.0).with_clip(rect, 10.0));
    let bevels: Vec<&UiNode> = built.iter().filter(|n| width_of(n) < 1.9).collect();
    assert_eq!(bevels.len(), 1);
    assert_eq!(bevels[0].colour[3], COLOUR[3] / 2);
}

#[test]
fn many_lines_build_as_one_component_in_order() {
    let roads = Polylines::with_capacity(2)
        .with_line(line(&[(0.0, 0.0), (10.0, 0.0)], 2.0).with_z(2.0))
        .with_line(line(&[(0.0, 5.0), (10.0, 5.0), (10.0, 15.0)], 1.0).with_z(4.0));
    assert_eq!(roads.len(), 2);
    let built = nodes(&roads);
    assert_eq!(built.len(), 1 + 3);
    assert_eq!(built[0].z, 2.0);
    assert!(built[1..].iter().all(|n| n.z == 4.0));
}

#[test]
fn the_reserve_hint_wins_only_when_it_is_larger() {
    let mut roads = Polylines::new().with_line(line(&[(0.0, 0.0), (1.0, 0.0), (2.0, 1.0)], 1.0));
    assert_eq!(roads.node_estimate(), 4);
    roads = roads.with_reserve(100);
    assert_eq!(roads.node_estimate(), 100);
    roads.clear();
    assert!(roads.is_empty());
    assert_eq!(roads.lines().len(), 0);
    assert_eq!(roads.node_estimate(), 100);
}

#[test]
fn points_can_be_refilled_in_place() {
    let mut road = line(&[(0.0, 0.0), (10.0, 0.0)], 2.0);
    road.points_mut().clear();
    road.points_mut().extend([Vec2::ZERO, Vec2::new(0.0, 10.0), Vec2::new(10.0, 10.0)]);
    assert_eq!(road.points().len(), 3);
    assert_eq!(nodes(&road).len(), 3, "two segments and the bevel between them");
}
