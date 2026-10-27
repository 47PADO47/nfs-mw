//! The HUD map's plate: a subtle, translucent ellipse in the theme's scrim colour behind the roads, so thin lines stay
//! readable over a bright ground. It is far lighter than the full-screen map's backdrop (`full_map/chrome.rs`), which dims
//! the whole scene: the two must not be mistaken for each other.
//!
//! The presenter draws quads, so the ellipse is a stack of horizontal strips, each as wide as the ellipse is at its height.
//! The plate stops [`INSET`] of the way into the soft edge the roads fade over, so it does not show past them.

use blackbox_ui::{Component, DrawList, LayoutCx, Panel, Rect, Skin, token_keys};
use glam::Vec2;

use super::draw::colour;
use super::view::FADE_SHARE;
use crate::ui::theme::custom::tokens::fade;

/// The plate's share of the theme's scrim alpha. The full map draws the scrim at 1.
pub const PLATE_SHARE: f32 = 0.35;
/// How many strips the ellipse is cut into.
pub const STRIPS: usize = 28;
/// The plate's depth: just behind the roads (`map::ROAD_Z`).
pub const PLATE_Z: f32 = 61.0;
/// How far into the soft edge the plate reaches, as a share of the edge's width.
pub const INSET: f32 = 0.5;
/// A strip is this much taller than its pitch, so neighbours overlap and leave no seam.
const OVERLAP: f32 = 0.4;
/// The key the strips' colour sits under in their own skin.
const KEY: &str = "map.plate";

/// The plate's strips as one component.
pub struct EllipsePlate(Vec<Panel>);

impl Component for EllipsePlate {
    /// Each strip is laid out where it is made.
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        for strip in &self.0 {
            strip.build(out);
        }
    }
}

/// The rectangles of the plate's strips for an ellipse round `centre` with half-axes `radii`: [`STRIPS`] rows, each as wide
/// as the ellipse at the row's middle. Empty for an ellipse with no size.
pub fn strip_rects(centre: Vec2, radii: Vec2) -> Vec<Rect> {
    if radii.x <= 0.0 || radii.y <= 0.0 || !radii.is_finite() {
        return Vec::new();
    }
    let pitch = 2.0 * radii.y / STRIPS as f32;
    (0..STRIPS)
        .map(|i| {
            let y = centre.y - radii.y + (i as f32 + 0.5) * pitch;
            let across = ((y - centre.y) / radii.y).abs().min(1.0);
            let half = radii.x * (1.0 - across * across).sqrt();
            Rect::new(centre.x, y, 2.0 * half, pitch + OVERLAP)
        })
        .collect()
}

/// The plate's half-axes for the ellipse `radii`: [`INSET`] of the way into the soft edge from the outside.
pub fn plate_radii(radii: Vec2) -> Vec2 {
    radii * (1.0 - FADE_SHARE * (1.0 - INSET))
}

/// The plate colour at `alpha` (the overlay's fade times the zoom's): the theme's scrim, [`PLATE_SHARE`] of its alpha.
pub fn plate_colour(cx: &LayoutCx<'_>, alpha: f32) -> [u8; 4] {
    fade(colour(cx, token_keys::SCRIM), PLATE_SHARE * alpha)
}

/// The plate behind an ellipse round `centre` with half-axes `radii`, at `alpha`. `None` when it would be invisible.
pub fn plate(centre: Vec2, radii: Vec2, alpha: f32, cx: &LayoutCx<'_>) -> Option<EllipsePlate> {
    let tint = plate_colour(cx, alpha);
    if tint[3] == 0 {
        return None;
    }
    let skin = Skin::new().with_colour(KEY, tint);
    let inner = LayoutCx::new(&skin, cx.time_s);
    let strips = strip_rects(centre, plate_radii(radii))
        .into_iter()
        .map(|rect| {
            let mut panel = Panel::new(KEY).with_z(PLATE_Z);
            panel.layout(&inner, rect);
            panel
        })
        .collect();
    Some(EllipsePlate(strips))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strips_trace_the_ellipse() {
        let rects = strip_rects(Vec2::new(10.0, -100.0), Vec2::new(100.0, 50.0));
        assert_eq!(rects.len(), STRIPS);
        let widest = rects.iter().map(|r| r.w).fold(0.0, f32::max);
        assert!(widest > 190.0 && widest <= 200.0, "{widest}");
        assert!(rects[0].w < 0.5 * widest && rects[STRIPS - 1].w < 0.5 * widest, "narrow at the top and bottom");
        assert!(rects.iter().all(|r| (r.x - 10.0).abs() < 1e-4), "centred across");
        assert!(rects[0].y < -100.0 && rects[STRIPS - 1].y > -100.0, "from the top down");
        assert!(strip_rects(Vec2::ZERO, Vec2::new(0.0, 5.0)).is_empty());
    }

    #[test]
    fn the_plate_stops_inside_the_ellipse() {
        let radii = Vec2::new(100.0, 50.0);
        let inner = plate_radii(radii);
        assert!(inner.x < radii.x && inner.x > radii.x * (1.0 - FADE_SHARE));
        assert!((inner.y / inner.x - 0.5).abs() < 1e-5, "the shape is kept");
    }
}
