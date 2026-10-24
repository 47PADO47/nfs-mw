//! Tests for the world-to-canvas projection, the edge arrows' ellipse, the label low-pass and the pixel snap
//! (`project.rs`).

use glam::{Vec2, Vec3};

use super::project::{LABEL_JUMP, PixelGrid, Placement, edge_point, edge_rect, follow, place};
use crate::hud::HudCamera;
use crate::ui::present::{FENG_HEIGHT, Screen};
use crate::ui::theme::custom::layout::{SAFE_INSET, window};
use crate::viewer::camera::Lens;

const SCREEN: Screen = Screen { width: 1600.0, height: 900.0, pixels_per_point: 1.0 };
/// A 32:9 window.
const SUPER_WIDE: Screen = Screen { width: 3840.0, height: 1080.0, pixels_per_point: 1.0 };

/// A hand-made camera at the origin looking along +x (render space: x forward, y left, z up) with a 90 degree view both
/// ways: clip = (-y, z, 0.1, x), so a point lands at ndc (-y / x, z / x).
fn known() -> HudCamera {
    let view_proj = [[0.0, 0.0, 0.0, 1.0], [-1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 0.1, 0.0]];
    HudCamera { view_proj, viewport: [1600.0, 900.0] }
}

/// The game's own lens at the origin looking along +x.
fn lens() -> HudCamera {
    let lens = Lens::look_at(Vec3::ZERO, Vec3::X, 60.0, 0.3, 16.0 / 9.0);
    HudCamera { view_proj: lens.view_proj().to_cols_array_2d(), viewport: [1600.0, 900.0] }
}

fn on_screen(placement: Option<Placement>) -> Vec2 {
    match placement {
        Some(Placement::OnScreen(at)) => at,
        other => panic!("expected on screen, got {other:?}"),
    }
}

fn off_screen(placement: Option<Placement>) -> Vec2 {
    match placement {
        Some(Placement::OffScreen(direction)) => direction,
        other => panic!("expected off screen, got {other:?}"),
    }
}

fn close(a: Vec2, b: Vec2) -> bool {
    a.distance(b) < 1e-2
}

#[test]
fn a_known_point_lands_where_the_known_matrix_sends_it() {
    let window = window(SCREEN);
    // ndc (0.5, 0.25): a quarter of the window's width right of the centre, an eighth of its height up.
    let at = on_screen(place(&known(), window, [10.0, -5.0, 2.5], 0.0));
    assert!(close(at, Vec2::new(window.w / 4.0, -FENG_HEIGHT / 8.0)), "{at:?}");
    let ahead = on_screen(place(&lens(), window, [50.0, 0.0, 0.0], 0.0));
    assert!(close(ahead, Vec2::ZERO), "straight ahead is the window's centre: {ahead:?}");
}

#[test]
fn a_point_behind_the_camera_is_never_on_screen_and_keeps_its_side() {
    let window = window(SCREEN);
    // Behind and to the right (render y is left): the divided ndc would put it on the left.
    let right = off_screen(place(&known(), window, [-10.0, -5.0, 0.0], 0.0));
    assert!(close(right, Vec2::X), "{right:?}");
    let left = off_screen(place(&known(), window, [-10.0, 5.0, 0.0], 0.0));
    assert!(close(left, -Vec2::X), "{left:?}");
    let straight = off_screen(place(&known(), window, [-10.0, 0.0, 0.0], 0.0));
    assert!(close(straight, Vec2::Y), "straight behind points down: {straight:?}");
    assert!(lens().project([-10.0, 0.0, 0.0]).is_none(), "the camera rejects it too");
}

#[test]
fn a_point_in_front_but_outside_the_view_points_toward_it() {
    let window = window(SCREEN);
    let right = off_screen(place(&known(), window, [10.0, -30.0, 0.0], 0.0));
    assert!(close(right, Vec2::X), "{right:?}");
    let up = off_screen(place(&known(), window, [10.0, 0.0, 30.0], 0.0));
    assert!(close(up, -Vec2::Y), "up the screen is negative y: {up:?}");
}

#[test]
fn a_point_within_the_margin_of_the_edge_counts_as_off_screen() {
    let window = window(SCREEN);
    // ndc x 0.98: about 8.5 units in from the right edge.
    let point = [10.0, -9.8, 0.0];
    assert!(matches!(place(&known(), window, point, 0.0), Some(Placement::OnScreen(_))));
    assert!(matches!(place(&known(), window, point, 12.0), Some(Placement::OffScreen(_))));
}

#[test]
fn a_camera_without_a_viewport_places_nothing() {
    let camera = HudCamera { viewport: [0.0, 0.0], ..known() };
    assert_eq!(place(&camera, window(SCREEN), [10.0, 0.0, 0.0], 0.0), None);
}

#[test]
fn edge_arrows_sit_on_the_inset_ellipse_inside_the_window_at_16_9_and_32_9() {
    for screen in [SCREEN, SUPER_WIDE] {
        let window = window(screen);
        let edge = edge_rect(window, 9.0);
        let safe = window.inset(SAFE_INSET * FENG_HEIGHT, SAFE_INSET * FENG_HEIGHT);
        let (a, b) = (edge.w / 2.0, edge.h / 2.0);
        for step in 0..24 {
            let angle = step as f32 * std::f32::consts::TAU / 24.0;
            let direction = Vec2::new(angle.cos(), angle.sin());
            let at = edge_point(edge, direction);
            let on = (at.x / a).powi(2) + (at.y / b).powi(2);
            assert!((on - 1.0).abs() < 1e-3, "{screen:?} {angle}: {at:?} is off the ellipse");
            assert!(safe.contains(at.x, at.y), "{screen:?} {angle}: {at:?} leaves the safe inset");
            assert!(at.normalize().dot(direction) > 0.999, "{screen:?} {angle}: the arrow is along its direction");
        }
    }
    // On 32:9 the arrows use the whole window, past the 16:9 content box.
    let right = edge_point(edge_rect(window(SUPER_WIDE), 9.0), Vec2::X);
    assert!(right.x > FENG_HEIGHT * 16.0 / 9.0 / 2.0, "{right:?}");
}

#[test]
fn the_label_low_pass_converges_on_its_target() {
    let target = Vec2::new(10.0, -4.0);
    let mut at = Vec2::ZERO;
    let mut gap = at.distance(target);
    for _ in 0..30 {
        at = follow(at, target, 1.0 / 60.0);
        let now = at.distance(target);
        assert!(now < gap, "every step comes closer");
        gap = now;
    }
    assert!(gap < 0.01, "half a second settles it: {gap}");
    let one = follow(Vec2::ZERO, Vec2::X, 0.05);
    assert!((one.x - (1.0 - (-1.0f32).exp())).abs() < 1e-4, "one time constant covers 63 %: {one:?}");
}

#[test]
fn the_label_low_pass_jumps_far_moves_and_holds_without_time() {
    let far = Vec2::new(LABEL_JUMP + 1.0, 0.0);
    assert_eq!(follow(Vec2::ZERO, far, 1.0 / 60.0), far, "a far move is not shake");
    assert_eq!(follow(Vec2::ZERO, Vec2::X, 0.0), Vec2::ZERO, "no time, no step");
}

#[test]
fn snapped_labels_sit_on_whole_pixels_and_move_less_than_half_a_pixel() {
    let screens = [
        SCREEN,
        Screen { width: 1601.0, height: 901.0, pixels_per_point: 1.0 },
        Screen { pixels_per_point: 2.0, ..SCREEN },
    ];
    for screen in screens {
        let grid = PixelGrid::of(screen);
        let half = grid.size / 2.0;
        for value in [Vec2::ZERO, Vec2::new(0.3, -0.7), Vec2::new(123.456, 78.9), Vec2::new(-200.1, -3.33)] {
            let snapped = grid.snap(value);
            let pixel = snapped * grid.per_unit + half;
            assert!((pixel - pixel.round()).abs().max_element() < 1e-3, "{screen:?} {value:?}: {pixel:?}");
            let moved = (snapped - value).abs().max_element() * grid.per_unit;
            assert!(moved <= 0.5 + 1e-3, "{screen:?} {value:?} moved {moved} px");
        }
    }
    let grid = PixelGrid::of(SCREEN);
    assert_eq!(grid.per_unit, 900.0 / 480.0, "480 units fill the height");
}
