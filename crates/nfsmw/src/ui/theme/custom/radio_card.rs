//! The Custom radio card (Custom HUD 2026, PR 11b): the song on the air on a Custom plate at the top left of the content
//! box (`docs/specs/custom-hud/layout.md`, the top-left zone), in the Custom tokens and the OS fonts. The title in the
//! label face, the artist and album under it in the secondary colour, and the time played in the numeric face.
//!
//! - **State.** The song and how long ago it was announced come from the radio feed (`radio_feed.rs`), as before; the
//!   window and the `[custom_hud]` options (scale, text size, opacity, motion reduction) from the Custom feed.
//! - **Place.** `radio_placement::card_rect`, from the content box's top-left anchor (`HudBox`), at the HUD scale.
//! - **Timing.** The card is up for [`SHOW_SECS`] after the announcement. It fades in over the motion table's `fade_in`
//!   and slides in from the left over `slide` (no slide with motion reduction), and fades out over the last `fade_out`
//!   seconds ([`card_opacity`]). Its whole alpha is multiplied by `[custom_hud] opacity`. An idle card, or one past its
//!   time, builds nothing.
//!
//! [`opacity`] is the older timing (a one-second fade at the end), which the Remastered card still reads through the
//! radio feed.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect, justify};

use super::feed::{CustomFeed, CustomFeedCell};
use super::layout::HudBox;
use super::plate::{Plate, PlateStyle};
use super::radio_feed::{RadioFeed, RadioFeedCell};
use super::radio_placement::{self, PAD};
use super::text::{Label, Sizing};
use super::tokens::{Motion, keys, ramp};
use crate::hud::Card;
use crate::ui::raster_text::{CUSTOM_LABEL_FONT_KEY, width_units};

/// How long a card stays up after the radio changed, in seconds, and how much of that the older timing fades out over.
pub(super) const SHOW_SECS: f32 = 6.0;
pub(super) const FADE_SECS: f32 = 1.0;

/// The rows' text sizes at scale 1.
pub(super) const TITLE_SIZE: f32 = 14.0;
pub(super) const DETAIL_SIZE: f32 = 11.0;

/// The depth of the plate and of the text over it.
const PLATE_Z: f32 = 2.0;
const TEXT_Z: f32 = 1.0;

/// The Custom `RadioCard` component. Built per resolve from the shared feeds; it holds no state of its own.
pub struct RadioCard {
    radio: RadioFeedCell,
    feed: CustomFeedCell,
    parts: Vec<Box<dyn Component>>,
}

impl RadioCard {
    /// The card that reads the song from `radio` and the window and options from `feed` (the cells the Custom theme
    /// holds).
    pub fn new(radio: RadioFeedCell, feed: CustomFeedCell) -> Self {
        Self { radio, feed, parts: Vec::new() }
    }
}

impl Component for RadioCard {
    /// The card does not use `bounds`: its place comes from the HUD layout, not from the slot it is drawn in.
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.parts = parts(&self.radio.snapshot(), &self.feed.get(), cx);
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// The card's parts for one frame, back to front: nothing when there is no card or it is not showing.
fn parts(radio: &RadioFeed, feed: &CustomFeed, cx: &LayoutCx<'_>) -> Vec<Box<dyn Component>> {
    let Some(card) = radio.card.as_ref() else { return Vec::new() };
    let settings = &feed.settings;
    let motion = Motion::for_settings(settings);
    let opacity = card_opacity(radio.age, &motion) * settings.opacity.clamp(0.0, 1.0);
    if opacity <= 0.0 {
        return Vec::new();
    }
    let frame = HudBox::new(feed.screen, settings.scale);
    let sizing = Sizing::of(&frame, settings);
    let slide = -(1.0 - ramp(radio.age, motion.slide)) * motion.slide_distance;
    let rect = radio_placement::card_rect(&frame, settings.text_size, slide);
    let style =
        PlateStyle::SURFACE.with_accent(keys::ACCENT).scaled(frame.scale()).with_opacity(opacity).with_z(PLATE_Z);
    let mut parts: Vec<Box<dyn Component>> = vec![Box::new(Plate::new(rect, style, cx))];
    let s = frame.scale();
    let left = rect.min_x() + PAD * s;
    let room = (rect.w - 2.0 * PAD * s).max(0.0);
    for (i, row) in rows(card).into_iter().enumerate() {
        let y = rect.min_y() + radio_placement::row_middle(i, settings.text_size) * s;
        let size = fitted(&row.text, row.size, room, sizing);
        let label = row.label(size).opacity(opacity).justify(justify::V_CENTRE).z(TEXT_Z);
        let line = Rect::new(left + room / 2.0, y, room, sizing.units(row.size) * 2.0);
        parts.push(Box::new(label.laid_out(line, sizing, cx)));
    }
    parts
}

/// One row of the card: its text, its face and colour, and its size.
struct Row {
    text: String,
    numeric: bool,
    colour: &'static str,
    size: f32,
}

impl Row {
    fn label(&self, size: f32) -> Label {
        let label = match self.numeric {
            true => Label::numeric(self.text.clone(), size),
            false => Label::label(self.text.clone(), size),
        };
        label.colour(self.colour)
    }
}

/// The card's three rows: the title, the artist and album, and the time played (with the pause state).
fn rows(card: &Card) -> [Row; 3] {
    [
        Row { text: card.title.clone(), numeric: false, colour: keys::TEXT, size: TITLE_SIZE },
        Row { text: card.credit(), numeric: false, colour: keys::TEXT_SECONDARY, size: DETAIL_SIZE },
        Row { text: card.status(), numeric: true, colour: keys::TEXT_SECONDARY, size: DETAIL_SIZE },
    ]
}

/// `size`, or less when `text` at that size is wider than `room` units, so a long title stays on the plate. A text whose
/// width is not known (the font did not load) keeps `size`.
fn fitted(text: &str, size: f32, room: f32, sizing: Sizing) -> f32 {
    match width_units(CUSTOM_LABEL_FONT_KEY, text, sizing.glyph_scale(size)) {
        Some(width) if width > room && width > 0.0 => size * room / width,
        _ => size,
    }
}

/// The card's opacity `age` seconds after its announcement, in `motion`: rising over `fade_in`, 1 while it is up, and
/// falling to 0 over the last `fade_out` seconds before [`SHOW_SECS`].
pub(super) fn card_opacity(age: f32, motion: &Motion) -> f32 {
    if !(0.0..SHOW_SECS).contains(&age) {
        return 0.0;
    }
    let rise = ramp(age, motion.fade_in);
    let fall = ramp(SHOW_SECS - age, motion.fade_out);
    rise.min(fall)
}

/// The older timing, which the Remastered card reads through `RadioFeed::opacity`: 1 while the card is up, falling to 0
/// over the last [`FADE_SECS`], and 0 from [`SHOW_SECS`] on.
pub(super) fn opacity(age: f32) -> f32 {
    ((SHOW_SECS - age) / FADE_SECS).clamp(0.0, 1.0)
}
