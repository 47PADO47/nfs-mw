//! A question or a notice in the Remastered look: a dark panel in the middle of the screen over a scrim, with the title in the
//! accent colour, the message wrapped under it, and the answers as slanted bars, one under the other. The same geometry
//! serves the drawing and the frontend's hit test.

use blackbox_ui::{Panel, Rect, Slant};
use glam::Vec2;

use super::Parts;
use crate::ui::raster_text::{RASTER_MENU_FONT_KEY, width_units};
use crate::ui::theme::remastered::skin::keys;
use crate::ui::theme::{MenuEntry, MenuList};

/// The panel's width, the room inside its edges, and the rows' sizes, in FEng units.
const WIDTH: f32 = 420.0;
const PAD: f32 = 24.0;
const TITLE_HEIGHT: f32 = 30.0;
const LINE_PITCH: f32 = 19.0;
const MESSAGE_SCALE: f32 = 1.55;
const MESSAGE_GAP: f32 = 14.0;
const ROW_PITCH: f32 = 30.0;
const ROW_HEIGHT: f32 = 25.0;
const BOTTOM_PAD: f32 = 16.0;
/// The panel sits a little above the middle of the screen.
const CENTRE_Y: f32 = -14.0;
/// The panel's shade and the thin accent lines along its top and bottom.
const PANEL_STOPS: [(f32, f32); 3] = [(0.0, 0.97), (0.5, 0.99), (1.0, 0.97)];
const EDGE_LINE: f32 = 1.2;
/// The bars' ramp, as the menus' selected bar.
const BAR_STOPS: [(f32, f32); 5] = [(0.0, 0.1), (0.3, 0.3), (0.65, 0.55), (0.9, 0.75), (1.0, 0.92)];
const LEAN: f32 = 9.0;

/// Where the parts of a dialog are, worked out from its text and its number of answers.
pub struct Geometry {
    pub panel: Rect,
    pub title_y: Option<f32>,
    pub lines: Vec<String>,
    pub first_line_y: f32,
    pub rows_y: Vec<f32>,
}

/// The message cut into lines no wider than the panel's inside. `^` is the language table's line break.
fn wrap(message: &str) -> Vec<String> {
    let room = WIDTH - 2.0 * PAD;
    let fits = |text: &str| width_units(RASTER_MENU_FONT_KEY, text, MESSAGE_SCALE).is_none_or(|width| width <= room);
    let mut lines = Vec::new();
    for paragraph in message.split('^') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let longer = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
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

impl Geometry {
    /// The dialog of a list: its title, its message and `list.entries.len()` answers.
    pub fn of(title: &str, message: &str, answers: usize) -> Self {
        let lines = wrap(message);
        let titled = !title.is_empty();
        let head = PAD + if titled { TITLE_HEIGHT } else { 0.0 };
        let height = head + lines.len() as f32 * LINE_PITCH + MESSAGE_GAP + answers as f32 * ROW_PITCH + BOTTOM_PAD;
        let top = CENTRE_Y - height / 2.0;
        let title_y = titled.then_some(top + PAD + TITLE_HEIGHT / 2.0 - 4.0);
        let first_line_y = top + head + LINE_PITCH / 2.0;
        let rows_top = top + head + lines.len() as f32 * LINE_PITCH + MESSAGE_GAP;
        let rows_y = (0..answers).map(|i| rows_top + (i as f32 + 0.5) * ROW_PITCH).collect();
        Self { panel: Rect::new(0.0, CENTRE_Y, WIDTH, height), title_y, lines, first_line_y, rows_y }
    }
}

/// The answer under a point of the screen, for a dialog of this title, message and number of answers.
pub fn row_at(point: Vec2, list: &MenuList) -> Option<usize> {
    let geometry = Geometry::of(&list.title, &list.message, list.entries.len());
    if !geometry.panel.contains(point.x, point.y) {
        return None;
    }
    geometry.rows_y.iter().position(|y| (point.y - y).abs() <= ROW_PITCH / 2.0)
}

impl Parts<'_, '_> {
    /// The dialog: a scrim, the panel with its two accent lines, the title, the message and the answers.
    pub(super) fn dialog(&mut self, list: &MenuList) {
        let geometry = Geometry::of(&list.title, &list.message, list.entries.len());
        let panel = geometry.panel;
        let scrim = Rect::new(0.0, 0.0, list.aspect * 480.0 + 4.0, 484.0);
        self.push(Panel::new(keys::MENU_SCRIM), scrim);
        self.add(Slant::at_default_depth(keys::MAP_GROUND, panel, 0.0, &PANEL_STOPS, self.cx));
        for y in [panel.min_y(), panel.max_y() - EDGE_LINE] {
            self.push(Panel::new(keys::MENU_TITLE), Rect::new(panel.x, y + EDGE_LINE / 2.0, panel.w, EDGE_LINE));
        }
        let left = panel.min_x() + PAD;
        if let Some(y) = geometry.title_y {
            self.text(keys::FONT_MENU_TITLE, &list.title.to_uppercase(), keys::MENU_TITLE, 1.9, (left, y), false);
        }
        for (i, line) in geometry.lines.iter().enumerate() {
            let y = geometry.first_line_y + i as f32 * LINE_PITCH;
            self.text(keys::FONT_MENU, line, keys::MENU_TEXT, MESSAGE_SCALE, (left, y), false);
        }
        for (i, (entry, y)) in list.entries.iter().zip(&geometry.rows_y).enumerate() {
            self.answer(entry, i == list.selected, *y, panel);
        }
    }

    /// One answer: a bar under the selected one, and its text.
    fn answer(&mut self, entry: &MenuEntry, selected: bool, y: f32, panel: Rect) {
        let colour = match (entry.enabled, selected) {
            (false, _) => keys::MENU_TEXT_DISABLED,
            (true, true) => keys::MENU_TEXT_SELECTED,
            (true, false) => keys::MENU_TEXT,
        };
        if selected {
            let bar = Rect::new(panel.x, y, panel.w - 2.0 * (PAD - 10.0), ROW_HEIGHT);
            self.add(Slant::at_default_depth(keys::MENU_BAR, bar, LEAN, &BAR_STOPS, self.cx));
        }
        let at = (panel.min_x() + PAD + 12.0, y);
        self.text(keys::FONT_MENU, &entry.label.to_uppercase(), colour, 1.75, at, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_message_is_one_line_and_a_long_one_wraps_within_the_panel() {
        assert_eq!(wrap("Quit?"), ["Quit?"]);
        let long = "Are you sure you want to quit to the main menu? Anything you have not saved will be lost for good.";
        let lines = wrap(long);
        assert!(lines.len() >= 2, "{lines:?}");
        assert_eq!(lines.join(" "), long, "no word is lost or cut");
    }

    #[test]
    fn the_language_tables_line_break_starts_a_new_line() {
        assert_eq!(wrap("Go to My Cars^to add cars"), ["Go to My Cars", "to add cars"]);
    }

    #[test]
    fn the_answers_stack_under_the_message_inside_the_panel_which_grows_with_both() {
        let one = Geometry::of("Title", "Short", 2);
        let two = Geometry::of("Title", "Short^and more^and more", 2);
        assert!(two.panel.h > one.panel.h, "a longer message makes a taller panel");
        assert!(one.rows_y[1] - one.rows_y[0] - ROW_PITCH < 1e-4);
        assert!(one.rows_y[0] > one.first_line_y && one.rows_y[1] < one.panel.max_y());
        assert!(one.title_y.is_some() && Geometry::of("", "Short", 1).title_y.is_none());
    }
}
