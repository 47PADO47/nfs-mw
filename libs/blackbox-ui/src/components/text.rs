//! `Text`: one string in one font, placed in its `Rect` with justification flags. It emits one `Text` node.

use glam::{Mat4, Vec2};

use super::{DEFAULT_Z, FALLBACK_COLOUR, node_at};
use crate::component::Component;
use crate::context::LayoutCx;
use crate::draw::DrawList;
use crate::geom::Rect;
use crate::tree::NodeKind;

/// Justification flags. They are the same bits as the FEng text style, so a value from a game package passes through
/// unchanged. `H_RIGHT` wins over `H_CENTRE`, and `V_BOTTOM` wins over `V_CENTRE`. With no flag, the text starts at
/// the left edge and the top edge (the smallest `y`). `WRAP` wraps the text at the width of the rectangle.
pub mod justify {
    pub const H_CENTRE: u32 = 1;
    pub const H_RIGHT: u32 = 2;
    pub const V_CENTRE: u32 = 4;
    pub const V_BOTTOM: u32 = 8;
    pub const WRAP: u32 = 0x10;
}

/// A string in a skin font. The text origin sits on the rectangle edge or centre that the justification names, and
/// the node's `max_width` is the rectangle width. The colour comes from the skin key `colour_key` (default `"text"`).
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    font_key: String,
    text: String,
    colour_key: String,
    justification: u32,
    leading: i32,
    z: f32,
    rect: Rect,
    font: u32,
    colour: [u8; 4],
}

impl Text {
    /// A string `text` in the skin font `font_key`. A key the skin does not have uses font 0.
    pub fn new(font_key: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            font_key: font_key.into(),
            text: text.into(),
            colour_key: "text".to_owned(),
            justification: 0,
            leading: 0,
            z: DEFAULT_Z,
            rect: Rect::ZERO,
            font: 0,
            colour: FALLBACK_COLOUR,
        }
    }

    /// Sets the justification flags (see [`justify`]).
    pub fn with_justification(mut self, flags: u32) -> Self {
        self.justification = flags;
        self
    }

    /// Sets the line spacing, in FEng units.
    pub fn with_leading(mut self, leading: i32) -> Self {
        self.leading = leading;
        self
    }

    /// Reads the colour from the skin key `key` instead of `"text"`.
    pub fn with_colour_key(mut self, key: impl Into<String>) -> Self {
        self.colour_key = key.into();
        self
    }

    pub fn with_z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }

    /// Replaces the string. The next `build` shows it; no new layout is needed, since the rectangle is unchanged.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}

/// The point the text origin lands on inside `rect`, for the given justification flags.
fn origin(rect: Rect, flags: u32) -> Vec2 {
    let x = match (flags & justify::H_RIGHT != 0, flags & justify::H_CENTRE != 0) {
        (true, _) => rect.max_x(),
        (false, true) => rect.x,
        (false, false) => rect.min_x(),
    };
    let y = match (flags & justify::V_BOTTOM != 0, flags & justify::V_CENTRE != 0) {
        (true, _) => rect.max_y(),
        (false, true) => rect.y,
        (false, false) => rect.min_y(),
    };
    Vec2::new(x, y)
}

impl Component for Text {
    fn layout(&mut self, cx: &LayoutCx<'_>, bounds: Rect) {
        self.rect = bounds;
        self.font = cx.skin.font(&self.font_key).unwrap_or(0);
        self.colour = cx.skin.colour(&self.colour_key).unwrap_or(FALLBACK_COLOUR);
    }

    fn build(&self, out: &mut DrawList) {
        let at = origin(self.rect, self.justification).extend(self.z);
        let kind = NodeKind::Text {
            font: self.font,
            justification: self.justification,
            leading: self.leading,
            max_width: self.rect.w.round() as i32,
        };
        let mut node = node_at(Mat4::from_translation(at), self.z, self.colour, kind);
        node.text = Some(self.text.clone());
        out.push(node);
    }
}

#[cfg(test)]
mod tests {
    use super::{Text, justify};
    use crate::components::testing::{drawn, lay_out};
    use crate::geom::Rect;
    use crate::skin::Skin;
    use crate::tree::NodeKind;
    use glam::Vec3;

    fn skin() -> Skin {
        Skin::new().with_font("body", 7).with_colour("text", [1, 2, 3, 255])
    }

    #[test]
    fn text_carries_the_font_key_string_and_skin_colour() {
        let mut t = Text::new("body", "Hello");
        lay_out(&mut t, &skin(), Rect::new(0.0, 0.0, 200.0, 40.0));
        let out = drawn(&t);
        assert_eq!(out.len(), 1);
        let node = out.iter().next().expect("one node");
        assert_eq!(node.text.as_deref(), Some("Hello"));
        assert_eq!(node.kind, NodeKind::Text { font: 7, justification: 0, leading: 0, max_width: 200 });
        assert_eq!((node.colour, node.world_colour, node.visible), ([1, 2, 3, 255], [1, 2, 3, 255], true));
    }

    #[test]
    fn default_justification_starts_at_the_top_left_corner() {
        let mut t = Text::new("body", "A");
        lay_out(&mut t, &skin(), Rect::new(0.0, 0.0, 200.0, 40.0));
        let node = drawn(&t).iter().next().expect("one node").clone();
        assert_eq!(node.world.w_axis.truncate(), Vec3::new(-100.0, -20.0, 1.0));
    }

    #[test]
    fn centred_flags_put_the_origin_on_the_rect_centre_and_right_wins_over_centre() {
        let mut centred = Text::new("body", "A").with_justification(justify::H_CENTRE | justify::V_CENTRE);
        lay_out(&mut centred, &skin(), Rect::new(10.0, 10.0, 200.0, 40.0));
        let node = drawn(&centred).iter().next().expect("one node").clone();
        assert_eq!(node.world.w_axis.truncate(), Vec3::new(10.0, 10.0, 1.0));

        let mut right =
            Text::new("body", "A").with_justification(justify::H_RIGHT | justify::H_CENTRE | justify::V_BOTTOM);
        lay_out(&mut right, &skin(), Rect::new(0.0, 0.0, 200.0, 40.0));
        let node = drawn(&right).iter().next().expect("one node").clone();
        assert_eq!(node.world.w_axis.truncate(), Vec3::new(100.0, 20.0, 1.0));
    }

    #[test]
    fn set_text_changes_the_string_without_relayout() {
        let mut t = Text::new("body", "1");
        lay_out(&mut t, &skin(), Rect::SCREEN);
        t.set_text("42");
        let node = drawn(&t).iter().next().expect("one node").clone();
        assert_eq!(node.text.as_deref(), Some("42"));
    }
}
