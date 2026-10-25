//! A question or a notice in the Custom look: a surface scrim over the screen, and a plate in the middle of the
//! content box with an accent line along its top, the title in the spaced face, the message wrapped under it and the
//! answers one under the other, the selected one on a raised plate with the accent bar. The same geometry serves the
//! drawing and the frontend's hit test ([`row_at`]).

use blackbox_ui::Rect;
use glam::Vec2;

use super::layout::{BAR_WIDTH, MenuFrame};
use super::list::row_colour;
use super::{Align, Parts};
use crate::ui::raster_text::{CUSTOM_LABEL_FONT_KEY, width_units};
use crate::ui::theme::custom::plate::PlateStyle;
use crate::ui::theme::custom::text::{Label, Sizing, size};
use crate::ui::theme::custom::tokens::keys;
use crate::ui::theme::{MenuEntry, MenuList};

/// The plate's width, the room inside its edges and the rows' sizes, at scale 1 (the heights also grow with the text
/// size).
const WIDTH: f32 = 380.0;
const PAD: f32 = 18.0;
const TITLE_HEIGHT: f32 = 30.0;
const MESSAGE_SIZE: f32 = 13.0;
const LINE_PITCH: f32 = 18.0;
const MESSAGE_GAP: f32 = 12.0;
const ROW_PITCH: f32 = 28.0;
const ROW_GAP: f32 = 4.0;
/// The scrim's alpha over the screen.
const SCRIM: f32 = 0.6;

/// Where the parts of a dialog are, worked out from its text and its number of answers.
#[derive(Clone, Debug, PartialEq)]
pub struct DialogFrame {
    pub plate: Rect,
    pub title_y: Option<f32>,
    pub lines: Vec<String>,
    pub first_line_y: f32,
    pub line_pitch: f32,
    pub rows_y: Vec<f32>,
    pub row_pitch: f32,
}

/// `message` cut into lines no wider than `room` units at `sizing`. `^` is the language table's line break.
pub(super) fn wrap(message: &str, room: f32, sizing: Sizing) -> Vec<String> {
    let scale = sizing.glyph_scale(MESSAGE_SIZE);
    let fits = |text: &str| width_units(CUSTOM_LABEL_FONT_KEY, text, scale).is_none_or(|width| width <= room);
    let mut lines = Vec::new();
    for paragraph in message.split('^') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let longer = match line.is_empty() {
                true => word.to_owned(),
                false => format!("{line} {word}"),
            };
            if fits(&longer) || line.is_empty() {
                line = longer;
                continue;
            }
            lines.push(std::mem::replace(&mut line, word.to_owned()));
        }
        lines.push(line);
    }
    lines
}

impl DialogFrame {
    /// The dialog of a list in `frame`: its title, its message and `answers` answers, centred in the content box.
    pub fn of(frame: &MenuFrame, title: &str, message: &str, answers: usize) -> Self {
        let s = frame.scale();
        let sizing = frame.sizing();
        let width = (WIDTH * s).min(frame.safe().w);
        let pad = PAD * s;
        let lines = match message.is_empty() {
            true => Vec::new(),
            false => wrap(message, width - 2.0 * pad, sizing),
        };
        let line_pitch = sizing.units(LINE_PITCH);
        let row_pitch = sizing.units(ROW_PITCH);
        let title_height = match title.is_empty() {
            true => 0.0,
            false => sizing.units(TITLE_HEIGHT),
        };
        let head = pad + title_height;
        let body = lines.len() as f32 * line_pitch + MESSAGE_GAP * s;
        let height = head + body + answers as f32 * row_pitch + pad;
        let top = frame.safe().y - height / 2.0;
        let title_y = (!title.is_empty()).then_some(top + pad + sizing.units(TITLE_HEIGHT) / 2.0);
        let first_line_y = top + head + line_pitch / 2.0;
        let rows_top = top + head + body;
        let rows_y = (0..answers).map(|i| rows_top + (i as f32 + 0.5) * row_pitch).collect();
        let plate = Rect::new(frame.safe().x, top + height / 2.0, width, height);
        Self { plate, title_y, lines, first_line_y, line_pitch, rows_y, row_pitch }
    }

    /// The plate of the answer whose middle is at `y`.
    pub fn row_rect(&self, y: f32, scale: f32) -> Rect {
        let inset = PAD * scale / 2.0;
        Rect::new(self.plate.x, y, self.plate.w - 2.0 * inset, self.row_pitch - ROW_GAP * scale)
    }
}

/// The answer under a point of the screen, for a dialog list on a window `aspect` wide at `text_size`.
pub fn row_at(point: Vec2, list: &MenuList, aspect: f32, text_size: f32) -> Option<usize> {
    let frame = MenuFrame::new(aspect, text_size);
    let dialog = DialogFrame::of(&frame, &list.title, &list.message, list.entries.len());
    if !dialog.plate.contains(point.x, point.y) {
        return None;
    }
    dialog.rows_y.iter().position(|y| (point.y - y).abs() <= dialog.row_pitch / 2.0)
}

impl Parts<'_, '_> {
    /// The dialog: the scrim, the plate, the title, the message, the answers and the footer's hints.
    pub(super) fn dialog(&mut self, list: &MenuList) {
        let frame = self.frame();
        let dialog = DialogFrame::of(&frame, &list.title, &list.message, list.entries.len());
        self.panel(keys::SURFACE, frame.window(), SCRIM);
        self.plate(dialog.plate, PlateStyle::SURFACE.with_accent(keys::ACCENT));
        let left = dialog.plate.min_x() + PAD * frame.scale();
        if let Some(y) = dialog.title_y {
            let heading = Label::heading(list.title.to_uppercase(), size::MENU_ROW).colour(keys::TEXT);
            self.text(heading, (left, y), Align::Left);
        }
        for (i, line) in dialog.lines.iter().enumerate() {
            let y = dialog.first_line_y + i as f32 * dialog.line_pitch;
            let text = Label::label(line.clone(), MESSAGE_SIZE).colour(keys::TEXT);
            self.text(text, (left, y), Align::Left);
        }
        for (i, (entry, y)) in list.entries.iter().zip(&dialog.rows_y).enumerate() {
            self.answer(entry, i == list.selected, dialog.row_rect(*y, frame.scale()));
        }
        self.footer(&list.hints);
    }

    /// One answer: a raised plate with the accent bar under the selected one, and its text in the middle.
    fn answer(&mut self, entry: &MenuEntry, selected: bool, rect: Rect) {
        if selected {
            self.plate(rect, PlateStyle::RAISED);
            let bar = BAR_WIDTH * self.frame().scale();
            self.panel(keys::ACCENT, Rect::new(rect.min_x() + bar / 2.0, rect.y, bar, rect.h), 1.0);
        }
        let (colour, alpha) = row_colour(entry, selected);
        let label = Label::label(entry.label.clone(), size::MENU_ROW).colour(colour).opacity(alpha);
        self.text(label, (rect.x, rect.y), Align::Centre);
    }
}
