//! The Remastered vertical menu: the main menu, the option categories, the pause menu and the safe house as a list down the
//! left of the screen. A title in wide-spaced capitals, the entries under it with the selected one on a slanted gold bar
//! and a lightning bolt, the game's logo above, and the footer's hints in a ruled strip at the bottom. It reads the
//! entries from the cell the frontend fills ([`MenuListCell`]) and draws nothing for a slot with no list.
//!
//! [`layout`] holds the places, shared with the frontend's hit tests.

pub mod layout;

use blackbox_ui::{Component, DrawList, Image, LayoutCx, Outline, Panel, Rect, Slant, SlotId, Text, justify};

use self::layout as place;
use self::layout::HintSpot;
use super::skin::keys;
use crate::ui::input_icons::Glyph;
use crate::ui::prompts::Prompt;
use crate::ui::theme::{MenuEntry, MenuList, MenuListCell, MenuStyle};

const TAGLINE: [&str; 2] = ["RACE YOUR WAY", "TO THE TOP"];

/// How the highlight bar fades in from its left end to its slanted gold end: `(position, opacity)`.
const BAR_STOPS: [(f32, f32); 7] =
    [(0.0, 0.02), (0.24, 0.18), (0.47, 0.32), (0.65, 0.5), (0.82, 0.62), (0.94, 0.72), (1.0, 0.92)];
/// The shade behind the left of the screen: dark at the edge, clear by the end of its reach.
const SHADE_STOPS: [(f32, f32); 3] = [(0.0, 0.82), (0.55, 0.5), (1.0, 0.0)];
/// The footer's rules fade out towards their right end.
const RULE_STOPS: [(f32, f32); 3] = [(0.0, 0.8), (0.65, 0.8), (1.0, 0.0)];

/// The Remastered menu component for one menu slot.
pub struct VerticalMenu {
    slot: SlotId,
    cell: MenuListCell,
    parts: Vec<Box<dyn Component>>,
}

impl VerticalMenu {
    pub fn new(slot: SlotId, cell: MenuListCell) -> Self {
        Self { slot, cell, parts: Vec::new() }
    }
}

impl Component for VerticalMenu {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.parts = match self.cell.get(self.slot) {
            Some(list) => Parts::new(cx).of(&list),
            None => Vec::new(),
        };
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// Builds the parts of one list, back to front.
struct Parts<'a, 'b> {
    cx: &'a LayoutCx<'b>,
    parts: Vec<Box<dyn Component>>,
}

impl<'a, 'b> Parts<'a, 'b> {
    fn new(cx: &'a LayoutCx<'b>) -> Self {
        Self { cx, parts: Vec::new() }
    }

    fn of(mut self, list: &MenuList) -> Vec<Box<dyn Component>> {
        let left = place::left_edge(list.aspect);
        self.shade(left);
        if list.style != MenuStyle::Pause {
            self.logo(left);
        }
        self.text(
            keys::FONT_MENU_TITLE,
            &list.title.to_uppercase(),
            keys::MENU_TITLE,
            place::TITLE_SCALE,
            (left + place::TEXT_LEFT, place::TITLE_Y),
            false,
        );
        let shown = place::shown(list.entries.len());
        for (i, entry) in list.entries.iter().skip(list.first).take(shown).enumerate() {
            self.entry(entry, list.first + i == list.selected, place::row_y(i), left);
        }
        self.footer(list, left);
        if list.style == MenuStyle::Main {
            self.tagline(place::right_edge(list.aspect));
        }
        self.parts
    }

    fn push(&mut self, mut part: impl Component + 'static, bounds: Rect) {
        part.layout(self.cx, bounds);
        self.parts.push(Box::new(part));
    }

    fn shade(&mut self, left: f32) {
        let rect = Rect::new(left + place::SHADE_WIDTH / 2.0, 0.0, place::SHADE_WIDTH, 480.0);
        self.parts.push(Box::new(Slant::at_default_depth(keys::MENU_SHADE, rect, 0.0, &SHADE_STOPS, self.cx)));
    }

    fn logo(&mut self, left: f32) {
        let height = place::LOGO_WIDTH / place::LOGO_ASPECT;
        let rect = Rect::new(
            left + place::LOGO_LEFT + place::LOGO_WIDTH / 2.0,
            place::LOGO_TOP + height / 2.0,
            place::LOGO_WIDTH,
            height,
        );
        self.push(Image::new(keys::LOGO), rect);
    }

    /// A string whose left (or, for `right`, right) edge is at `at.0`, centred on `at.1`.
    fn text(&mut self, font: &str, text: &str, colour: &str, scale: f32, at: (f32, f32), right: bool) {
        let width = 400.0;
        let (x, flags) = match right {
            true => (at.0 - width / 2.0, justify::H_RIGHT | justify::V_CENTRE),
            false => (at.0 + width / 2.0, justify::V_CENTRE),
        };
        let text = Text::new(font, text).with_colour_key(colour).with_scale(scale).with_justification(flags);
        self.push(text, Rect::new(x, at.1, width, 20.0));
    }

    fn entry(&mut self, entry: &MenuEntry, selected: bool, y: f32, left: f32) {
        let colour = match (entry.enabled, selected) {
            (false, _) => keys::MENU_TEXT_DISABLED,
            (true, true) => keys::MENU_TEXT_SELECTED,
            (true, false) => keys::MENU_TEXT,
        };
        if selected {
            let (rect, lean) = place::bar(y, left);
            self.parts.push(Box::new(Slant::at_default_depth(keys::MENU_BAR, rect, lean, &BAR_STOPS, self.cx)));
            self.bolt(left + 17.0, y + 1.5);
        }
        let at = (left + place::TEXT_LEFT, y);
        self.text(keys::FONT_MENU, &entry.label.to_uppercase(), colour, place::ENTRY_SCALE, at, false);
    }

    /// The lightning bolt at the selected entry: a leaning upper block, a wider middle and a thin tail.
    fn bolt(&mut self, x: f32, y: f32) {
        let part =
            |parts: &mut Vec<Box<dyn Component>>, cx: &LayoutCx<'_>, dx: f32, dy: f32, w: f32, h: f32, lean: f32| {
                let rect = Rect::new(x + dx, y + dy, w, h);
                parts.push(Box::new(Slant::at_default_depth(keys::MENU_BOLT, rect, lean, &[], cx)));
            };
        part(&mut self.parts, self.cx, 2.0, -6.0, 6.0, 11.0, 5.0);
        part(&mut self.parts, self.cx, 0.0, 0.5, 9.0, 4.0, 3.0);
        part(&mut self.parts, self.cx, -3.0, 7.0, 2.6, 10.0, 6.0);
    }

    fn footer(&mut self, list: &MenuList, left: f32) {
        let spots = place::hint_spots(&list.hints, left);
        let Some(right) = place::rule_right(&spots) else { return };
        let from = left + place::RULE_LEFT;
        for y in [place::RULE_TOP_Y, place::RULE_BOTTOM_Y] {
            let rect = Rect::new((from + right) / 2.0, y, right - from, place::RULE_THICKNESS);
            self.parts.push(Box::new(Slant::at_default_depth(keys::MENU_RULE, rect, 0.0, &RULE_STOPS, self.cx)));
        }
        let tick = Rect::new(
            from + place::RULE_THICKNESS / 2.0,
            place::RULE_BOTTOM_Y - place::RULE_TICK / 2.0,
            place::RULE_THICKNESS,
            place::RULE_TICK,
        );
        self.push(Panel::new(keys::MENU_RULE), tick);
        for (hint, spot) in list.hints.iter().zip(&spots) {
            self.hint(hint.label, spot);
        }
    }

    fn hint(&mut self, label: &str, spot: &HintSpot) {
        for key in &spot.keys {
            match &key.prompt {
                Prompt::Icon(glyph) => self.glyph(*glyph, key.rect),
                Prompt::Key(text) => {
                    let ring = Outline::new(keys::MENU_TEXT, key.rect, place::KEY_RADIUS, place::KEY_EDGE, self.cx);
                    self.parts.push(Box::new(ring));
                    let text = Text::new(keys::FONT_MENU, text.to_uppercase())
                        .with_colour_key(keys::MENU_TEXT)
                        .with_scale(place::KEY_TEXT_SCALE)
                        .with_justification(justify::H_CENTRE | justify::V_CENTRE);
                    self.push(text, Rect::new(key.rect.x, key.rect.y, key.rect.w, 20.0));
                }
            }
        }
        let at = (spot.label_x, spot.area.y);
        self.text(keys::FONT_MENU, label, keys::MENU_TEXT, place::LABEL_SCALE, at, false);
    }

    fn glyph(&mut self, glyph: Glyph, rect: Rect) {
        self.push(Image::new(keys::ATLAS).with_uv(glyph.uv()), rect);
    }

    fn tagline(&mut self, right: f32) {
        for (line, y) in TAGLINE.iter().zip(place::TAGLINE_Y) {
            self.text(
                keys::FONT_MENU_WIDE,
                line,
                keys::MENU_TAGLINE,
                place::TAGLINE_SCALE,
                (right - place::TAGLINE_RIGHT, y),
                true,
            );
        }
    }
}

#[cfg(test)]
mod tests;
