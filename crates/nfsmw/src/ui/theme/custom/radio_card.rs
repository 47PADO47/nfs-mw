//! The Custom radio card: the song on the air as neutral nodes. A `Panel` background and a `Stack` of three `Text` rows
//! (title, artist and album, time). It replaces the egui card of `hud/radio.rs` (retired in T9).
//!
//! State comes from the feed (`radio_feed.rs`). `layout` does the per-frame work: it reads the feed, works out the
//! fade from the card's age, and lays the components out in the card's place with the card's alpha. `build` emits what
//! `layout` made. So an idle card, or one past its last fade, emits nothing.
//!
//! Timing (the egui card's): the card is up for [`SHOW_SECS`] after the announcement, and its alpha falls linearly over
//! the last [`FADE_SECS`]. Sizes are in FEng units, scaled by the viewport scale in the anchor. The font is the skin's
//! body font at its own size, which stands for the egui title size (15 units); the detail rows are 0.8 of it.
//! The egui card sized its box to the longest line. This one has a fixed width, and the presenter squeezes a longer
//! line into it.

use blackbox_ui::{Component, DrawList, LayoutCx, Panel, Rect, Stack, StackAxis, Text, token_keys};

use super::radio_feed::{RadioFeed, RadioFeedCell};
use super::radio_placement::Anchor;
use super::skin::{faded, keys};

/// How long a card stays up after the radio changed, in seconds, and how much of that it fades out over.
pub(super) const SHOW_SECS: f32 = 6.0;
pub(super) const FADE_SECS: f32 = 1.0;

/// The card's box in FEng units at scale 1: the text width, the padding round it, and one row per line.
const TEXT_WIDTH: f32 = 230.0;
const PAD: f32 = 6.0;
const LINE: f32 = 16.0;
/// The title's glyph scale, and the detail rows' (the egui sizes 15 and 12 units).
const TITLE_SCALE: f32 = 1.0;
const DETAIL_SCALE: f32 = 0.8;

/// The Custom `RadioCard` component. Built per resolve from the shared feed; it holds no state of its own.
pub struct RadioCard {
    feed: RadioFeedCell,
    body: Option<Body>,
}

impl RadioCard {
    /// The card that reads `feed` (the cell the Custom theme holds).
    pub fn new(feed: RadioFeedCell) -> Self {
        Self { feed, body: None }
    }
}

impl Component for RadioCard {
    /// The card does not use `bounds`: its place comes from the HUD layout in the feed, not from the slot it is drawn in.
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.body = Body::of(&self.feed.snapshot(), cx);
    }

    fn build(&self, out: &mut DrawList) {
        let Some(body) = &self.body else { return };
        body.background.build(out);
        body.lines.build(out);
    }
}

/// The laid-out parts of the card for one frame: the background first, so the text draws on top of it.
struct Body {
    background: Panel,
    lines: Stack,
}

impl Body {
    /// The card for `feed` at its current age, or `None` when there is no card or its fade is over.
    fn of(feed: &RadioFeed, cx: &LayoutCx<'_>) -> Option<Self> {
        let card = feed.card.as_ref()?;
        let opacity = opacity(feed.age);
        if opacity <= 0.0 {
            return None;
        }
        let skin = faded(cx.skin, opacity);
        let local = LayoutCx::new(&skin, cx.time_s);
        let place = Frame::at(feed.anchor, cx.skin.metric(token_keys::PADDING).unwrap_or(PAD));
        let mut background = Panel::new(keys::BACKGROUND);
        background.layout(&local, place.outer);
        let s = feed.anchor.scale;
        let mut lines = Stack::new(StackAxis::Column)
            .with_child(line(card.title.clone(), keys::TITLE, TITLE_SCALE * s))
            .with_child(line(card.credit(), keys::CREDIT, DETAIL_SCALE * s))
            .with_child(line(card.status(), keys::STATUS, DETAIL_SCALE * s));
        lines.layout(&local, place.lines);
        Some(Self { background, lines })
    }
}

/// The card's rectangles in FEng units from the canvas centre (a `Rect` is centred): `outer` is the background, `lines`
/// the box the three text rows split.
struct Frame {
    outer: Rect,
    lines: Rect,
}

impl Frame {
    fn at(anchor: Anchor, padding: f32) -> Self {
        let s = anchor.scale;
        let pad = padding * s;
        let width = TEXT_WIDTH * s;
        let height = 3.0 * LINE * s;
        let (left, top) = (anchor.left, anchor.top);
        Self {
            outer: Rect::new(
                left + (width + 2.0 * pad) / 2.0,
                top + (height + 2.0 * pad) / 2.0,
                width + 2.0 * pad,
                height + 2.0 * pad,
            ),
            lines: Rect::new(left + pad + width / 2.0, top + pad + height / 2.0, width, height),
        }
    }
}

/// One text row: the body font, the skin colour `colour`, at `scale` glyphs.
fn line(text: String, colour: &'static str, scale: f32) -> Text {
    Text::new(keys::FONT, text).with_colour_key(colour).with_scale(scale)
}

/// The card's opacity `age` seconds after its announcement: 1 while it is up, falling to 0 over the last
/// [`FADE_SECS`], and 0 from [`SHOW_SECS`] on.
pub(super) fn opacity(age: f32) -> f32 {
    ((SHOW_SECS - age) / FADE_SECS).clamp(0.0, 1.0)
}
