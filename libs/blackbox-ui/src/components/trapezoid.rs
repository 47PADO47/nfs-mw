//! `Trapezoid`: a right trapezoid whose opacity ramps along its width: a horizontal top and bottom, a vertical right edge and a
//! left edge that slants in. The presenter draws quads with one colour each, so the shape is a row of thin upright columns, each
//! as tall as the slanted edge leaves it, and the ramp is a different alpha per column.
//!
//! It holds no state of its own: [`Trapezoid::new`] lays it out against the skin, so `layout` is a no-op.

use glam::Mat4;

use super::slant::opacity_at;
use super::{DEFAULT_Z, FALLBACK_COLOUR, FULL_UV, UNROTATED, node_at};
use crate::component::Component;
use crate::context::LayoutCx;
use crate::draw::DrawList;
use crate::geom::Rect;
use crate::tree::NodeKind;

/// The width of a column along the slanted edge, in FEng units: narrow, so the slant reads as a straight edge.
const SLANT_COLUMN: f32 = 0.5;
/// How many columns the rest of the width is cut into, for the ramp.
const FLAT_COLUMNS: usize = 40;

/// One column: its transform and colour.
struct Column {
    world: Mat4,
    colour: [u8; 4],
}

/// A right trapezoid in one skin colour, with an opacity that follows `stops` from its left end to its right end.
pub struct Trapezoid {
    z: f32,
    columns: Vec<Column>,
}

impl Trapezoid {
    /// The trapezoid over `rect`: the top edge spans all of it, and the bottom edge starts `slant` units in from the left, so
    /// the left edge runs from the top-left corner down and right (a negative `slant` slants the other way, from the
    /// bottom-left corner up). `stops` are `(position, opacity)` pairs, position 0 at the left end and 1 at the right end;
    /// the opacity multiplies the colour's own alpha. With no stops the shape is opaque.
    pub fn new(
        colour: impl AsRef<str>,
        rect: Rect,
        slant: f32,
        stops: &[(f32, f32)],
        z: f32,
        cx: &LayoutCx<'_>,
    ) -> Self {
        let base = cx.skin.colour(colour.as_ref()).unwrap_or(FALLBACK_COLOUR);
        let reach = slant.abs().min(rect.w);
        let steps = (reach / SLANT_COLUMN).ceil() as usize;
        let mut edges: Vec<f32> = (0..steps).map(|k| k as f32 * SLANT_COLUMN).collect();
        let flat = rect.w - reach;
        edges.extend((0..=FLAT_COLUMNS).map(|k| reach + flat * k as f32 / FLAT_COLUMNS as f32));
        let columns = edges
            .windows(2)
            .filter(|pair| pair[1] > pair[0])
            .map(|pair| {
                let (from, to) = (pair[0], pair[1]);
                let middle = (from + to) / 2.0;
                // The part of the height the slanted edge leaves at this column: none at the corner, all of it by `reach`.
                let share = if reach > 0.0 { (middle / reach).min(1.0) } else { 1.0 };
                let height = rect.h * share;
                let alpha = (f32::from(base[3]) * opacity_at(stops, middle / rect.w)).round().clamp(0.0, 255.0) as u8;
                // A positive slant keeps the top and cuts the bottom; a negative one keeps the bottom and cuts the top.
                let y = if slant >= 0.0 { rect.min_y() + height / 2.0 } else { rect.max_y() - height / 2.0 };
                let world = Mat4::from_cols_array(&[
                    to - from,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    height,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                    0.0,
                    rect.min_x() + middle,
                    y,
                    z,
                    1.0,
                ]);
                Column { world, colour: [base[0], base[1], base[2], alpha] }
            })
            .collect();
        Self { z, columns }
    }

    /// The same shape at the default depth.
    pub fn at_default_depth(
        colour: impl AsRef<str>,
        rect: Rect,
        slant: f32,
        stops: &[(f32, f32)],
        cx: &LayoutCx<'_>,
    ) -> Self {
        Self::new(colour, rect, slant, stops, DEFAULT_Z, cx)
    }
}

impl Component for Trapezoid {
    /// The trapezoid is laid out in [`Trapezoid::new`], so there is nothing to place here.
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        for column in &self.columns {
            let kind =
                NodeKind::Image { texture: 0, uv: FULL_UV, mask: None, mask_rotation: UNROTATED, mask_uv: FULL_UV };
            out.push(node_at(column.world, self.z, column.colour, kind));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::Skin;

    /// A column on the canvas: its left and right, top and bottom, and colour.
    type Span = (f32, f32, f32, f32, [u8; 4]);

    /// A plate 200 wide and 60 tall whose top-left corner is the canvas origin's (0, 20): its columns.
    fn built(slant: f32, stops: &[(f32, f32)]) -> Vec<Span> {
        let skin = Skin::new().with_colour("plate", [20, 30, 40, 200]);
        let rect = Rect::new(100.0, 50.0, 200.0, 60.0);
        let plate = Trapezoid::new("plate", rect, slant, stops, 3.0, &LayoutCx::new(&skin, 0.0));
        let mut out = DrawList::new();
        plate.build(&mut out);
        out.iter()
            .map(|n| {
                let (w, h) = (n.world.x_axis.x, n.world.y_axis.y);
                let (x, y) = (n.world.w_axis.x, n.world.w_axis.y);
                (x - w / 2.0, x + w / 2.0, y - h / 2.0, y + h / 2.0, n.colour)
            })
            .collect()
    }

    #[test]
    fn the_columns_tile_the_width_with_no_gap_and_end_on_the_right_edge() {
        let columns = built(18.0, &[]);
        assert!((columns[0].0 - 0.0).abs() < 1e-3, "the first column starts at the left edge");
        assert!((columns.last().expect("columns").1 - 200.0).abs() < 1e-3, "the last ends at the right edge");
        for pair in columns.windows(2) {
            assert!((pair[0].1 - pair[1].0).abs() < 1e-3, "no gap between columns");
        }
    }

    #[test]
    fn the_top_is_full_width_the_bottom_starts_after_the_slant_and_the_right_edge_is_upright() {
        let columns = built(18.0, &[]);
        assert!(columns.iter().all(|c| (c.2 - 20.0).abs() < 1e-3), "every column hangs from the top edge");
        assert!((columns.last().expect("columns").3 - 80.0).abs() < 1e-3, "full height at the right edge");
        assert!(columns[0].3 < 22.0, "next to nothing at the top-left corner");
        let first_full = columns.iter().find(|c| (c.3 - 80.0).abs() < 1e-3).expect("a full column");
        assert!((first_full.0 - 18.0).abs() < 0.6, "the bottom edge starts about `slant` in from the left");
        assert!(columns.windows(2).all(|p| p[0].3 <= p[1].3 + 1e-4), "the slanted edge is straight");
    }

    #[test]
    fn a_negative_slant_cuts_the_top_instead() {
        let columns = built(-18.0, &[]);
        assert!(columns.iter().all(|c| (c.3 - 80.0).abs() < 1e-3), "every column stands on the bottom edge");
        assert!(columns[0].2 > 78.0, "next to nothing at the bottom-left corner");
    }

    #[test]
    fn the_ramp_follows_the_stops_and_no_slant_is_a_rectangle() {
        let ramp = built(10.0, &[(0.0, 0.0), (1.0, 1.0)]);
        assert!(ramp[0].4[3] < ramp.last().expect("columns").4[3]);
        let flat = built(0.0, &[]);
        assert!(flat.iter().all(|c| (c.2 - 20.0).abs() < 1e-3 && (c.3 - 80.0).abs() < 1e-3));
    }
}
