//! A section title in the option package's own rows (the Original look), with no game asset of its own: the title string of
//! a row slot is centred across the whole row, in the colour a highlighted title has, with the slot's value, arrows and slider
//! hidden. The Remastered and Custom lists draw titles themselves (`ui/theme/*/menu`).
//!
//! The colour is [`ACCENT`], the colour of the selected row's title in the package (docs/specs/menu-sections.md, section 4).
//! The slots are reused as the list scrolls, so a slot that stops showing a title gets its authored colour back.

use blackbox_feng::ObjectRef;

use super::{Cx, WidgetMenu};
use crate::frontend::logic::anchor_offset;

/// The colour the selected row's title has in the option package (read from its string object, `colour_rgba`, on the installed
/// `Options.fng`; the other titles are 180, 180, 140). [verified]
pub(super) const ACCENT: [u8; 3] = [190, 128, 43];

/// Puts a string across a box (parent coordinates) the way its justification says, so that the text is centred in it: the
/// anchor of a right aligned string is its right edge, of a left aligned one its left edge, of a centred one its middle. The
/// text's box is centred vertically on `y`. The string must already hold its text, which sets its width.
pub(super) fn centre_string(cx: &mut Cx, o: ObjectRef, left: f32, width: f32, y: f32) {
    let Some(info) = cx.rt.string_info(o) else { return };
    let size = cx.text_size(o);
    let middle = left + width * 0.5;
    let anchor_x = match info.justification {
        j if j & 1 != 0 => middle,
        j if j & 2 != 0 => middle + size.x * 0.5,
        _ => middle - size.x * 0.5,
    };
    let down = anchor_offset(info.justification, size.y, 4, 8);
    cx.rt.set_position_xy(o, anchor_x, y - (down + size.y * 0.5));
}

impl WidgetMenu {
    /// Whether the row on screen slot `slot` is a section title.
    pub(super) fn heading_in(&self, slot: usize) -> bool {
        self.rows.get(self.top + slot).is_some_and(|row| row.setting.is_heading())
    }

    /// Dresses the title string of slot `slot`: the accent colour for a title, or, for a slot that showed a
    /// title a moment ago and does not any more, the colour the package gave its titles. The package's scripts own the
    /// colour of every other title.
    pub(super) fn style_title(&mut self, cx: &mut Cx, slot: usize) {
        let Some(r) = self.objects.get(slot) else { return };
        let name = r.name;
        let heading = self.heading_in(slot);
        if self.titled.len() <= slot {
            self.titled.resize(slot + 1, false);
        }
        let was = std::mem::replace(&mut self.titled[slot], heading);
        if heading {
            // Only the colour: the alpha stays the package's, which carries the screen's fade in and out.
            cx.rt.set_colour(name, ACCENT);
            return;
        }
        if let (true, Some(authored)) = (was, self.authored) {
            cx.rt.set_colour(name, authored);
        }
    }
}
