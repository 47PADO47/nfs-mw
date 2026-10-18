//! A rounded `Panel` for the Custom cluster. The presenter draws quads only, so a rounded rectangle is a stack of thin
//! quads: a strip at each corner, inset to the circle's edge, and one full-height middle. The parts do not overlap, so a
//! translucent colour blends the same as it would on one rectangle.

use blackbox_ui::{Component, DrawList, LayoutCx, Panel, Rect};

/// The height of one corner strip, in FEng units. Smaller strips follow the curve more closely.
const STRIP: f32 = 0.5;

/// A rectangle with rounded corners of `radius`, drawn in one colour. It holds no state: `new` lays it out.
pub(super) struct Rounded {
    parts: Vec<Panel>,
}

impl Rounded {
    /// The rounded rectangle `rect` in the skin colour `colour`, with corners of `radius` FEng units. The radius is
    /// clamped to half the smaller side, so a thin bar comes out as a pill.
    pub(super) fn new(colour: &'static str, rect: Rect, radius: f32, cx: &LayoutCx<'_>) -> Self {
        let (left, top, right, bottom) = (rect.min_x(), rect.min_y(), rect.max_x(), rect.max_y());
        let r = radius.min(rect.w / 2.0).min(rect.h / 2.0).max(0.0);
        let mut parts = Vec::new();
        let mut push = |x0: f32, y0: f32, x1: f32, y1: f32| {
            if x1 <= x0 || y1 <= y0 {
                return;
            }
            let mut panel = Panel::new(colour);
            panel.layout(cx, from_edges(x0, y0, x1, y1));
            parts.push(panel);
        };
        // Top and bottom corners: strips from the edge in, each inset to the circle at its middle.
        let strips = (r / STRIP).ceil().max(0.0) as usize;
        for k in 0..strips {
            let y0 = k as f32 * STRIP;
            let y1 = ((k + 1) as f32 * STRIP).min(r);
            let inset = r - circle_half_width(r, (y0 + y1) / 2.0);
            push(left + inset, top + y0, right - inset, top + y1);
            push(left + inset, bottom - y1, right - inset, bottom - y0);
        }
        // The middle, between the corners, full width.
        push(left, top + r, right, bottom - r);
        Self { parts }
    }
}

impl Component for Rounded {
    /// The rounded panel is laid out in [`Rounded::new`], so there is nothing to place here.
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// Half the width of a circle of radius `r` at `dy` from its centre, measured down from the top of the corner.
/// `dy` runs from 0 at the top edge to `r` at the centre line, so the half width is `sqrt(r^2 - (r - dy)^2)`.
fn circle_half_width(r: f32, dy: f32) -> f32 {
    let from_centre = r - dy;
    (r * r - from_centre * from_centre).max(0.0).sqrt()
}

/// The rectangle with the given edges, as a `Rect` (which is centred).
fn from_edges(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
    Rect::new((x0 + x1) / 2.0, (y0 + y1) / 2.0, x1 - x0, y1 - y0)
}
