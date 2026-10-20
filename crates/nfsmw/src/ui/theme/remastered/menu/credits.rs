//! The credits roll in the Remastered look: a dark scrim, the title at the top left, and the lines of the roll in a column in
//! the middle of the screen, scrolling up, fading in at the bottom and out at the top. A line that is all capitals is a
//! heading, drawn spaced and in the accent colour.

use blackbox_ui::Panel;

use super::Parts;
use crate::ui::theme::MenuList;
use crate::ui::theme::remastered::group::Group;
use crate::ui::theme::remastered::skin::keys;

/// Where the roll runs: the y of the line that has just entered at the bottom and of the one about to leave at the top, and the
/// distance between lines.
const BOTTOM: f32 = 190.0;
const TOP: f32 = -110.0;
const PITCH: f32 = 22.0;
/// How far from either end a line starts to fade.
const FADE: f32 = 46.0;
const NAME_SCALE: f32 = 1.65;
const HEADING_SCALE: f32 = 1.75;

/// Whether a line of the roll is a heading: it has letters, and none of them is lower case.
pub(super) fn is_heading(line: &str) -> bool {
    let letters: Vec<char> = line.chars().filter(|c| c.is_alphabetic()).collect();
    !letters.is_empty() && letters.iter().all(|c| c.is_uppercase())
}

/// The opacity of a line at height `y`: 1 in the middle of the roll, fading to 0 at its two ends.
pub(super) fn opacity_at(y: f32) -> f32 {
    ((y - TOP) / FADE).min((BOTTOM - y) / FADE).clamp(0.0, 1.0)
}

impl Parts<'_, '_> {
    /// The roll: `list.message` holds its lines, `list.scroll` how far past the first one it has gone.
    pub(super) fn credits(&mut self, list: &MenuList) {
        let scrim = blackbox_ui::Rect::new(0.0, 0.0, list.aspect * 480.0 + 4.0, 484.0);
        self.push(Panel::new(keys::MENU_SCRIM), scrim);
        self.shade(super::layout::left_edge(list.aspect));
        let left = super::layout::left_edge(list.aspect);
        let title = (left + super::layout::TEXT_LEFT, super::layout::TITLE_Y);
        self.text(
            keys::FONT_MENU_TITLE,
            &list.title.to_uppercase(),
            keys::MENU_TITLE,
            super::layout::TITLE_SCALE,
            title,
            false,
        );
        for (i, line) in list.message.split('\n').enumerate() {
            let y = TOP + (i as f32 - list.scroll) * PITCH;
            let opacity = opacity_at(y);
            if line.trim().is_empty() || opacity <= 0.0 {
                continue;
            }
            let mut inner = Parts::new(self.cx());
            match is_heading(line) {
                true => inner.centred(keys::FONT_MENU_WIDE, line.trim(), keys::MENU_TITLE, HEADING_SCALE, y),
                false => inner.centred(keys::FONT_MENU, line.trim(), keys::MENU_TEXT, NAME_SCALE, y),
            }
            self.add(Group::new(inner.finish(), opacity));
        }
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
        assert!(!is_heading("  "));
        assert!(!is_heading("1234"), "no letters, no heading");
    }

    #[test]
    fn the_roll_fades_in_at_the_bottom_and_out_at_the_top() {
        assert_eq!(opacity_at(BOTTOM + 5.0), 0.0);
        assert_eq!(opacity_at(TOP - 5.0), 0.0);
        assert_eq!(opacity_at((TOP + BOTTOM) / 2.0), 1.0);
        assert!(opacity_at(BOTTOM - 10.0) < 0.5 && opacity_at(TOP + 10.0) < 0.5);
    }
}
