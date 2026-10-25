//! The value of a settings row, in its column at the right of the row: a value that left and right cycle sits between
//! two small chevrons (the one last pressed is nudged outward for a moment); a value that only does something when
//! pressed is right-aligned; a slider is a 10-segment bar like the HUD's, with its number at the right.

use blackbox_ui::{Rect, bounce};

use super::list::{DISABLED, row_colour};
use super::{Align, Parts};
use crate::i18n::fmt::percent;
use crate::ui::theme::custom::text::{Label, size};
use crate::ui::theme::custom::tokens::keys;
use crate::ui::theme::{MenuEntry, MenuValue};

/// The chevrons either side of a cycling value.
const LEFT_CHEVRON: &str = "\u{2039}";
const RIGHT_CHEVRON: &str = "\u{203A}";
/// A slider's segments: how many, the gap between two, their height, and the room kept right of them for the number.
pub(super) const SEGMENTS: usize = 10;
const SEGMENT_GAP: f32 = 2.0;
const SEGMENT_HEIGHT: f32 = 6.0;
const NUMBER_ROOM: f32 = 44.0;
/// The alpha of an idle row's chevrons.
const IDLE_CHEVRON: f32 = 0.6;

impl Parts<'_, '_> {
    /// Draws the value of `entry` in its column on the row at `y`. `nudge` is `(forward, age)` of the last press on
    /// this row.
    pub(super) fn value(&mut self, entry: &MenuEntry, selected: bool, y: f32, nudge: Option<(bool, f32)>) {
        let column = self.frame().value_column(y);
        let (colour, alpha) = row_colour(entry, selected);
        match &entry.value {
            MenuValue::None | MenuValue::Rival(_) => {}
            MenuValue::Choice { text, arrows: true } => self.choice(text, column, (colour, alpha), entry, nudge),
            MenuValue::Choice { text, arrows: false } => {
                let label = Label::label(text.clone(), size::MENU_ROW).colour(colour).opacity(alpha);
                self.text(label, (column.max_x(), y), Align::Right);
            }
            MenuValue::Slider(level) => self.slider(*level, column, (colour, alpha), selected && entry.enabled),
            MenuValue::Keys { primary, secondary, .. } => {
                let keys = format!("{primary}  \u{00B7}  {secondary}");
                let label = Label::label(keys, size::MENU_ROW).colour(colour).opacity(alpha);
                self.text(label, (column.max_x(), y), Align::Right);
            }
        }
    }

    /// A cycling value: the text in the column's middle, a chevron at each end. The selected row's chevrons are in the
    /// accent colour.
    fn choice(
        &mut self,
        text: &str,
        column: Rect,
        (colour, alpha): (&'static str, f32),
        entry: &MenuEntry,
        nudge: Option<(bool, f32)>,
    ) {
        let y = column.y;
        let label = Label::label(text.to_owned(), size::MENU_ROW).colour(colour).opacity(alpha);
        self.text(label, (column.x, y), Align::Centre);
        let live = colour == keys::TEXT && entry.enabled;
        let (chevron, chevron_alpha) = match (live, entry.enabled) {
            (true, _) => (keys::ACCENT, 1.0),
            (false, true) => (keys::TEXT_SECONDARY, IDLE_CHEVRON),
            (false, false) => (keys::TEXT_SECONDARY, DISABLED * IDLE_CHEVRON),
        };
        let push = |forward: bool| match nudge {
            Some((way, age)) if way == forward => bounce(age),
            _ => 0.0,
        };
        let inset = self.frame().scale() * 6.0;
        let left = Label::label(LEFT_CHEVRON, size::MENU_ROW).colour(chevron).opacity(chevron_alpha);
        self.text(left, (column.min_x() + inset - push(false), y), Align::Left);
        let right = Label::label(RIGHT_CHEVRON, size::MENU_ROW).colour(chevron).opacity(chevron_alpha);
        self.text(right, (column.max_x() - inset + push(true), y), Align::Right);
    }

    /// A level from 0 to 100 as ten segments, the filled ones in the accent colour on the selected row, and the number.
    fn slider(&mut self, level: u8, column: Rect, (colour, alpha): (&'static str, f32), live: bool) {
        let s = self.frame().scale();
        let left = column.min_x();
        let width = column.w - NUMBER_ROOM * s;
        let gap = SEGMENT_GAP * s;
        let segment = (width - gap * (SEGMENTS - 1) as f32) / SEGMENTS as f32;
        let filled = filled_segments(level);
        let fill = match live {
            true => keys::ACCENT,
            false => keys::TEXT_SECONDARY,
        };
        for i in 0..SEGMENTS {
            let x = left + i as f32 * (segment + gap) + segment / 2.0;
            let rect = Rect::new(x, column.y, segment, SEGMENT_HEIGHT * s);
            let key = match i < filled {
                true => fill,
                false => keys::RULE,
            };
            self.panel(key, rect, alpha);
        }
        let number = Label::numeric(percent(i64::from(level)), size::MENU_ROW).colour(colour).opacity(alpha);
        self.text(number, (column.max_x(), column.y), Align::Right);
    }
}

/// How many of the ten segments a level from 0 to 100 fills, to the nearest.
pub(super) fn filled_segments(level: u8) -> usize {
    (usize::from(level.min(100)) + 5) / 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_fills_the_nearest_number_of_segments() {
        assert_eq!(filled_segments(0), 0);
        assert_eq!(filled_segments(4), 0);
        assert_eq!(filled_segments(5), 1);
        assert_eq!(filled_segments(80), 8);
        assert_eq!(filled_segments(100), SEGMENTS);
        assert_eq!(filled_segments(255), SEGMENTS, "a level past 100 fills the bar and no more");
    }
}
