//! The footer's hints at the bottom right of the content box: each hint's keys from the live bindings (a controller
//! glyph, or the key's name on a small cap), then what it does. The hints are laid out from the right edge, so the last
//! one ends at the safe area's edge. The same places serve the frontend's hit test ([`hint_at`]).

use blackbox_ui::{Component, Image, LayoutCx, Rect, Rounded};
use glam::Vec2;

use super::layout::MenuFrame;
use super::{Align, Parts};
use crate::input::Action;
use crate::ui::input_icons::ATLAS;
use crate::ui::prompts::Prompt;
use crate::ui::raster_text::{CUSTOM_LABEL_FONT_KEY, width_units};
use crate::ui::theme::MenuHint;
use crate::ui::theme::custom::text::{Label, Sizing};
use crate::ui::theme::custom::tokens::{faded_skin, keys};

/// A hint's text size, and a key cap's.
const HINT_SIZE: f32 = 12.0;
const KEY_SIZE: f32 = 11.0;
/// A controller glyph's size, and a key cap's height and the room each side of its text, at scale 1.
const GLYPH: f32 = 16.0;
const KEY_HEIGHT: f32 = 16.0;
const KEY_PAD: f32 = 5.0;
const KEY_RADIUS: f32 = 2.0;
/// The space between two keys, between the keys and the label, and between two hints, at scale 1.
const KEY_GAP: f32 = 4.0;
const LABEL_GAP: f32 = 6.0;
const HINT_GAP: f32 = 18.0;
/// A key cap's alpha.
const KEY_ALPHA: f32 = 0.9;

/// One key of a hint: where it is drawn, and what it shows.
#[derive(Clone, Debug, PartialEq)]
pub struct KeySpot {
    pub rect: Rect,
    pub prompt: Prompt,
}

/// One footer hint: its keys, where its label starts, and the area a click presses.
#[derive(Clone, Debug, PartialEq)]
pub struct HintSpot {
    pub keys: Vec<KeySpot>,
    pub label_x: f32,
    pub area: Rect,
}

/// The width of `text` in the label face at `size`. An estimate stands in when the font did not load.
fn text_width(text: &str, size: f32, sizing: Sizing) -> f32 {
    let estimate = text.chars().count() as f32 * 0.5 * sizing.units(size);
    width_units(CUSTOM_LABEL_FONT_KEY, text, sizing.glyph_scale(size)).unwrap_or(estimate)
}

/// How wide one key of a hint is.
fn key_width(prompt: &Prompt, frame: &MenuFrame) -> f32 {
    let s = frame.scale();
    match prompt {
        Prompt::Icon(_) => GLYPH * s,
        Prompt::Key(name) => text_width(name, KEY_SIZE, frame.sizing()) + 2.0 * KEY_PAD * s,
    }
}

/// How wide a whole hint is: its keys, the gaps and its label.
fn hint_width(hint: &MenuHint, frame: &MenuFrame) -> f32 {
    let s = frame.scale();
    let keys: f32 = hint.keys.iter().map(|prompt| key_width(prompt, frame) + KEY_GAP * s).sum();
    keys - KEY_GAP * s + LABEL_GAP * s + text_width(&hint.label, HINT_SIZE, frame.sizing())
}

/// Lays the hints out along the footer line, the last one ending at the safe area's right edge.
pub fn hint_spots(hints: &[MenuHint], frame: &MenuFrame) -> Vec<HintSpot> {
    let s = frame.scale();
    let y = frame.footer_y();
    let total: f32 = hints.iter().map(|hint| hint_width(hint, frame)).sum::<f32>()
        + HINT_GAP * s * hints.len().saturating_sub(1) as f32;
    let mut x = frame.safe().max_x() - total;
    let mut spots = Vec::with_capacity(hints.len());
    for hint in hints {
        let start = x;
        let mut keys = Vec::with_capacity(hint.keys.len());
        for prompt in &hint.keys {
            let width = key_width(prompt, frame);
            let height = match prompt {
                Prompt::Icon(_) => GLYPH * s,
                Prompt::Key(_) => KEY_HEIGHT * s,
            };
            keys.push(KeySpot { rect: Rect::new(x + width / 2.0, y, width, height), prompt: prompt.clone() });
            x += width + KEY_GAP * s;
        }
        let label_x = x - KEY_GAP * s + LABEL_GAP * s;
        let end = label_x + text_width(&hint.label, HINT_SIZE, frame.sizing());
        let area = Rect::new((start + end) / 2.0, y, end - start + 4.0 * s, (KEY_HEIGHT + 6.0) * s);
        spots.push(HintSpot { keys, label_x, area });
        x = end + HINT_GAP * s;
    }
    spots
}

/// The hint under a point of the screen whose click presses an action, on a window `aspect` wide at `text_size`.
pub fn hint_at(hints: &[MenuHint], point: Vec2, aspect: f32, text_size: f32) -> Option<Action> {
    let frame = MenuFrame::new(aspect, text_size);
    let spots = hint_spots(hints, &frame);
    hints.iter().zip(&spots).find(|(_, spot)| spot.area.contains(point.x, point.y)).and_then(|(hint, _)| hint.action)
}

impl Parts<'_, '_> {
    /// The hints along the footer line.
    pub(super) fn footer(&mut self, hints: &[MenuHint]) {
        let frame = self.frame();
        for (hint, spot) in hints.iter().zip(hint_spots(hints, &frame)) {
            for key in &spot.keys {
                self.key(key);
            }
            let label = Label::label(hint.label.clone(), HINT_SIZE).colour(keys::TEXT_SECONDARY);
            self.text(label, (spot.label_x, spot.area.y), Align::Left);
        }
    }

    /// One key: the controller's glyph, or the key's name on a small cap.
    fn key(&mut self, key: &KeySpot) {
        match &key.prompt {
            Prompt::Icon(glyph) => {
                let mut image = Image::raw(ATLAS).with_uv(glyph.uv());
                image.layout(self.cx(), key.rect);
                self.add(image);
            }
            Prompt::Key(name) => {
                let skin = faded_skin(self.cx().skin, &[keys::RULE], &[], KEY_ALPHA);
                let radius = KEY_RADIUS * self.frame().scale();
                let cap = Rounded::new(keys::RULE, key.rect, radius, &LayoutCx::new(&skin, self.cx().time_s));
                self.add(cap);
                let label = Label::label(name.clone(), KEY_SIZE).colour(keys::TEXT);
                self.text(label, (key.rect.x, key.rect.y), Align::Centre);
            }
        }
    }
}
