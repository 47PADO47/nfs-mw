//! `PillBar`: a rounded track with a border and a gradient fill that grows from the left, as a gauge or a slider. The fill
//! can start in a point. It is built from the same thin strips as [`Rounded`], [`Outline`] and [`Slant`], so every part is
//! a quad the presenter draws, and it holds no state: [`PillBar::new`] lays it out against the skin.

use super::{Outline, Rounded, Slant};
use crate::component::Component;
use crate::context::LayoutCx;
use crate::draw::DrawList;
use crate::geom::Rect;

/// How the fill brightens from its start to its end: `(position, opacity)`.
const FILL_STOPS: [(f32, f32); 3] = [(0.0, 0.55), (0.6, 0.9), (1.0, 1.0)];
/// The border's thickness, and how far the fill sits inside the track, as a share of the bar's height.
const BORDER: f32 = 1.4;
const INSET: f32 = 0.15;

/// The skin colour keys of a [`PillBar`].
#[derive(Clone, Copy, Debug)]
pub struct PillStyle<'a> {
    /// The dark inside of the track.
    pub track: &'a str,
    /// The thin line round it.
    pub border: &'a str,
    /// The fill.
    pub fill: &'a str,
}

/// A track, its border and a fill up to `value` (0 to 1).
pub struct PillBar {
    parts: Vec<Box<dyn Component>>,
}

impl PillBar {
    /// The bar in `rect`. The fill is `value` of the way along the track (clamped to 0 to 1); with `point` it starts in a
    /// point half the bar's height long, so it reads as an arrow head. A value of 0 draws the track alone.
    pub fn new(style: PillStyle<'_>, rect: Rect, value: f32, point: bool, cx: &LayoutCx<'_>) -> Self {
        let radius = rect.h / 2.0;
        let mut parts: Vec<Box<dyn Component>> = vec![
            Box::new(Rounded::new(style.track, rect, radius, cx)),
            Box::new(Outline::new(style.border, rect, radius, BORDER, cx)),
        ];
        let value = if value.is_nan() { 0.0 } else { value.clamp(0.0, 1.0) };
        if value > 0.0 {
            parts.extend(fill(style.fill, rect, value, point, cx));
        }
        Self { parts }
    }
}

/// The fill: an optional point, then the body, both inside the track.
fn fill(colour: &str, rect: Rect, value: f32, point: bool, cx: &LayoutCx<'_>) -> Vec<Box<dyn Component>> {
    let inset = rect.h * INSET;
    let (top, bottom) = (rect.min_y() + inset, rect.max_y() - inset);
    let left = rect.min_x() + inset;
    let right = left + (rect.max_x() - inset - left) * value;
    let tip = if point { (rect.h / 2.0).min(right - left) } else { 0.0 };
    let mut parts: Vec<Box<dyn Component>> = Vec::new();
    if tip > 0.0 {
        // The point: two halves, the upper leaning right at the top and the lower leaning right at the bottom, meeting
        // at the fill's left edge.
        let middle = (top + bottom) / 2.0;
        for (from, to, lean) in [(top, middle, tip), (middle, bottom, -tip)] {
            let half = Rect::new(left + tip / 2.0, (from + to) / 2.0, tip, to - from);
            parts.push(Box::new(Slant::at_default_depth(colour, half, lean, &[(0.0, FILL_STOPS[0].1)], cx)));
        }
    }
    if right > left + tip {
        let body = Rect::new((left + tip + right) / 2.0, (top + bottom) / 2.0, right - left - tip, bottom - top);
        parts.push(Box::new(Slant::at_default_depth(colour, body, 0.0, &FILL_STOPS, cx)));
    }
    parts
}

impl Component for PillBar {
    /// The bar is laid out in [`PillBar::new`], so there is nothing to place here.
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

    const STYLE: PillStyle<'static> = PillStyle { track: "t", border: "b", fill: "f" };

    fn skin() -> Skin {
        Skin::new().with_colour("t", [1, 1, 1, 255]).with_colour("b", [2, 2, 2, 255]).with_colour("f", [9, 8, 7, 255])
    }

    /// The total width of the nodes in the fill's colour.
    fn fill_width(value: f32, point: bool) -> f32 {
        let skin = skin();
        let bar = PillBar::new(STYLE, Rect::new(0.0, 0.0, 200.0, 20.0), value, point, &LayoutCx::new(&skin, 0.0));
        let mut out = DrawList::new();
        bar.build(&mut out);
        out.iter().filter(|n| n.colour[..3] == [9, 8, 7]).map(|n| n.world.x_axis.x).sum()
    }

    #[test]
    fn the_fill_grows_with_the_value_inside_the_track() {
        assert_eq!(fill_width(0.0, false), 0.0, "an empty bar is the track alone");
        let half = fill_width(0.5, false);
        let full = fill_width(1.0, false);
        assert!((full - 194.0).abs() < 0.5, "the full fill spans the track less the insets: {full}");
        assert!((half - 97.0).abs() < 0.5, "half of that: {half}");
        assert!(fill_width(2.0, false) <= full + 1e-3, "a value past 1 is clamped");
        assert_eq!(fill_width(f32::NAN, false), 0.0);
    }

    /// The right edge of the fill.
    fn fill_end(value: f32, point: bool) -> f32 {
        let skin = skin();
        let bar = PillBar::new(STYLE, Rect::new(0.0, 0.0, 200.0, 20.0), value, point, &LayoutCx::new(&skin, 0.0));
        let mut out = DrawList::new();
        bar.build(&mut out);
        out.iter()
            .filter(|n| n.colour[..3] == [9, 8, 7])
            .map(|n| n.world.w_axis.x + n.world.x_axis.x / 2.0)
            .fold(f32::MIN, f32::max)
    }

    #[test]
    fn a_pointed_fill_ends_where_a_plain_one_does_and_a_short_one_keeps_its_point_inside() {
        assert!((fill_end(0.5, true) - fill_end(0.5, false)).abs() < 0.5);
        assert!(fill_end(0.02, true) < -90.0, "and near the left end: {}", fill_end(0.02, true));
    }

    #[test]
    fn the_track_and_the_border_are_always_drawn() {
        let skin = skin();
        let bar = PillBar::new(STYLE, Rect::new(0.0, 0.0, 100.0, 12.0), 0.0, true, &LayoutCx::new(&skin, 0.0));
        let mut out = DrawList::new();
        bar.build(&mut out);
        assert!(out.iter().any(|n| n.colour[..3] == [1, 1, 1]) && out.iter().any(|n| n.colour[..3] == [2, 2, 2]));
    }
}
