//! `Slant`: a parallelogram whose opacity ramps along its width. The presenter draws quads with one colour each, so the
//! ramp is a row of thin strips, each a parallelogram of the same lean, so their slanted ends meet without a seam.
//!
//! It holds no state of its own: [`Slant::new`] lays it out against the skin, so `layout` is a no-op.

use glam::{Mat4, Vec4};

use super::{DEFAULT_Z, FALLBACK_COLOUR, FULL_UV, UNROTATED, node_at};
use crate::component::Component;
use crate::context::LayoutCx;
use crate::draw::DrawList;
use crate::geom::Rect;
use crate::tree::NodeKind;

/// How many strips the width is cut into. More strips follow the ramp more closely.
const STRIPS: usize = 32;

/// One strip: its transform and colour.
struct Strip {
    world: Mat4,
    colour: [u8; 4],
}

/// A parallelogram in one skin colour, with an opacity that follows `stops` from its left end to its right end.
pub struct Slant {
    z: f32,
    strips: Vec<Strip>,
}

impl Slant {
    /// The parallelogram over `rect`, in the skin colour `colour`. `lean` is how far the top edge sits to the right of
    /// the bottom edge, in FEng units (negative leans the other way); `rect` spans the middle height of the shape.
    /// `stops` are `(position, opacity)` pairs, position 0 at the left end and 1 at the right end, in increasing
    /// position; the opacity multiplies the colour's own alpha and is held flat beyond the first and last stop. With no
    /// stops the shape is fully opaque.
    pub fn new(
        colour: impl AsRef<str>,
        rect: Rect,
        lean: f32,
        stops: &[(f32, f32)],
        z: f32,
        cx: &LayoutCx<'_>,
    ) -> Self {
        let base = cx.skin.colour(colour.as_ref()).unwrap_or(FALLBACK_COLOUR);
        let width = rect.w / STRIPS as f32;
        let strips = (0..STRIPS)
            .map(|k| {
                let mid = (k as f32 + 0.5) / STRIPS as f32;
                let alpha = (f32::from(base[3]) * opacity_at(stops, mid)).round().clamp(0.0, 255.0) as u8;
                let x = rect.min_x() + (k as f32 + 0.5) * width;
                // The unit quad's y axis carries the lean: the top corner (y = -0.5) moves right by `lean / 2`.
                let world = Mat4::from_cols(
                    Vec4::new(width, 0.0, 0.0, 0.0),
                    Vec4::new(-lean, rect.h, 0.0, 0.0),
                    Vec4::Z,
                    Vec4::new(x, rect.y, z, 1.0),
                );
                Strip { world, colour: [base[0], base[1], base[2], alpha] }
            })
            .collect();
        Self { z, strips }
    }

    /// The same shape at the default depth.
    pub fn at_default_depth(
        colour: impl AsRef<str>,
        rect: Rect,
        lean: f32,
        stops: &[(f32, f32)],
        cx: &LayoutCx<'_>,
    ) -> Self {
        Self::new(colour, rect, lean, stops, DEFAULT_Z, cx)
    }
}

/// The opacity at `t` along the stops, linear between them and flat outside them.
pub(super) fn opacity_at(stops: &[(f32, f32)], t: f32) -> f32 {
    let (Some(first), Some(last)) = (stops.first(), stops.last()) else { return 1.0 };
    if t <= first.0 {
        return first.1;
    }
    for pair in stops.windows(2) {
        let ((t0, a0), (t1, a1)) = (pair[0], pair[1]);
        if t <= t1 {
            return a0 + (a1 - a0) * (t - t0) / (t1 - t0).max(f32::EPSILON);
        }
    }
    last.1
}

impl Component for Slant {
    /// The slant is laid out in [`Slant::new`], so there is nothing to place here.
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        for strip in &self.strips {
            let kind =
                NodeKind::Image { texture: 0, uv: FULL_UV, mask: None, mask_rotation: UNROTATED, mask_uv: FULL_UV };
            out.push(node_at(strip.world, self.z, strip.colour, kind));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{STRIPS, Slant, opacity_at};
    use crate::component::Component;
    use crate::context::LayoutCx;
    use crate::draw::DrawList;
    use crate::geom::Rect;
    use crate::skin::Skin;
    use glam::Vec3;

    fn built(slant: &Slant) -> DrawList {
        let mut out = DrawList::new();
        slant.build(&mut out);
        out
    }

    #[test]
    fn the_opacity_is_linear_between_stops_and_flat_outside_them() {
        let stops = [(0.2, 0.0), (0.6, 1.0)];
        assert_eq!(opacity_at(&stops, 0.0), 0.0);
        assert!((opacity_at(&stops, 0.4) - 0.5).abs() < 1e-6);
        assert_eq!(opacity_at(&stops, 0.9), 1.0);
        assert_eq!(opacity_at(&[], 0.5), 1.0);
    }

    #[test]
    fn the_strips_tile_the_width_and_ramp_the_alpha() {
        let skin = Skin::new().with_colour("bar", [200, 100, 50, 200]);
        let cx = LayoutCx::new(&skin, 0.0);
        let slant = Slant::new("bar", Rect::new(10.0, 0.0, 320.0, 30.0), 12.0, &[(0.0, 0.0), (1.0, 1.0)], 2.0, &cx);
        let out = built(&slant);
        assert_eq!(out.len(), STRIPS);
        let nodes: Vec<_> = out.iter().collect();
        assert!(nodes[0].colour[3] < nodes[STRIPS - 1].colour[3], "the ramp rises to the right");
        assert_eq!(nodes[0].colour[..3], [200, 100, 50]);
        // The first strip starts at the left edge, the last ends at the right edge.
        assert!((nodes[0].world.w_axis.x - (-150.0 + 5.0)).abs() < 1e-3);
        assert!((nodes[STRIPS - 1].world.w_axis.x - (170.0 - 5.0)).abs() < 1e-3);
        assert_eq!(nodes[0].z, 2.0);
    }

    #[test]
    fn the_top_edge_leans_right_of_the_bottom_edge() {
        let skin = Skin::new();
        let cx = LayoutCx::new(&skin, 0.0);
        let slant = Slant::new("x", Rect::new(0.0, 0.0, 64.0, 30.0), 12.0, &[], 1.0, &cx);
        let out = built(&slant);
        let node = out.iter().next().expect("a strip");
        let corner = |y: f32| node.world.transform_point3(Vec3::new(0.0, y, 0.0)).x - node.world.w_axis.x;
        assert!((corner(-0.5) - 6.0).abs() < 1e-4, "the top corner moves right by half the lean");
        assert!((corner(0.5) + 6.0).abs() < 1e-4, "the bottom corner moves left by half the lean");
    }
}
