//! `Choice`: the control of a row that cycles through values. The value is centred in its column, with a chevron at each end
//! when the row can be changed that way. It is laid out against the skin when built.

use super::chevron::{Chevron, Facing, bounce};
use super::{Text, justify};
use crate::component::Component;
use crate::context::LayoutCx;
use crate::draw::DrawList;
use crate::geom::Rect;

/// The chevron's size and how far inside the column's ends it sits.
const CHEVRON: f32 = 14.0;
const CHEVRON_INSET: f32 = 1.0;

/// What a [`Choice`] reads from the skin.
#[derive(Clone, Copy, Debug)]
pub struct ChoiceStyle<'a> {
    /// The value's font and colour key, and its glyph scale.
    pub font: &'a str,
    pub text: &'a str,
    pub scale: f32,
    /// The chevron's texture key and colour key.
    pub arrow: &'a str,
    pub arrow_colour: &'a str,
}

/// A value between two chevrons.
pub struct Choice {
    parts: Vec<Box<dyn Component>>,
}

impl Choice {
    /// The control in `column`, showing `value`. `arrows` shows the chevrons (a row that is not under the selection, or
    /// that cannot change, leaves them out). `nudge` is `(facing, age)` of the last press on this row, which moves that
    /// chevron out for a moment ([`bounce`]).
    pub fn new(
        style: ChoiceStyle<'_>,
        column: Rect,
        value: &str,
        arrows: bool,
        nudge: Option<(Facing, f32)>,
        cx: &LayoutCx<'_>,
    ) -> Self {
        let mut parts: Vec<Box<dyn Component>> = Vec::new();
        let tint = cx.skin.colour(style.arrow_colour).unwrap_or([255; 4]);
        if arrows {
            for facing in [Facing::Left, Facing::Right] {
                let x = match facing {
                    Facing::Left => column.min_x() + CHEVRON / 2.0 + CHEVRON_INSET,
                    Facing::Right => column.max_x() - CHEVRON / 2.0 - CHEVRON_INSET,
                };
                let pushed = nudge.filter(|(side, _)| *side == facing).map_or(0.0, |(_, age)| bounce(age));
                let rect = Rect::new(x, column.y, CHEVRON, CHEVRON);
                parts.push(Box::new(Chevron::new(style.arrow, facing, rect, tint, pushed, cx)));
            }
        }
        let text_width = (column.w - 2.0 * (CHEVRON + CHEVRON_INSET)).max(0.0);
        let mut text = Text::new(style.font, value)
            .with_colour_key(style.text)
            .with_scale(style.scale)
            .with_justification(justify::H_CENTRE | justify::V_CENTRE);
        text.layout(cx, Rect::new(column.x, column.y, text_width, column.h));
        parts.push(Box::new(text));
        Self { parts }
    }
}

impl Component for Choice {
    /// The control is laid out in [`Choice::new`], so there is nothing to place here.
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::Skin;
    use crate::tree::UiNode;

    const STYLE: ChoiceStyle<'static> = ChoiceStyle { font: "f", text: "c", scale: 1.0, arrow: "a", arrow_colour: "c" };

    fn built(arrows: bool, nudge: Option<(Facing, f32)>) -> Vec<UiNode> {
        let skin = Skin::new().with_font("f", 3).with_texture("a", 9).with_colour("c", [200, 100, 50, 255]);
        let choice =
            Choice::new(STYLE, Rect::new(0.0, 0.0, 120.0, 16.0), "High", arrows, nudge, &LayoutCx::new(&skin, 0.0));
        let mut out = DrawList::new();
        choice.build(&mut out);
        out.iter().cloned().collect()
    }

    #[test]
    fn the_value_sits_between_two_chevrons_at_the_ends_of_the_column() {
        let nodes = built(true, None);
        assert_eq!(nodes.len(), 3);
        let xs: Vec<f32> = nodes.iter().map(|n| n.world.w_axis.x).collect();
        assert!(xs[0] < xs[2] && xs[0] < -50.0 && xs[1] > 50.0, "{xs:?}");
        assert_eq!(nodes[2].text.as_deref(), Some("High"));
        assert_eq!(nodes[0].colour, [200, 100, 50, 255], "the chevrons take the arrow colour");
    }

    #[test]
    fn a_row_without_arrows_shows_only_the_value() {
        let nodes = built(false, None);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].text.as_deref(), Some("High"));
    }

    #[test]
    fn a_press_nudges_only_the_chevron_it_pressed() {
        let rest = built(true, None);
        let pressed = built(true, Some((Facing::Right, 0.0)));
        assert_eq!(pressed[0].world.w_axis.x, rest[0].world.w_axis.x, "the left one stays");
        assert!(pressed[1].world.w_axis.x > rest[1].world.w_axis.x + 3.0, "the right one is out");
        let settled = built(true, Some((Facing::Right, 1.0)));
        assert_eq!(settled[1].world.w_axis.x, rest[1].world.w_axis.x, "and back once it has settled");
    }
}
