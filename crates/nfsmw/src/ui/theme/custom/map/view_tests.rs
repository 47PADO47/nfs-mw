//! Tests for the map's projection: north-up and heading-up, the scale, the ellipse and pinning to its edge.

use blackbox_ui::Clip;
use glam::Vec2;

use super::view::{FADE_SHARE, MapView, PIN_SHARE, ellipse, pin};

fn close(a: Vec2, b: Vec2) {
    assert!((a - b).length() < 1e-3, "{a} versus {b}");
}

#[test]
fn north_up_puts_north_up_and_east_right_whatever_the_heading() {
    // The car at (100, 50) heading west; two units per metre round (10, 20).
    let view = MapView::new([100.0, 50.0], [-1.0, 0.0], false, 2.0, Vec2::new(10.0, 20.0));
    close(view.to_screen([100.0, 50.0]), Vec2::new(10.0, 20.0));
    close(view.to_screen([100.0, 60.0]), Vec2::new(10.0, 0.0));
    close(view.to_screen([110.0, 50.0]), Vec2::new(30.0, 20.0));
    close(view.direction([-1.0, 0.0]), Vec2::NEG_X);
    close(view.direction([0.0, 3.0]), Vec2::NEG_Y);
}

#[test]
fn heading_up_turns_the_heading_to_the_top_and_the_left_to_the_left() {
    // Render space: x forward, y left. A car heading +x sees +x ahead (up) and +y on its left.
    let view = MapView::new([0.0, 0.0], [1.0, 0.0], true, 1.0, Vec2::ZERO);
    close(view.to_screen([10.0, 0.0]), Vec2::new(0.0, -10.0));
    close(view.to_screen([0.0, 10.0]), Vec2::new(-10.0, 0.0));
    close(view.direction([1.0, 0.0]), Vec2::NEG_Y);
    // Heading north (+y): the world's west (-x) is on the left.
    let north = MapView::new([0.0, 0.0], [0.0, 2.0], true, 1.0, Vec2::ZERO);
    close(north.to_screen([0.0, 5.0]), Vec2::new(0.0, -5.0));
    close(north.to_screen([-5.0, 0.0]), Vec2::new(-5.0, 0.0));
    // Heading south-east: whatever the heading, it points up.
    let any = MapView::new([3.0, 4.0], [1.0, -1.0], true, 1.0, Vec2::ZERO);
    close(any.direction([1.0, -1.0]), Vec2::NEG_Y);
}

#[test]
fn a_heading_of_no_length_points_up() {
    let view = MapView::new([0.0, 0.0], [0.0, 0.0], true, 1.0, Vec2::ZERO);
    close(view.direction([0.0, 0.0]), Vec2::NEG_Y);
}

#[test]
fn the_overlay_is_an_ellipse_with_a_soft_edge() {
    let (clip, band) = ellipse(Vec2::ZERO, Vec2::new(200.0, 100.0));
    assert_eq!(clip, Clip::Ellipse { centre: Vec2::ZERO, radii: Vec2::new(200.0, 100.0) });
    assert!((band - FADE_SHARE * 150.0).abs() < 1e-3);
    // The ellipse's box corner is outside it.
    assert!(!clip.contains(Vec2::new(190.0, 90.0)));
    assert!(clip.contains(Vec2::new(150.0, 0.0)));
}

#[test]
fn a_pinned_mark_is_held_inside_the_edge() {
    let radii = Vec2::new(200.0, 100.0);
    close(pin(Vec2::new(50.0, 10.0), Vec2::ZERO, radii), Vec2::new(50.0, 10.0));
    close(pin(Vec2::new(1000.0, 0.0), Vec2::ZERO, radii), Vec2::new(200.0 * PIN_SHARE, 0.0));
    close(pin(Vec2::new(0.0, -500.0), Vec2::ZERO, radii), Vec2::new(0.0, -100.0 * PIN_SHARE));
}
