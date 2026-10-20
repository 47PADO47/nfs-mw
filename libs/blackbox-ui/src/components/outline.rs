//! `Outline`: the border of a rounded rectangle, drawn in one skin colour. Like [`Rounded`](super::Rounded) it is a stack
//! of thin quads: a ring cut into strips, with two segments in each strip (one in the middle band, where the sides are
//! straight). The parts do not overlap, so a translucent colour blends the same as it would on one ring.
//!
//! It holds no state of its own: [`Outline::new`] lays it out against the skin, so `layout` is a no-op.

use super::{DEFAULT_Z, Panel};
use crate::component::Component;
use crate::context::LayoutCx;
use crate::draw::DrawList;
use crate::geom::Rect;

/// The height of one corner strip, in FEng units.
const STRIP: f32 = 0.5;

/// The border of a rounded rectangle.
pub struct Outline {
    parts: Vec<Panel>,
}

impl Outline {
    /// The ring of `thickness` FEng units along the edge of `rect`, with outer corners of `radius`. The radius is clamped
    /// to half the smaller side, and the thickness to the same bound.
    pub fn new(colour: impl Into<String>, rect: Rect, radius: f32, thickness: f32, cx: &LayoutCx<'_>) -> Self {
        let colour = colour.into();
        let (left, top, right, bottom) = (rect.min_x(), rect.min_y(), rect.max_x(), rect.max_y());
        let half = rect.w.min(rect.h) / 2.0;
        let r = radius.clamp(0.0, half);
        let t = thickness.clamp(0.0, half);
        let inner_r = (r - t).max(0.0);
        let mut parts = Vec::new();
        let mut push = |x0: f32, y0: f32, x1: f32, y1: f32| {
            if x1 <= x0 || y1 <= y0 {
                return;
            }
            let mut panel = Panel::new(colour.clone()).with_z(DEFAULT_Z);
            panel.layout(cx, Rect::new((x0 + x1) / 2.0, (y0 + y1) / 2.0, x1 - x0, y1 - y0));
            parts.push(panel);
        };
        // The corner bands, top and bottom: each strip sits between the outer circle and (below the border) the inner one.
        let strips = (r / STRIP).ceil().max(0.0) as usize;
        for k in 0..strips {
            let (y0, y1) = (k as f32 * STRIP, ((k + 1) as f32 * STRIP).min(r));
            let mid = (y0 + y1) / 2.0;
            let outer = r - circle_half_width(r, mid);
            for (from, to) in [(top + y0, top + y1), (bottom - y1, bottom - y0)] {
                match mid < t {
                    true => push(left + outer, from, right - outer, to),
                    false => {
                        let inner = t + inner_r - circle_half_width(inner_r, mid - t);
                        push(left + outer, from, left + inner, to);
                        push(right - inner, from, right - outer, to);
                    }
                }
            }
        }
        // Between the corner bands. A border thicker than the corner radius reaches below the bands, so the top and
        // bottom border bars run full width down to the border's own thickness; the sides fill the rest.
        let edge = r.max(t);
        push(left, top + r, right, top + edge);
        push(left, bottom - edge, right, bottom - r);
        push(left, top + edge, left + t, bottom - edge);
        push(right - t, top + edge, right, bottom - edge);
        Self { parts }
    }
}

impl Component for Outline {
    /// The outline is laid out in [`Outline::new`], so there is nothing to place here.
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// Half the width of a circle of radius `r` at `dy` from its top, measured at the strip's middle.
fn circle_half_width(r: f32, dy: f32) -> f32 {
    let from_centre = r - dy;
    (r * r - from_centre * from_centre).max(0.0).sqrt()
}

#[cfg(test)]
mod tests {
    use super::Outline;
    use crate::component::Component;
    use crate::context::LayoutCx;
    use crate::draw::DrawList;
    use crate::geom::Rect;
    use crate::skin::Skin;

    /// Whether any part of the outline covers the point.
    fn covers(outline: &Outline, x: f32, y: f32) -> bool {
        let mut out = DrawList::new();
        outline.build(&mut out);
        out.iter().any(|n| {
            let (w, h) = (n.world.x_axis.x, n.world.y_axis.y);
            (x - n.world.w_axis.x).abs() <= w / 2.0 && (y - n.world.w_axis.y).abs() <= h / 2.0
        })
    }

    fn outline(radius: f32, thickness: f32) -> Outline {
        let skin = Skin::new().with_colour("edge", [255; 4]);
        Outline::new("edge", Rect::new(0.0, 0.0, 40.0, 24.0), radius, thickness, &LayoutCx::new(&skin, 0.0))
    }

    #[test]
    fn the_ring_covers_the_border_and_not_the_inside() {
        let ring = outline(4.0, 1.5);
        assert!(covers(&ring, 0.0, -11.5), "the top border");
        assert!(covers(&ring, 0.0, 11.5), "the bottom border");
        assert!(covers(&ring, -19.5, 0.0), "the left border");
        assert!(covers(&ring, 19.5, 0.0), "the right border");
        assert!(!covers(&ring, 0.0, 0.0), "the middle is empty");
        assert!(!covers(&ring, -10.0, 8.0), "inside the border is empty");
    }

    #[test]
    fn the_corners_are_cut() {
        let ring = outline(6.0, 1.5);
        assert!(!covers(&ring, -19.9, -11.9), "the very corner is outside the rounded ring");
        assert!(!covers(&ring, -18.5, -11.5), "the top border starts inside the corner, not at the edge");
        assert!(covers(&ring, -14.0, -11.5), "the border runs on past the corner");
        assert!(covers(&ring, -19.0, -4.0), "and the side takes over below it");
    }

    #[test]
    fn a_zero_radius_ring_is_a_plain_frame() {
        let ring = outline(0.0, 1.0);
        assert!(covers(&ring, -19.9, -11.9), "a square ring reaches the corner");
        assert!(!covers(&ring, 0.0, 0.0));
    }
}
