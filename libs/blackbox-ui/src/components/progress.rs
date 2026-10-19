//! `LinearProgress`: a rounded horizontal bar that fills from left to right. It draws a border, a
//! track behind the fill, and a gradient-shaded fill whose width is `value * total_width`. Corners
//! come from the theme radius (`theme.radius`); the gradient lightens the top and darkens the
//! bottom of the fill by a small amount.

use crate::component::Component;
use crate::context::LayoutCx;
use crate::draw::DrawList;
use crate::geom::Rect;
use crate::tokens::keys as token_keys;
use crate::tree::NodeKind;

use super::{DEFAULT_Z, FALLBACK_COLOUR, FULL_UV, UNROTATED, node_at};

/// The height of one horizontal strip when approximating rounded corners.
const STRIP: f32 = 0.5;
/// The border thickness around the track, in FEng units.
const BORDER_W: f32 = 0.8;
/// How much the gradient lightens the top and darkens the bottom, as a fraction of the RGB.
const GRADIENT_LIFT: f32 = 0.25;

/// A horizontal progress bar with rounded corners, a thin border and a slight vertical gradient on
/// the fill. Its corners read `theme.radius` from the skin; each colour is a skin key.
pub struct LinearProgress {
    track_key: &'static str,
    fill_key: &'static str,
    border_key: &'static str,
    value: f32,
    quads: Vec<Quad>,
}

struct Quad {
    rect: Rect,
    colour: [u8; 4],
}

impl LinearProgress {
    pub fn new(track_key: &'static str, fill_key: &'static str, border_key: &'static str) -> Self {
        Self { track_key, fill_key, border_key, value: 0.0, quads: Vec::new() }
    }

    pub fn with_value(mut self, value: f32) -> Self {
        self.value = value.clamp(0.0, 1.0);
        self
    }
}

impl Component for LinearProgress {
    fn layout(&mut self, cx: &LayoutCx<'_>, bounds: Rect) {
        let radius = cx.skin.metric(token_keys::RADIUS).unwrap_or(2.0);
        let border_c = cx.skin.colour(self.border_key).unwrap_or([255, 255, 255, 60]);
        let track_c = cx.skin.colour(self.track_key).unwrap_or([0, 0, 0, 100]);
        let fill_c = cx.skin.colour(self.fill_key).unwrap_or(FALLBACK_COLOUR);

        self.quads.clear();
        rounded_rect(&mut self.quads, bounds, radius, border_c);
        let inner = bounds.inset(BORDER_W, BORDER_W);
        let inner_r = (radius - BORDER_W).max(0.0);
        rounded_rect(&mut self.quads, inner, inner_r, track_c);
        if self.value > 0.001 {
            gradient_fill(&mut self.quads, inner, inner_r, self.value, fill_c);
        }
    }

    fn build(&self, out: &mut DrawList) {
        for quad in &self.quads {
            let kind =
                NodeKind::Image { texture: 0, uv: FULL_UV, mask: None, mask_rotation: UNROTATED, mask_uv: FULL_UV };
            out.push(node_at(quad.rect.world(DEFAULT_Z), DEFAULT_Z, quad.colour, kind));
        }
    }
}

fn rounded_rect(out: &mut Vec<Quad>, rect: Rect, radius: f32, colour: [u8; 4]) {
    let (left, top, right, bottom) = (rect.min_x(), rect.min_y(), rect.max_x(), rect.max_y());
    let r = radius.min(rect.w / 2.0).min(rect.h / 2.0).max(0.0);
    let strips = (r / STRIP).ceil().max(0.0) as usize;
    for k in 0..strips {
        let y0 = k as f32 * STRIP;
        let y1 = ((k + 1) as f32 * STRIP).min(r);
        let inset = r - circle_half(r, (y0 + y1) / 2.0);
        push(out, left + inset, top + y0, right - inset, top + y1, colour);
        push(out, left + inset, bottom - y1, right - inset, bottom - y0, colour);
    }
    if top + r < bottom - r {
        push(out, left, top + r, right, bottom - r, colour);
    }
}

fn gradient_fill(out: &mut Vec<Quad>, inner: Rect, radius: f32, value: f32, base: [u8; 4]) {
    let (left, top, _right, bottom) = (inner.min_x(), inner.min_y(), inner.max_x(), inner.max_y());
    let r = radius.min(inner.w / 2.0).min(inner.h / 2.0).max(0.0);
    let fill_right = left + inner.w * value;
    let height = inner.h;
    let count = (height / STRIP).ceil().max(1.0) as usize;
    for k in 0..count {
        let y0 = top + k as f32 * STRIP;
        let y1 = (top + (k + 1) as f32 * STRIP).min(bottom);
        let y_mid = (y0 + y1) / 2.0;
        let dy_top = y_mid - top;
        let dy_bot = bottom - y_mid;
        let left_inset = if dy_top < r {
            r - circle_half(r, dy_top)
        } else if dy_bot < r {
            r - circle_half(r, dy_bot)
        } else {
            0.0
        };
        let x0 = left + left_inset;
        let right_limit = if fill_right >= inner.max_x() - 0.5 { inner.max_x() - left_inset } else { fill_right };
        let x1 = right_limit;
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        let t = if height > 0.0 { ((y_mid - top) / height).clamp(0.0, 1.0) } else { 0.5 };
        push(out, x0, y0, x1, y1, tint(base, t));
    }
}

fn tint(base: [u8; 4], t: f32) -> [u8; 4] {
    let shift = (0.5 - t) * 2.0 * GRADIENT_LIFT;
    let adj = |v: u8| (f32::from(v) * (1.0 + shift)).round().clamp(0.0, 255.0) as u8;
    [adj(base[0]), adj(base[1]), adj(base[2]), base[3]]
}

fn push(out: &mut Vec<Quad>, x0: f32, y0: f32, x1: f32, y1: f32, colour: [u8; 4]) {
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    out.push(Quad { rect: Rect::new((x0 + x1) / 2.0, (y0 + y1) / 2.0, x1 - x0, y1 - y0), colour });
}

fn circle_half(r: f32, dy: f32) -> f32 {
    let from_centre = r - dy;
    (r * r - from_centre * from_centre).max(0.0).sqrt()
}

#[cfg(test)]
mod tests {
    use super::LinearProgress;
    use crate::components::testing::{drawn, lay_out};
    use crate::geom::Rect;
    use crate::skin::Skin;
    use crate::tokens::keys as token_keys;

    fn test_skin() -> Skin {
        Skin::new()
            .with_colour("track", [0, 0, 0, 80])
            .with_colour("fill", [255, 200, 80, 255])
            .with_colour("border", [255, 255, 255, 60])
            .with_metric(token_keys::RADIUS, 3.0)
    }

    #[test]
    fn an_empty_bar_draws_only_the_border_and_track() {
        let skin = test_skin();
        let mut bar = LinearProgress::new("track", "fill", "border").with_value(0.0);
        lay_out(&mut bar, &skin, Rect::new(0.0, 0.0, 100.0, 8.0));
        let out = drawn(&bar);
        let colours: Vec<[u8; 4]> = out.iter().map(|n| n.colour).collect();
        assert!(colours.contains(&[0, 0, 0, 80]), "the track is drawn");
        assert!(colours.contains(&[255, 255, 255, 60]), "the border is drawn");
        assert!(!colours.iter().any(|c| c[0] == 255 && c[1] == 200), "no fill");
    }

    #[test]
    fn a_half_full_bar_has_fill_nodes() {
        let skin = test_skin();
        let mut bar = LinearProgress::new("track", "fill", "border").with_value(0.5);
        lay_out(&mut bar, &skin, Rect::new(0.0, 0.0, 100.0, 8.0));
        let out = drawn(&bar);
        let fill_nodes: Vec<_> = out.iter().filter(|n| n.colour[1] >= 190 && n.colour[1] <= 210).collect();
        assert!(!fill_nodes.is_empty(), "fill strips are drawn");
    }

    #[test]
    fn a_full_bar_fills_the_whole_width() {
        let skin = test_skin();
        let mut bar = LinearProgress::new("track", "fill", "border").with_value(1.0);
        lay_out(&mut bar, &skin, Rect::new(0.0, 0.0, 100.0, 8.0));
        let out = drawn(&bar);
        assert!(out.len() > 10, "many strips for border + track + fill");
    }
}
