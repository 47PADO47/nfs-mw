//! `Panel`: a solid or textured rectangle. It emits one `Image` node.

use super::{DEFAULT_Z, FALLBACK_COLOUR, FULL_UV, UNROTATED, node_at};
use crate::component::Component;
use crate::context::LayoutCx;
use crate::draw::DrawList;
use crate::geom::Rect;
use crate::tree::NodeKind;

/// A rectangle filled with a skin colour, optionally textured. It always emits one `NodeKind::Image` node. With no
/// texture the texture key is 0 (none), so the node draws the colour alone. The node carries the colour either way.
#[derive(Clone, Debug, PartialEq)]
pub struct Panel {
    colour_key: String,
    texture_key: Option<String>,
    z: f32,
    rect: Rect,
    colour: [u8; 4],
    texture: u32,
}

impl Panel {
    /// A panel filled with the skin colour `colour_key`. A key the skin does not have falls back to opaque white.
    pub fn new(colour_key: impl Into<String>) -> Self {
        Self {
            colour_key: colour_key.into(),
            texture_key: None,
            z: DEFAULT_Z,
            rect: Rect::ZERO,
            colour: FALLBACK_COLOUR,
            texture: 0,
        }
    }

    /// Draws the skin texture `texture_key` on the panel. A key the skin does not have leaves the panel untextured.
    pub fn textured(mut self, texture_key: impl Into<String>) -> Self {
        self.texture_key = Some(texture_key.into());
        self
    }

    pub fn with_z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }
}

impl Component for Panel {
    fn layout(&mut self, cx: &LayoutCx<'_>, bounds: Rect) {
        self.rect = bounds;
        self.colour = cx.skin.colour(&self.colour_key).unwrap_or(FALLBACK_COLOUR);
        self.texture = self.texture_key.as_deref().and_then(|key| cx.skin.texture(key)).unwrap_or(0);
    }

    fn build(&self, out: &mut DrawList) {
        let kind = NodeKind::Image {
            texture: self.texture,
            uv: FULL_UV,
            mask: None,
            mask_rotation: UNROTATED,
            mask_uv: FULL_UV,
        };
        out.push(node_at(self.rect.world(self.z), self.z, self.colour, kind));
    }
}

#[cfg(test)]
mod tests {
    use super::Panel;
    use crate::components::testing::{drawn, lay_out};
    use crate::geom::Rect;
    use crate::skin::Skin;
    use crate::tree::NodeKind;
    use glam::Vec3;

    #[test]
    fn solid_panel_emits_one_untextured_image_in_its_colour() {
        let skin = Skin::new().with_colour("panel", [10, 20, 30, 255]);
        let mut p = Panel::new("panel");
        lay_out(&mut p, &skin, Rect::new(5.0, -6.0, 80.0, 30.0));
        let out = drawn(&p);
        assert_eq!(out.len(), 1);
        let node = out.iter().next().expect("one node");
        assert!(matches!(node.kind, NodeKind::Image { texture: 0, mask: None, .. }));
        assert_eq!((node.colour, node.world_colour, node.visible), ([10, 20, 30, 255], [10, 20, 30, 255], true));
        assert_eq!(node.world.w_axis.truncate(), Vec3::new(5.0, -6.0, 1.0));
        assert_eq!((node.world.x_axis.x, node.world.y_axis.y), (80.0, 30.0));
    }

    #[test]
    fn textured_panel_carries_the_skin_texture_and_falls_back_to_white_for_a_missing_colour() {
        let skin = Skin::new().with_texture("card", 9);
        let mut p = Panel::new("missing").textured("card").with_z(4.0);
        lay_out(&mut p, &skin, Rect::SCREEN);
        let out = drawn(&p);
        let node = out.iter().next().expect("one node");
        assert!(matches!(node.kind, NodeKind::Image { texture: 9, uv: [0.0, 0.0, 1.0, 1.0], .. }));
        assert_eq!((node.colour, node.z), ([255, 255, 255, 255], 4.0));
    }
}
