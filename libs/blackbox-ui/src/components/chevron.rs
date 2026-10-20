//! `Chevron`: the double arrow that says a control changes with left and right. One picture serves both ways (the left one
//! is the picture mirrored), and it can be nudged outward, which a control does for a moment after the player presses
//! that way. [`bounce`] is the nudge over time.

use super::Image;
use crate::component::Component;
use crate::context::LayoutCx;
use crate::draw::DrawList;
use crate::geom::Rect;

/// How long a nudge lasts, in seconds, and how far it goes at first, in FEng units.
const BOUNCE_SECONDS: f32 = 0.28;
const BOUNCE_REACH: f32 = 4.0;

/// Which way the chevron points. The skin's picture points right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing {
    Left,
    Right,
}

/// How far out a chevron is nudged `age` seconds after the press that way: it jumps out and settles back, quickly at first.
/// Zero before the press (a negative age) and after it has settled.
pub fn bounce(age: f32) -> f32 {
    if !(0.0..BOUNCE_SECONDS).contains(&age) {
        return 0.0;
    }
    let left = 1.0 - age / BOUNCE_SECONDS;
    BOUNCE_REACH * left * left
}

/// The chevron: a skin texture over a rectangle, mirrored for the left one, moved `nudge` units in the way it points.
pub struct Chevron {
    image: Image,
}

impl Chevron {
    /// The chevron of the skin texture `texture_key` in `rect`, tinted `tint`, nudged by `nudge` units outward.
    pub fn new(texture_key: &str, facing: Facing, rect: Rect, tint: [u8; 4], nudge: f32, cx: &LayoutCx<'_>) -> Self {
        let (uv, shift) = match facing {
            Facing::Right => ([0.0, 0.0, 1.0, 1.0], nudge),
            Facing::Left => ([1.0, 0.0, 0.0, 1.0], -nudge),
        };
        let mut image = Image::new(texture_key).with_uv(uv).with_tint(tint);
        image.layout(cx, Rect::new(rect.x + shift, rect.y, rect.w, rect.h));
        Self { image }
    }
}

impl Component for Chevron {
    /// The chevron is laid out in [`Chevron::new`], so there is nothing to place here.
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        self.image.build(out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::Skin;
    use crate::tree::NodeKind;

    fn built(facing: Facing, nudge: f32) -> crate::tree::UiNode {
        let skin = Skin::new().with_texture("arrow", 5);
        let chevron =
            Chevron::new("arrow", facing, Rect::new(10.0, 0.0, 8.0, 8.0), [255; 4], nudge, &LayoutCx::new(&skin, 0.0));
        let mut out = DrawList::new();
        chevron.build(&mut out);
        out.iter().next().expect("one node").clone()
    }

    #[test]
    fn the_left_chevron_is_the_picture_mirrored_and_each_is_nudged_outward() {
        let (right, left) = (built(Facing::Right, 3.0), built(Facing::Left, 3.0));
        let uv = |n: &crate::tree::UiNode| match n.kind {
            NodeKind::Image { uv, .. } => uv,
            _ => panic!("an image"),
        };
        assert_eq!(uv(&right), [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(uv(&left), [1.0, 0.0, 0.0, 1.0], "mirrored");
        assert_eq!(right.world.w_axis.x, 13.0);
        assert_eq!(left.world.w_axis.x, 7.0);
    }

    #[test]
    fn the_bounce_jumps_out_and_settles_in_a_fraction_of_a_second() {
        assert_eq!(bounce(-0.1), 0.0);
        assert!((bounce(0.0) - BOUNCE_REACH).abs() < 1e-6);
        assert!(bounce(0.1) < bounce(0.0) && bounce(0.2) < bounce(0.1));
        assert_eq!(bounce(BOUNCE_SECONDS), 0.0);
        assert_eq!(bounce(5.0), 0.0);
    }
}
