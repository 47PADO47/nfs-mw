//! `Gauge`: a dial filled by a rotating mask, with an optional needle. It generalises the tachometer mechanic: the
//! mask texture turns about a pivot by an angle that follows a fraction, and the rest of the dial shows through it.
//! It emits one `Image` node for the dial, plus one more `Image` node for the needle when it has one.

use glam::{Mat4, Quat, Vec3};

use super::{DEFAULT_Z, FALLBACK_COLOUR, FULL_UV, UNROTATED, node_at};
use crate::component::Component;
use crate::context::LayoutCx;
use crate::draw::DrawList;
use crate::geom::Rect;
use crate::tree::NodeKind;

/// A gauge. `base_key` is the dial texture (node `texture`). `fill_key` is the mask texture (node `mask`), which
/// turns about `pivot` by `fraction * sweep_deg` degrees (node `mask_rotation`). The mask shows the whole texture
/// (`mask_uv` is the unit square), as a gauge does. The needle, if set, is the `needle_key` texture drawn over the
/// whole rectangle and turned about its centre by the same angle.
///
/// The fraction is the input: `set_fraction` clamps it to 0..=1, and `build` reads it, so no new layout is needed
/// when it changes.
#[derive(Clone, Debug, PartialEq)]
pub struct Gauge {
    base_key: String,
    fill_key: String,
    needle_key: Option<String>,
    fraction: f32,
    pivot: [f32; 2],
    sweep_deg: f32,
    tint: [u8; 4],
    z: f32,
    rect: Rect,
    base: u32,
    fill: u32,
    needle: u32,
}

/// Maps a fraction to 0..=1. NaN becomes 0; infinities clamp to the ends.
fn clamp_fraction(f: f32) -> f32 {
    if f.is_nan() {
        return 0.0;
    }
    f.clamp(0.0, 1.0)
}

impl Gauge {
    /// A gauge with the dial texture `base_key` and the mask texture `fill_key`, both from the skin. It starts empty
    /// (fraction 0), turns its mask a full 360 degrees at fraction 1, and pivots the mask about the texture centre.
    pub fn new(base_key: impl Into<String>, fill_key: impl Into<String>) -> Self {
        Self {
            base_key: base_key.into(),
            fill_key: fill_key.into(),
            needle_key: None,
            fraction: 0.0,
            pivot: [0.5, 0.5],
            sweep_deg: 360.0,
            tint: FALLBACK_COLOUR,
            z: DEFAULT_Z,
            rect: Rect::ZERO,
            base: 0,
            fill: 0,
            needle: 0,
        }
    }

    /// Adds a needle drawn from the skin texture `needle_key`.
    pub fn with_needle(mut self, needle_key: impl Into<String>) -> Self {
        self.needle_key = Some(needle_key.into());
        self
    }

    /// Sets the starting fraction (clamped as in `set_fraction`).
    pub fn with_fraction(mut self, fraction: f32) -> Self {
        self.fraction = clamp_fraction(fraction);
        self
    }

    /// Sets the mask pivot `[x, y]` as fractions of the mask texture. The default is the centre `[0.5, 0.5]`.
    pub fn with_pivot(mut self, pivot: [f32; 2]) -> Self {
        self.pivot = pivot;
        self
    }

    /// Sets the angle that a full fraction (1.0) turns the mask through, in degrees. The default is 360.
    pub fn with_sweep_deg(mut self, sweep_deg: f32) -> Self {
        self.sweep_deg = sweep_deg;
        self
    }

    /// Sets the node colour `[r, g, b, a]`. The default is opaque white.
    pub fn with_tint(mut self, rgba: [u8; 4]) -> Self {
        self.tint = rgba;
        self
    }

    pub fn with_z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }

    /// Sets the fill fraction, clamped to 0..=1. NaN sets 0.
    pub fn set_fraction(&mut self, fraction: f32) {
        self.fraction = clamp_fraction(fraction);
    }

    /// The current fraction, 0..=1.
    pub fn fraction(&self) -> f32 {
        self.fraction
    }

    /// The angle the mask (and the needle) is turned through, in degrees: `fraction * sweep_deg`.
    pub fn angle_deg(&self) -> f32 {
        self.fraction * self.sweep_deg
    }
}

impl Component for Gauge {
    fn layout(&mut self, cx: &LayoutCx<'_>, bounds: Rect) {
        self.rect = bounds;
        self.base = cx.skin.texture(&self.base_key).unwrap_or(0);
        self.fill = cx.skin.texture(&self.fill_key).unwrap_or(0);
        self.needle = self.needle_key.as_deref().and_then(|key| cx.skin.texture(key)).unwrap_or(0);
    }

    fn build(&self, out: &mut DrawList) {
        let kind = NodeKind::Image {
            texture: self.base,
            uv: FULL_UV,
            mask: (self.fill != 0).then_some(self.fill),
            mask_rotation: [self.pivot[0], self.pivot[1], self.angle_deg()],
            mask_uv: FULL_UV,
        };
        out.push(node_at(self.rect.world(self.z), self.z, self.tint, kind));
        if self.needle_key.is_none() {
            return;
        }
        let turn = Quat::from_rotation_z(self.angle_deg().to_radians());
        let world = Mat4::from_translation(Vec3::new(self.rect.x, self.rect.y, self.z))
            * Mat4::from_quat(turn)
            * Mat4::from_scale(Vec3::new(self.rect.w, self.rect.h, 1.0));
        let needle = NodeKind::Image {
            texture: self.needle,
            uv: FULL_UV,
            mask: None,
            mask_rotation: UNROTATED,
            mask_uv: FULL_UV,
        };
        out.push(node_at(world, self.z, self.tint, needle));
    }
}

#[cfg(test)]
mod tests {
    use super::Gauge;
    use crate::components::testing::{drawn, lay_out};
    use crate::geom::Rect;
    use crate::skin::Skin;
    use crate::tree::NodeKind;

    fn skin() -> Skin {
        Skin::new().with_texture("dial", 3).with_texture("fill", 4).with_texture("needle", 5)
    }

    #[test]
    fn mask_rotation_follows_the_fraction() {
        let mut g = Gauge::new("dial", "fill");
        lay_out(&mut g, &skin(), Rect::new(0.0, 0.0, 100.0, 100.0));
        g.set_fraction(0.25);
        let out = drawn(&g);
        assert_eq!(out.len(), 1);
        let node = out.iter().next().expect("one node");
        assert_eq!(
            node.kind,
            NodeKind::Image {
                texture: 3,
                uv: [0.0, 0.0, 1.0, 1.0],
                mask: Some(4),
                mask_rotation: [0.5, 0.5, 90.0],
                mask_uv: [0.0, 0.0, 1.0, 1.0],
            }
        );
    }

    #[test]
    fn fraction_is_clamped_and_sweep_scales_the_angle() {
        let mut g = Gauge::new("dial", "fill").with_sweep_deg(270.0);
        lay_out(&mut g, &skin(), Rect::SCREEN);
        g.set_fraction(2.0);
        assert_eq!((g.fraction(), g.angle_deg()), (1.0, 270.0));
        g.set_fraction(-1.0);
        assert_eq!(g.angle_deg(), 0.0);
        g.set_fraction(f32::NAN);
        assert_eq!(g.fraction(), 0.0);
    }

    #[test]
    fn missing_fill_texture_leaves_the_mask_off() {
        let mut g = Gauge::new("dial", "missing");
        lay_out(&mut g, &skin(), Rect::SCREEN);
        let out = drawn(&g);
        assert!(matches!(out.iter().next().map(|n| &n.kind), Some(NodeKind::Image { mask: None, .. })));
    }

    #[test]
    fn needle_is_a_second_node_turned_by_the_same_angle() {
        let mut g = Gauge::new("dial", "fill").with_needle("needle").with_sweep_deg(90.0).with_fraction(1.0);
        lay_out(&mut g, &skin(), Rect::new(0.0, 0.0, 100.0, 50.0));
        let out = drawn(&g);
        assert_eq!(out.len(), 2);
        let needle = out.iter().nth(1).expect("needle node");
        assert!(matches!(needle.kind, NodeKind::Image { texture: 5, mask: None, .. }));
        // A 90 degree turn takes the x axis (length 100) onto the y axis.
        assert!(needle.world.x_axis.x.abs() < 1e-3);
        assert!((needle.world.x_axis.y - 100.0).abs() < 1e-3);
        assert!((needle.world.y_axis.length() - 50.0).abs() < 1e-3);
    }
}
