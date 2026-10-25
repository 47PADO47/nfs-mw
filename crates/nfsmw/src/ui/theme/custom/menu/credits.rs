//! The credits roll (`Credits`): a surface scrim over the whole window, the title at the top left like any list, and the
//! lines of the roll centred in the content box between the title and the footer, scrolling up, fading in at the
//! bottom and out at the top. A line whose letters are all capitals is a heading, in the spaced face and the accent
//! colour; the others are names in the neutral face.
//!
//! The frontend hands the window of lines that can show (`list.message`, one line per line of text) and how far the
//! roll has moved past the first of them (`list.scroll`, a fraction of a line).

use super::{Align, Parts};
use crate::ui::theme::MenuList;
use crate::ui::theme::custom::text::Label;
use crate::ui::theme::custom::tokens::keys;

/// The scrim's alpha over the window.
const SCRIM: f32 = 0.7;
/// The distance between two lines, and the sizes of a name and a heading, at text size 1.
const PITCH: f32 = 22.0;
const NAME_SIZE: f32 = 15.0;
const HEADING_SIZE: f32 = 13.0;
/// The room kept above the footer line, and how far from either end of the roll a line starts to fade, at scale 1.
const FOOTER_GAP: f32 = 24.0;
const FADE: f32 = 40.0;

/// Whether a line of the roll is a heading: it has letters, and none of them is lower case.
pub(super) fn is_heading(line: &str) -> bool {
    let mut letters = line.chars().filter(|c| c.is_alphabetic()).peekable();
    letters.peek().is_some() && letters.all(|c| !c.is_lowercase())
}

/// The opacity of a line at height `y` on a roll from `top` to `bottom`: 1 in the middle, falling to 0 over `fade`
/// units at either end, and 0 past them.
pub(super) fn opacity_at(y: f32, top: f32, bottom: f32, fade: f32) -> f32 {
    if fade <= 0.0 {
        return f32::from(u8::from((top..=bottom).contains(&y)));
    }
    ((y - top) / fade).min((bottom - y) / fade).clamp(0.0, 1.0)
}

impl Parts<'_, '_> {
    /// The roll of `list`, with the footer's hints.
    pub(super) fn credits(&mut self, list: &MenuList) {
        let frame = self.frame();
        self.panel(keys::SURFACE, frame.window(), SCRIM);
        self.title(&list.title);
        let s = frame.scale();
        let top = frame.list_top();
        let bottom = frame.footer_y() - FOOTER_GAP * s;
        let pitch = frame.sizing().units(PITCH);
        let x = frame.safe().x;
        for (i, line) in list.message.split('\n').enumerate() {
            let y = top + (i as f32 - list.scroll) * pitch;
            let opacity = opacity_at(y, top, bottom, FADE * s);
            let text = line.trim();
            if text.is_empty() || opacity <= 0.0 {
                continue;
            }
            let label = match is_heading(text) {
                true => Label::heading(text.to_owned(), HEADING_SIZE).colour(keys::ACCENT),
                false => Label::label(text.to_owned(), NAME_SIZE).colour(keys::TEXT),
            };
            self.text(label.opacity(opacity), (x, y), Align::Centre);
        }
        self.footer(&list.hints);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_of_capitals_is_a_heading_and_a_name_is_not() {
        assert!(is_heading("EXECUTIVE PRODUCER"));
        assert!(is_heading("MUSIC & SOUND 2"));
        assert!(!is_heading("Jane Q. Public"));
        assert!(!is_heading("   "));
        assert!(!is_heading("1234"), "no letters, no heading");
    }

    #[test]
    fn the_roll_fades_in_at_the_bottom_and_out_at_the_top() {
        let (top, bottom, fade) = (-100.0, 200.0, 40.0);
        assert_eq!(opacity_at(bottom + 5.0, top, bottom, fade), 0.0);
        assert_eq!(opacity_at(top - 5.0, top, bottom, fade), 0.0);
        assert_eq!(opacity_at(50.0, top, bottom, fade), 1.0);
        assert!((opacity_at(top + 20.0, top, bottom, fade) - 0.5).abs() < 1e-5);
        assert!((opacity_at(bottom - 10.0, top, bottom, fade) - 0.25).abs() < 1e-5);
        assert_eq!(opacity_at(0.0, top, bottom, 0.0), 1.0, "no fade: on the roll is fully shown");
    }
}
