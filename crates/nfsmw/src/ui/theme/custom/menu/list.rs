//! A menu as a list down the left of the content box: a surface scrim behind it for contrast (the whole screen for the
//! pause menu, whose world stays frozen behind it), the title in the spaced face with a short accent rule, the rows, a
//! scroll track when the list is longer than a screenful, the description line and the footer's hints.
//!
//! A row is its label in the neutral face, in the secondary text colour. The selected row is in the primary colour on
//! a raised plate with an accent bar on its left; a disabled row is the secondary colour at half alpha.

use blackbox_ui::Rect;

use super::layout::{self, BAR_WIDTH};
use super::{Align, Parts};
use crate::ui::raster_text::{CUSTOM_LABEL_FONT_KEY, width_units};
use crate::ui::theme::custom::plate::PlateStyle;
use crate::ui::theme::custom::text::{Label, Sizing, size};
use crate::ui::theme::custom::tokens::keys;
use crate::ui::theme::{MenuEntry, MenuList, MenuStyle};

/// The scrim's alpha behind a list, and over the whole frozen world behind the pause menu.
const LIST_SCRIM: f32 = 0.55;
const PAUSE_SCRIM: f32 = 0.7;
/// How far the list's scrim reaches past the rows' plates.
const SCRIM_REACH: f32 = 28.0;
/// A disabled row's alpha.
pub(super) const DISABLED: f32 = 0.5;
/// The rule under the title: its width, its thickness and its drop below the title's middle.
const TITLE_RULE: (f32, f32, f32) = (32.0, 2.0, 16.0);
/// The description line's size.
pub(super) const DESCRIPTION: f32 = 12.0;
/// The scroll track: its gap from the rows' plates and its width.
const TRACK_GAP: f32 = 6.0;
const TRACK_WIDTH: f32 = 2.0;

/// The colour key and alpha of a row's text: the primary colour when selected, the secondary otherwise, at half alpha
/// when disabled.
pub(super) fn row_colour(entry: &MenuEntry, selected: bool) -> (&'static str, f32) {
    match (entry.enabled, selected) {
        (false, _) => (keys::TEXT_SECONDARY, DISABLED),
        (true, true) => (keys::TEXT, 1.0),
        (true, false) => (keys::TEXT_SECONDARY, 1.0),
    }
}

/// The row size, or less when the label at that size is longer than `room` units, so it stops short of the values. An
/// estimate stands in for the width when the font did not load, so a label is never shrunk on a guess.
fn fitted(label: &str, room: f32, sizing: Sizing) -> f32 {
    let row = size::MENU_ROW;
    match width_units(CUSTOM_LABEL_FONT_KEY, label, sizing.glyph_scale(row)) {
        Some(width) if width > room && width > 0.0 => row * room / width,
        _ => row,
    }
}

impl Parts<'_, '_> {
    /// The list, with the selection plate at `plate_y` (where its glide has it), or no plate when the selected entry is
    /// out of view.
    pub(super) fn list(&mut self, list: &MenuList, plate_y: Option<f32>) {
        let frame = self.frame();
        let wide = list.has_values();
        self.scrim(list.style, wide);
        self.title(&list.title);
        if let Some(y) = plate_y {
            self.selection(y, wide);
        }
        let shown = layout::shown(list.entries.len().saturating_sub(list.first));
        for (i, entry) in list.entries.iter().skip(list.first).take(shown).enumerate() {
            let row = list.first + i;
            let selected = row == list.selected;
            let y = frame.row_y(i);
            let (colour, alpha) = row_colour(entry, selected);
            let fit = fitted(&entry.label, frame.label_room(wide), frame.sizing());
            let label = Label::label(entry.label.clone(), fit).colour(colour).opacity(alpha);
            self.text(label, (frame.text_left(), y), Align::Left);
            if !wide {
                continue;
            }
            let nudge = list.nudge.filter(|nudge| nudge.row == row).map(|nudge| (nudge.forward, nudge.age));
            self.value(entry, selected, y, nudge);
        }
        self.scroll_track(list, shown, wide);
        if !list.message.is_empty() {
            let line = Label::label(list.message.clone(), DESCRIPTION).colour(keys::TEXT_SECONDARY);
            self.text(line, (frame.text_left(), frame.footer_y()), Align::Left);
        }
        self.footer(&list.hints);
    }

    /// The surface behind the list: a column down the left, or the whole window for the pause menu.
    fn scrim(&mut self, style: MenuStyle, wide: bool) {
        let frame = self.frame();
        let window = frame.window();
        if style == MenuStyle::Pause {
            self.panel(keys::SURFACE, window, PAUSE_SCRIM);
            return;
        }
        let right = frame.row_right(wide) + SCRIM_REACH * frame.scale();
        let rect = Rect::new((window.min_x() + right) / 2.0, 0.0, right - window.min_x(), window.h);
        self.panel(keys::SURFACE, rect, LIST_SCRIM);
    }

    /// The title in the spaced face, and the short accent rule under it.
    fn title(&mut self, title: &str) {
        let frame = self.frame();
        if title.is_empty() {
            return;
        }
        let heading = Label::heading(title.to_uppercase(), size::MENU_TITLE).colour(keys::TEXT);
        self.text(heading, (frame.text_left(), frame.title_y()), Align::Left);
        let (width, thickness, drop) = TITLE_RULE;
        let s = frame.scale();
        let sizing = frame.sizing();
        let rule = Rect::new(
            frame.text_left() + width * s / 2.0,
            frame.title_y() + sizing.units(drop),
            width * s,
            thickness * s,
        );
        self.panel(keys::ACCENT, rule, 1.0);
    }

    /// The selected row's raised plate at `y`, with the accent bar on its left.
    fn selection(&mut self, y: f32, wide: bool) {
        let frame = self.frame();
        let rect = frame.row_rect(y, wide);
        self.plate(rect, PlateStyle::RAISED);
        let bar = BAR_WIDTH * frame.scale();
        self.panel(keys::ACCENT, Rect::new(rect.min_x() + bar / 2.0, y, bar, rect.h), 1.0);
    }

    /// A thin track right of the rows with a thumb for the shown part, when the list is longer than a screenful.
    fn scroll_track(&mut self, list: &MenuList, shown: usize, wide: bool) {
        let count = list.entries.len();
        if shown == 0 || count <= shown {
            return;
        }
        let frame = self.frame();
        let s = frame.scale();
        let x = frame.row_right(wide) + (TRACK_GAP + TRACK_WIDTH / 2.0) * s;
        let top = frame.list_top();
        let height = frame.pitch() * layout::MAX_ROWS as f32;
        self.panel(keys::RULE, Rect::new(x, top + height / 2.0, TRACK_WIDTH * s, height), 1.0);
        let thumb = height * shown as f32 / count as f32;
        let thumb_top = top + height * list.first as f32 / count as f32;
        self.panel(keys::ACCENT, Rect::new(x, thumb_top + thumb / 2.0, TRACK_WIDTH * s, thumb), 1.0);
    }
}
