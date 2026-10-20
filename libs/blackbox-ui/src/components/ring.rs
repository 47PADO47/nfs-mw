//! `Ring`: an arc of a circle drawn in one skin colour, as a ring, a gauge's sweep or a compass. The presenter draws quads
//! only, so the arc is a row of short rotated quads laid along it, a few degrees each. Angles are in degrees clockwise from
//! straight up (north), so 0 is the top, 90 the right, 180 the bottom.
//!
//! It holds no state of its own: [`Ring::new`] lays it out against the skin, so `layout` is a no-op.

use glam::{Mat4, Vec3};

use super::{DEFAULT_Z, FALLBACK_COLOUR, FULL_UV, UNROTATED, node_at};
use crate::component::Component;
use crate::context::LayoutCx;
use crate::draw::DrawList;
use crate::geom::Rect;
use crate::tree::{NodeKind, UiNode};

/// The most one quad turns: a quad per this many degrees, or fewer for a short arc. A little over a chord that stays within
/// about a tenth of a unit of the circle for rings up to 100 units across.
const STEP_DEGREES: f32 = 3.0;

/// An arc of a ring.
pub struct Ring {
    nodes: Vec<UiNode>,
}

impl Ring {
    /// The arc from `from` to `to` degrees (clockwise from up; `to` may be less than `from` for the other way) of the circle
    /// with this `centre` and `radius`, `thickness` wide (half of it inside the radius, half outside), in the skin colour
    /// `colour`. A full ring is `0.0` to `360.0`.
    pub fn new(
        colour: impl AsRef<str>,
        centre: (f32, f32),
        radius: f32,
        thickness: f32,
        from: f32,
        to: f32,
        cx: &LayoutCx<'_>,
    ) -> Self {
        Self::at_depth(colour, centre, radius, thickness, (from, to), DEFAULT_Z, cx)
    }

    /// As [`Ring::new`], at depth `z` (a lower `z` draws on top).
    pub fn at_depth(
        colour: impl AsRef<str>,
        centre: (f32, f32),
        radius: f32,
        thickness: f32,
        (from, to): (f32, f32),
        z: f32,
        cx: &LayoutCx<'_>,
    ) -> Self {
        let colour = cx.skin.colour(colour.as_ref()).unwrap_or(FALLBACK_COLOUR);
        let sweep = to - from;
        let count = (sweep.abs() / STEP_DEGREES).ceil().max(1.0) as usize;
        let step = sweep / count as f32;
        let nodes = (0..count)
            .map(|k| {
                let middle = (from + step * (k as f32 + 0.5)).to_radians();
                // The chord between the ends of the step, a little longer so neighbours overlap and leave no seam.
                let length = 2.0 * radius * (step.abs().to_radians() / 2.0).sin() + 0.4;
                let (sin, cos) = middle.sin_cos();
                let at = Vec3::new(centre.0 + radius * sin, centre.1 - radius * cos, z);
                // The quad's long side runs along the tangent: turn it by the angle clockwise from up.
                let world = Mat4::from_translation(at)
                    * Mat4::from_rotation_z(middle)
                    * Mat4::from_scale(Vec3::new(length, thickness, 1.0));
                let kind =
                    NodeKind::Image { texture: 0, uv: FULL_UV, mask: None, mask_rotation: UNROTATED, mask_uv: FULL_UV };
                node_at(world, z, colour, kind)
            })
            .collect();
        Self { nodes }
    }

    /// The bounding rectangle of the circle the arc is on, for a caller that wants to place something against it.
    pub fn bounds(centre: (f32, f32), radius: f32) -> Rect {
        Rect::new(centre.0, centre.1, radius * 2.0, radius * 2.0)
    }
}

impl Component for Ring {
    /// The ring is laid out in [`Ring::new`], so there is nothing to place here.
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        for node in &self.nodes {
            out.push(node.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::Skin;

    fn built(from: f32, to: f32) -> Vec<UiNode> {
        let skin = Skin::new().with_colour("ring", [9, 8, 7, 200]);
        let ring = Ring::new("ring", (10.0, -20.0), 50.0, 4.0, from, to, &LayoutCx::new(&skin, 0.0));
        let mut out = DrawList::new();
        ring.build(&mut out);
        out.iter().cloned().collect()
    }

    fn at(node: &UiNode) -> (f32, f32) {
        (node.world.w_axis.x, node.world.w_axis.y)
    }

    #[test]
    fn every_quad_sits_on_the_circle_at_its_angle() {
        for node in built(0.0, 360.0) {
            let (x, y) = at(&node);
            let distance = ((x - 10.0).powi(2) + (y + 20.0).powi(2)).sqrt();
            assert!((distance - 50.0).abs() < 0.1, "{distance}");
        }
    }

    #[test]
    fn zero_degrees_is_straight_up_and_ninety_is_to_the_right() {
        let up = built(0.0, 3.0);
        let (x, y) = at(&up[0]);
        assert!((x - 10.0 - 50.0 * 1.5_f32.to_radians().sin()).abs() < 1e-3 && y < -20.0 - 49.0, "{x} {y}");
        let right = built(88.5, 91.5);
        let (x, y) = at(&right[0]);
        assert!(x > 10.0 + 49.0 && (y + 20.0).abs() < 2.0, "{x} {y}");
    }

    #[test]
    fn a_quad_lies_along_the_tangent_and_is_as_thick_as_asked() {
        let right = built(88.5, 91.5);
        let node = &right[0];
        // At the right of the circle the tangent runs down the screen: the quad's long axis points down, its width is the
        // thickness across.
        assert!(node.world.x_axis.y.abs() > node.world.x_axis.x.abs(), "{:?}", node.world.x_axis);
        assert!((node.world.y_axis.length() - 4.0).abs() < 1e-4);
    }

    #[test]
    fn a_longer_arc_takes_more_quads_and_a_short_one_takes_one() {
        assert_eq!(built(0.0, 360.0).len(), 120);
        assert_eq!(built(10.0, 11.0).len(), 1);
        assert_eq!(built(90.0, 0.0).len(), 30, "an arc can run anticlockwise");
        assert_eq!(built(0.0, 0.0).len(), 1, "a zero sweep is one sliver, not none");
    }

    #[test]
    fn the_colour_comes_from_the_skin() {
        assert!(built(0.0, 30.0).iter().all(|n| n.colour == [9, 8, 7, 200]));
    }
}
