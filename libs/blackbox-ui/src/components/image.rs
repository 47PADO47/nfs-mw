//! `Image`: one textured quad over a UV rectangle, filling its `Rect`. It emits one `Image` node.

use super::{DEFAULT_Z, FALLBACK_COLOUR, FULL_UV, UNROTATED, node_at};
use crate::component::Component;
use crate::context::LayoutCx;
use crate::draw::DrawList;
use crate::geom::Rect;
use crate::tree::NodeKind;

/// A skin texture drawn over the whole rectangle. The rectangle sets the size on screen, so a caller that wants an
/// aspect-preserving fit sizes the rectangle itself. The UV rectangle picks the part of the texture to show.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    texture_key: String,
    uv: [f32; 4],
    tint: [u8; 4],
    z: f32,
    rect: Rect,
    texture: u32,
}

impl Image {
    /// The skin texture `texture_key`, showing all of it. A key the skin does not have draws nothing (texture 0).
    pub fn new(texture_key: impl Into<String>) -> Self {
        Self {
            texture_key: texture_key.into(),
            uv: FULL_UV,
            tint: FALLBACK_COLOUR,
            z: DEFAULT_Z,
            rect: Rect::ZERO,
            texture: 0,
        }
    }

    /// Shows only the part of the texture `[u0, v0, u1, v1]`.
    pub fn with_uv(mut self, uv: [f32; 4]) -> Self {
        self.uv = uv;
        self
    }

    /// Sets the node colour `[r, g, b, a]`. The default is opaque white, which leaves the texture unchanged.
    pub fn with_tint(mut self, rgba: [u8; 4]) -> Self {
        self.tint = rgba;
        self
    }

    pub fn with_z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }
}

impl Component for Image {
    fn layout(&mut self, cx: &LayoutCx<'_>, bounds: Rect) {
        self.rect = bounds;
        self.texture = cx.skin.texture(&self.texture_key).unwrap_or(0);
    }

    fn build(&self, out: &mut DrawList) {
        let kind = NodeKind::Image {
            texture: self.texture,
            uv: self.uv,
            mask: None,
            mask_rotation: UNROTATED,
            mask_uv: FULL_UV,
        };
        out.push(node_at(self.rect.world(self.z), self.z, self.tint, kind));
    }
}

#[cfg(test)]
mod tests {
    use super::Image;
    use crate::components::testing::{drawn, lay_out};
    use crate::geom::Rect;
    use crate::skin::Skin;
    use crate::tree::NodeKind;
    use glam::Vec3;

    #[test]
    fn image_fills_its_rect_with_the_skin_texture_and_uv() {
        let skin = Skin::new().with_texture("icon", 9);
        let mut img = Image::new("icon").with_uv([0.0, 0.0, 0.5, 1.0]).with_tint([200, 100, 50, 128]);
        lay_out(&mut img, &skin, Rect::new(-30.0, 40.0, 64.0, 32.0));
        let out = drawn(&img);
        assert_eq!(out.len(), 1);
        let node = out.iter().next().expect("one node");
        assert_eq!(
            node.kind,
            NodeKind::Image {
                texture: 9,
                uv: [0.0, 0.0, 0.5, 1.0],
                mask: None,
                mask_rotation: [0.5, 0.5, 0.0],
                mask_uv: [0.0, 0.0, 1.0, 1.0],
            }
        );
        assert_eq!((node.colour, node.world_colour), ([200, 100, 50, 128], [200, 100, 50, 128]));
        assert_eq!(node.world.w_axis.truncate(), Vec3::new(-30.0, 40.0, 1.0));
        assert_eq!((node.world.x_axis.x, node.world.y_axis.y), (64.0, 32.0));
    }

    #[test]
    fn unknown_texture_key_draws_with_texture_zero() {
        let mut img = Image::new("missing");
        lay_out(&mut img, &Skin::new(), Rect::SCREEN);
        let out = drawn(&img);
        assert!(matches!(out.iter().next().map(|n| &n.kind), Some(NodeKind::Image { texture: 0, .. })));
    }
}
