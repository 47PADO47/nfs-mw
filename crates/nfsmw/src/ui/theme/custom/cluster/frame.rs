//! Where the cluster's rows sit, and the colours its parts are drawn in.
//!
//! - [`Rows`]: the bands of the cluster, measured up from its bottom edge in HUD units at scale 1. A text row grows
//!   with `[custom_hud] text_size`, and the rows above it move up, so a larger text never overlaps the next row.
//! - [`Frame`]: the rows placed in the bottom-right corner of the content box through [`HudBox`], at the HUD scale.
//! - [`Paint`]: every token colour the cluster uses, faded by the HUD opacity once per frame, and the panels drawn in
//!   them or in a colour worked out for one part (a partly lit segment, a pulse).

use blackbox_ui::{Component, LayoutCx, Panel, Rect, Skin};

use super::super::feed::CustomFeed;
use super::super::layout::{Anchor, HudBox};
use super::super::text::{Sizing, size};
use super::super::tokens::{Motion, fade, faded_skin, keys};

/// The cluster's width at scale 1, in HUD units.
pub const WIDTH: f32 = 236.0;
/// The space between two rows.
pub const GAP: f32 = 4.0;
/// The bar row (nitrous, speedbreaker, boost) at text size 1.
pub const BAR_ROW: f32 = 10.0;
/// The speed scale's segments.
pub const SCALE_ROW: f32 = 4.0;
/// The speed row's plate is the speed digits' height plus this.
pub const SPEED_PAD: f32 = 4.0;
/// The rev strip's ordinary segments. The amber and red ones are [`TALL`] times as high.
pub const REV_ROW: f32 = 4.0;
/// How much taller the rev strip's amber and red segments are, so the bands read by shape and not by colour alone.
pub const TALL: f32 = 1.4;
/// The advanced row at text size 1.
pub const ADVANCED_ROW: f32 = 12.0;
/// The advanced row's text, in HUD units at scale 1.
pub const ADVANCED_TEXT: f32 = 11.0;

/// A row: its bottom, measured up from the cluster's bottom edge, and its height, at scale 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band {
    pub bottom: f32,
    pub height: f32,
}

impl Band {
    /// The row's top, measured up from the cluster's bottom edge.
    pub fn top(self) -> f32 {
        self.bottom + self.height
    }

    /// A row `height` high, `gap` above this one.
    fn above(self, gap: f32, height: f32) -> Band {
        Band { bottom: self.top() + gap, height }
    }
}

/// The cluster's rows, bottom to top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rows {
    /// Nitrous, speedbreaker and boost.
    pub bars: Band,
    /// The segmented speed scale.
    pub scale: Band,
    /// The plate with the gear box, the speed digits and the unit.
    pub speed: Band,
    /// The rev strip's ordinary segments (the tall ones reach [`TALL`] times as high).
    pub revs: Band,
    /// The advanced row, drawn only with advanced telemetry on.
    pub advanced: Band,
}

impl Rows {
    /// The rows at `[custom_hud] text_size` `text_size`. A text size that is not a positive number counts as 1.
    pub fn for_text_size(text_size: f32) -> Self {
        let t = match text_size.is_finite() && text_size > 0.0 {
            true => text_size,
            false => 1.0,
        };
        let bars = Band { bottom: 0.0, height: BAR_ROW * t };
        let scale = bars.above(GAP, SCALE_ROW);
        let speed = scale.above(GAP, size::SPEED * t + SPEED_PAD);
        let revs = speed.above(GAP, REV_ROW);
        let advanced = revs.above(GAP + REV_ROW * (TALL - 1.0), ADVANCED_ROW * t);
        Self { bars, scale, speed, revs, advanced }
    }
}

/// The cluster placed for one frame: the HUD's frame, the text sizing and the rows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    hud: HudBox,
    pub sizing: Sizing,
    pub rows: Rows,
}

impl Frame {
    /// The cluster's frame for `feed`'s window and `[custom_hud]` scale and text size.
    pub fn new(feed: &CustomFeed) -> Self {
        let hud = HudBox::new(feed.screen, feed.settings.scale);
        Self { hud, sizing: Sizing::of(&hud, &feed.settings), rows: Rows::for_text_size(feed.settings.text_size) }
    }

    /// The effective HUD scale.
    pub fn scale(&self) -> f32 {
        self.hud.scale()
    }

    /// The `[custom_hud] text_size` the rows were sized for.
    pub fn text_size(&self) -> f32 {
        self.sizing.text_size
    }

    /// The rectangle `w` by `h` (scale 1) whose left edge is `x` units right of the cluster's left edge and whose
    /// bottom is `bottom` units above the cluster's bottom edge, in canvas units at the HUD scale.
    pub fn rect(&self, x: f32, bottom: f32, w: f32, h: f32) -> Rect {
        self.hud.place(Anchor::BottomRight, (x + w - WIDTH, -bottom), (w, h))
    }

    /// The rectangle of `band` across the whole cluster.
    pub fn band(&self, band: Band) -> Rect {
        self.rect(0.0, band.bottom, WIDTH, band.height)
    }
}

/// The token colours the cluster draws in.
const COLOURS: [&str; 9] = [
    keys::TEXT,
    keys::TEXT_SECONDARY,
    keys::SURFACE,
    keys::RULE,
    keys::ACCENT,
    keys::NAV,
    keys::CAUTION,
    keys::DANGER,
    keys::CRITICAL_TEXT,
];
/// The skin key of a part drawn in a colour of its own.
const SCRATCH: &str = "custom.cluster.part";
/// What a component draws for a colour the skin lacks.
const WHITE: [u8; 4] = [255, 255, 255, 255];

/// The cluster's colours for one frame, faded by the HUD opacity.
pub struct Paint {
    skin: Skin,
    time_s: f32,
    opacity: f32,
}

impl Paint {
    /// The colours of `cx`'s skin at `opacity`.
    pub fn new(cx: &LayoutCx<'_>, opacity: f32) -> Self {
        Self { skin: faded_skin(cx.skin, &COLOURS, &[], opacity), time_s: cx.time_s, opacity }
    }

    /// The HUD opacity the colours are faded by.
    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    /// The colour of `key`, faded; faded white for a key the skin lacks, as a component would draw it.
    pub fn colour(&self, key: &str) -> [u8; 4] {
        self.skin.colour(key).unwrap_or_else(|| fade(WHITE, self.opacity))
    }

    /// A layout context on the faded colours, for a component that reads its colour by key.
    pub fn cx(&self) -> LayoutCx<'_> {
        LayoutCx::new(&self.skin, self.time_s)
    }

    /// A panel over `rect` in `rgba`.
    pub fn panel(&mut self, rgba: [u8; 4], rect: Rect) -> Panel {
        self.skin = std::mem::take(&mut self.skin).with_colour(SCRATCH, rgba);
        let mut panel = Panel::new(SCRATCH);
        panel.layout(&self.cx(), rect);
        panel
    }
}

/// `a` moved towards `b` by `t` (0 to 1), channel by channel, alpha included.
pub fn mix(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let t = match t.is_nan() {
        true => 0.0,
        false => t.clamp(0.0, 1.0),
    };
    let channel = |i: usize| (f32::from(a[i]) + (f32::from(b[i]) - f32::from(a[i])) * t).round() as u8;
    [channel(0), channel(1), channel(2), channel(3)]
}

/// Lays out a row of `count` equal segments across `rect`, `gap` apart (canvas units), and returns each one's
/// rectangle, left to right.
pub fn segments(rect: Rect, count: usize, gap: f32) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }
    let n = count as f32;
    let width = ((rect.w - gap * (n - 1.0)) / n).max(0.0);
    (0..count)
        .map(|i| {
            let left = rect.min_x() + i as f32 * (width + gap);
            Rect::new(left + width / 2.0, rect.y, width, rect.h)
        })
        .collect()
}

/// How much of segment `index` (of `count`) a fill of `fraction` (0 to 1) covers: 1 for a full one, 0 for an empty
/// one, and the part in between for the segment under the fill's end.
pub fn lit(index: usize, count: usize, fraction: f32) -> f32 {
    if fraction.is_nan() {
        return 0.0;
    }
    let filled = fraction.clamp(0.0, 1.0) * count as f32;
    (filled - index as f32).clamp(0.0, 1.0)
}

/// The parts of the cluster, drawn in the order they were added.
pub type Parts = Vec<Box<dyn Component>>;

/// One frame of the cluster being laid out: where it sits, its colours, the motion table, the theme's layout context
/// (for the plate and the labels, which fade themselves) and the parts made so far.
pub struct Canvas<'a> {
    pub frame: Frame,
    pub paint: Paint,
    pub motion: Motion,
    pub cx: LayoutCx<'a>,
    pub parts: Parts,
}

impl<'a> Canvas<'a> {
    /// An empty canvas for `feed`, laid out against `cx`.
    pub fn new(feed: &CustomFeed, cx: LayoutCx<'a>) -> Self {
        Self {
            frame: Frame::new(feed),
            paint: Paint::new(&cx, feed.settings.opacity),
            motion: Motion::for_settings(&feed.settings),
            cx,
            parts: Vec::new(),
        }
    }

    /// Adds `part` on top of the parts made so far.
    pub fn push(&mut self, part: impl Component + 'static) {
        self.parts.push(Box::new(part));
    }

    /// A panel over `rect` in `rgba`, on top of the parts made so far.
    pub fn panel(&mut self, rgba: [u8; 4], rect: Rect) {
        let panel = self.paint.panel(rgba, rect);
        self.push(panel);
    }

    /// The HUD opacity.
    pub fn opacity(&self) -> f32 {
        self.paint.opacity()
    }
}
