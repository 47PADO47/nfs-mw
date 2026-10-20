//! Where the Remastered vertical menu puts things, in FEng units (480 high, origin at the centre, y down). It is placed
//! from the reference picture, 1600 by 915 pixels: a picture pixel is `PX` units on a screen that is 480 units tall.
//! Horizontal places that belong to the left of the screen are measured from the left edge, which depends on the window's
//! aspect (`left_edge`); vertical places do not.
//!
//! The geometry is shared with the frontend's hit tests (`row_at`, `hint_at`), so a click lands on what is drawn.

use blackbox_ui::Rect;
use glam::Vec2;

use crate::input::Action;
use crate::ui::prompts::Prompt;
use crate::ui::raster_text::{RASTER_MENU_FONT_KEY, width_units};
use crate::ui::theme::MenuHint;

pub use super::blacklist::row_at as blacklist_row_at;
pub use super::dialog::row_at as dialog_row_at;

/// FEng units per pixel of the reference picture.
const PX: f32 = 480.0 / 915.0;
/// The picture's vertical centre, in pixels: the screen's y origin.
const CENTRE_Y_PX: f32 = 457.5;
const HALF_HEIGHT: f32 = 240.0;

/// The screen's left edge for a width over height ratio.
pub fn left_edge(aspect: f32) -> f32 {
    -aspect * HALF_HEIGHT
}

/// The screen's right edge for a width over height ratio.
pub fn right_edge(aspect: f32) -> f32 {
    aspect * HALF_HEIGHT
}

const fn y_of(pixel: f32) -> f32 {
    (pixel - CENTRE_Y_PX) * PX
}

// The text column and the type sizes. A text scale is 10 units of image height; the cap height is about 0.62 of that.
/// Where the title and the entries start, from the left edge.
pub const TEXT_LEFT: f32 = 67.0 * PX;
pub const ENTRY_SCALE: f32 = 1.88;
pub const TITLE_SCALE: f32 = 2.15;
pub const TITLE_Y: f32 = (294.0 - CENTRE_Y_PX) * PX;
pub const TAGLINE_SCALE: f32 = 1.65;
/// The tagline's right edge, from the right screen edge, and the y of its two lines.
pub const TAGLINE_RIGHT: f32 = 83.0 * PX;
pub const TAGLINE_Y: [f32; 2] = [y_of(673.0), y_of(715.0)];

// The entries.
const FIRST_ROW_Y: f32 = (369.0 - CENTRE_Y_PX) * PX;
const ROW_PITCH: f32 = 61.4 * PX;
/// How many entries show at once. A longer list scrolls: it stays above the footer.
pub const MAX_ROWS: usize = 8;
/// The highlight bar: its middle-height span from the left edge, its height, and how far its top leans right of its bottom.
const BAR_LEFT: f32 = 33.5 * PX;
pub const BAR_RIGHT: f32 = 693.5 * PX;
/// The settings pages' bar is wider, to hold the values: its right end, and the value column inside it.
pub const WIDE_BAR_RIGHT: f32 = 500.0;
const VALUE_LEFT: f32 = 322.0;
const VALUE_RIGHT: f32 = 484.0;
/// How wide the chevron zone at each end of the value column is, for a click.
const CHEVRON_ZONE: f32 = 26.0;
const BAR_HEIGHT: f32 = 55.0 * PX;
const BAR_LEAN: f32 = 23.0 * PX;

/// The y of the middle of the `i`th shown entry.
pub fn row_y(i: usize) -> f32 {
    FIRST_ROW_Y + ROW_PITCH * i as f32
}

/// The highlight bar of the entry at `y`, as the middle-height rectangle and the lean of its edges. A settings page's bar
/// (`wide`) runs on to hold the values.
pub fn bar(y: f32, left: f32, wide: bool) -> (Rect, f32) {
    let right = bar_right(wide);
    let rect = Rect::new(left + (BAR_LEFT + right) / 2.0, y, right - BAR_LEFT, BAR_HEIGHT);
    (rect, BAR_LEAN)
}

fn bar_right(wide: bool) -> f32 {
    match wide {
        true => WIDE_BAR_RIGHT,
        false => BAR_RIGHT,
    }
}

/// The column of a settings row's value, at `y`.
pub fn value_column(y: f32, left: f32) -> Rect {
    Rect::new(left + (VALUE_LEFT + VALUE_RIGHT) / 2.0, y, VALUE_RIGHT - VALUE_LEFT, BAR_HEIGHT)
}

/// How far a label may run before it meets the value column, from the label's left.
pub const LABEL_LIMIT: f32 = VALUE_LEFT - TEXT_LEFT - 8.0;

/// Where a click on a settings row lands, by its place along the row from the left edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// On the left chevron.
    Left,
    /// Anywhere else on the row.
    Middle,
    /// On the right chevron.
    Right,
}

/// How far along the value column a point `from_left` units from the screen's left edge is, 0 at the column's left and 1 at
/// its right, clamped.
pub fn along_value(from_left: f32) -> f32 {
    ((from_left - VALUE_LEFT) / (VALUE_RIGHT - VALUE_LEFT)).clamp(0.0, 1.0)
}

/// The side of a settings row a point `from_left` units from the screen's left edge is on.
pub fn side_at(from_left: f32) -> Side {
    match from_left {
        x if (VALUE_LEFT..VALUE_LEFT + CHEVRON_ZONE).contains(&x) => Side::Left,
        x if (VALUE_RIGHT - CHEVRON_ZONE..=VALUE_RIGHT).contains(&x) => Side::Right,
        _ => Side::Middle,
    }
}

/// How many of `count` entries show: all of them up to [`MAX_ROWS`].
pub fn shown(count: usize) -> usize {
    count.min(MAX_ROWS)
}

/// The first entry to show so the selected one is in view, given the entry shown first until now: the window stays where it
/// is while the selection moves inside it, and slides by one entry when the selection passes either end.
pub fn window_start(first: usize, selected: usize, count: usize) -> usize {
    let first = match selected {
        s if s < first => s,
        s if s >= first + MAX_ROWS => s + 1 - MAX_ROWS,
        _ => first,
    };
    first.min(count.saturating_sub(MAX_ROWS))
}

/// The entry (an index into the shown ones) under a point of the screen (FEng units), for a list that shows `shown` entries
/// on a screen of this aspect. The whole band of a row answers, from the left edge to the end of the bar.
pub fn row_at(point: Vec2, shown: usize, aspect: f32, wide: bool) -> Option<usize> {
    if point.x > left_edge(aspect) + bar_right(wide) {
        return None;
    }
    (0..shown).find(|&i| (point.y - row_y(i)).abs() <= ROW_PITCH / 2.0)
}

/// The logo: from the left edge, the top, and the width. Its height follows the picture's own proportions.
pub const LOGO_LEFT: f32 = 50.0 * PX;
pub const LOGO_TOP: f32 = y_of(62.0);
pub const LOGO_WIDTH: f32 = 367.0;
/// The logo's picture is 512 by 103.
pub const LOGO_ASPECT: f32 = 512.0 / 103.0;

/// The dark shade behind the left of the screen: how far it reaches from the left edge.
pub const SHADE_WIDTH: f32 = 460.0;

// The footer.
pub const RULE_TOP_Y: f32 = y_of(832.0);
pub const RULE_BOTTOM_Y: f32 = y_of(905.0);
pub const RULE_LEFT: f32 = 44.0 * PX;
/// The rule's thickness, and how far it runs past the last hint.
pub const RULE_THICKNESS: f32 = 0.7;
const RULE_TAIL: f32 = 80.0;
/// The bottom rule's short upright tick at its left end.
pub const RULE_TICK: f32 = 6.0;
const HINT_Y: f32 = y_of(868.5);
const KEY_LEFT: f32 = 65.0 * PX;
pub const KEY_HEIGHT: f32 = 43.0 * PX;
pub const KEY_RADIUS: f32 = 4.2;
pub const KEY_EDGE: f32 = 1.2;
/// The space each side of a key cap's text.
const KEY_PAD: f32 = 7.0;
pub const KEY_TEXT_SCALE: f32 = 1.6;
pub const LABEL_SCALE: f32 = 1.88;
/// A controller glyph's size.
const GLYPH: f32 = 25.0;
/// The space between a key and the next key, between the keys and the label, and between two hints.
const KEY_GAP: f32 = 5.0;
const LABEL_GAP: f32 = 21.0 * PX;
const HINT_GAP: f32 = 26.0;

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

/// The width of `text` in the menu face at `scale`. An estimate stands in when the font did not load.
fn text_width(text: &str, scale: f32) -> f32 {
    width_units(RASTER_MENU_FONT_KEY, text, scale).unwrap_or(text.chars().count() as f32 * 3.4 * scale)
}

/// Lays out the footer hints from the left edge: each hint's keys, then its label, with a gap before the next hint.
pub fn hint_spots(hints: &[MenuHint], left: f32) -> Vec<HintSpot> {
    let mut x = left + KEY_LEFT;
    let mut spots = Vec::with_capacity(hints.len());
    for hint in hints {
        let start = x;
        let mut keys = Vec::with_capacity(hint.keys.len());
        for prompt in &hint.keys {
            let (width, height) = match prompt {
                Prompt::Icon(_) => (GLYPH, GLYPH),
                Prompt::Key(text) => (text_width(text, KEY_TEXT_SCALE) + 2.0 * KEY_PAD, KEY_HEIGHT),
            };
            keys.push(KeySpot { rect: Rect::new(x + width / 2.0, HINT_Y, width, height), prompt: prompt.clone() });
            x += width + KEY_GAP;
        }
        let label_x = x - KEY_GAP + LABEL_GAP;
        let end = label_x + text_width(hint.label, LABEL_SCALE);
        let area = Rect::new((start + end) / 2.0, HINT_Y, end - start + 6.0, KEY_HEIGHT + 4.0);
        spots.push(HintSpot { keys, label_x, area });
        x = end + HINT_GAP;
    }
    spots
}

/// How far the footer's rules run to the right of the left edge, for these hints (the hints' end plus a tail).
pub fn rule_right(spots: &[HintSpot]) -> Option<f32> {
    spots.last().map(|spot| spot.area.max_x() + RULE_TAIL)
}

/// The hint under a point of the screen whose click presses an action.
pub fn hint_at(hints: &[MenuHint], point: Vec2, aspect: f32) -> Option<Action> {
    let spots = hint_spots(hints, left_edge(aspect));
    hints.iter().zip(&spots).find(|(_, spot)| spot.area.contains(point.x, point.y)).and_then(|(hint, _)| hint.action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::input_icons::Glyph;

    fn hint(keys: Vec<Prompt>, label: &'static str, action: Option<Action>) -> MenuHint {
        MenuHint { keys, label, action }
    }

    #[test]
    fn the_left_edge_follows_the_aspect() {
        assert_eq!(left_edge(4.0 / 3.0), -320.0);
        assert!((left_edge(16.0 / 9.0) + 426.67).abs() < 0.01);
        assert_eq!(right_edge(2.0), 480.0);
    }

    #[test]
    fn the_rows_are_the_reference_spacing_and_the_eighth_stays_above_the_footer() {
        assert!((row_y(0) - FIRST_ROW_Y).abs() < 1e-4);
        assert!((row_y(1) - row_y(0) - ROW_PITCH).abs() < 1e-4);
        assert!(row_y(MAX_ROWS - 1) + BAR_HEIGHT / 2.0 < RULE_TOP_Y, "the last shown bar clears the footer's rule");
        assert_eq!((shown(3), shown(8), shown(20)), (3, 8, MAX_ROWS));
    }

    #[test]
    fn the_window_stays_while_the_selection_moves_inside_it_and_slides_one_at_a_time() {
        assert_eq!(window_start(0, 3, 12), 0, "inside the window");
        assert_eq!(window_start(0, 7, 12), 0, "the last shown entry");
        assert_eq!(window_start(0, 8, 12), 1, "past the bottom: the first entry goes");
        assert_eq!(window_start(1, 9, 12), 2);
        assert_eq!(window_start(4, 5, 12), 4, "going back up inside the window changes nothing");
        assert_eq!(window_start(4, 3, 12), 3, "past the top: the window slides back");
        assert_eq!(window_start(2, 11, 12), 4, "a jump to the end shows the last entries");
        assert_eq!(window_start(3, 1, 5), 0, "a list that fits never scrolls");
    }

    #[test]
    fn a_row_answers_across_its_band_up_to_the_end_of_the_bar() {
        let aspect = 16.0 / 9.0;
        let left = left_edge(aspect);
        assert_eq!(row_at(Vec2::new(left + 100.0, row_y(3)), 8, aspect, false), Some(3));
        assert_eq!(row_at(Vec2::new(left + 5.0, row_y(0) + ROW_PITCH * 0.49), 8, aspect, false), Some(0));
        assert_eq!(row_at(Vec2::new(left + BAR_RIGHT + 1.0, row_y(3)), 8, aspect, false), None, "right of the bar");
        let past = Vec2::new(left + BAR_RIGHT + 1.0, row_y(3));
        assert_eq!(row_at(past, 8, aspect, true), Some(3), "a settings row's bar runs on");
        assert_eq!(row_at(Vec2::new(left + 100.0, row_y(7) + ROW_PITCH), 8, aspect, false), None, "below the list");
        assert_eq!(row_at(Vec2::new(left + 100.0, row_y(4)), 3, aspect, false), None, "only the shown entries answer");
        assert_eq!(row_at(Vec2::new(left + 100.0, -200.0), 8, aspect, false), None, "above the list");
    }

    #[test]
    fn a_click_on_a_settings_row_lands_on_a_chevron_or_the_middle() {
        assert_eq!(side_at(100.0), Side::Middle, "on the label");
        assert_eq!(side_at(VALUE_LEFT + 5.0), Side::Left);
        assert_eq!(side_at(VALUE_RIGHT - 5.0), Side::Right);
        assert_eq!(side_at((VALUE_LEFT + VALUE_RIGHT) / 2.0), Side::Middle, "on the value");
        assert!(LABEL_LIMIT > 200.0, "a long label still fits before the values");
    }

    #[test]
    fn hints_follow_each_other_and_a_click_presses_the_one_under_it() {
        let hints = [
            hint(vec![Prompt::Key("Enter".into())], "Accept", Some(Action::MenuAccept)),
            hint(vec![Prompt::Icon(Glyph::Vertical)], "Select", None),
            hint(vec![Prompt::Key("Esc".into())], "Back", Some(Action::MenuBack)),
        ];
        let aspect = 16.0 / 9.0;
        let spots = hint_spots(&hints, left_edge(aspect));
        assert_eq!(spots.len(), 3);
        assert!(spots[0].area.max_x() < spots[1].area.min_x() && spots[1].area.max_x() < spots[2].area.min_x());
        let at = |spot: &HintSpot| Vec2::new(spot.area.x, spot.area.y);
        assert_eq!(hint_at(&hints, at(&spots[0]), aspect), Some(Action::MenuAccept));
        assert_eq!(hint_at(&hints, at(&spots[1]), aspect), None, "a pair of arrows presses nothing");
        assert_eq!(hint_at(&hints, at(&spots[2]), aspect), Some(Action::MenuBack));
        assert_eq!(hint_at(&hints, Vec2::new(0.0, 0.0), aspect), None);
        assert!(rule_right(&spots).is_some_and(|right| right > spots[2].area.max_x()));
        assert_eq!(rule_right(&[]), None);
    }
}
