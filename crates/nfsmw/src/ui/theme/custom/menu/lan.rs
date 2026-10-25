//! The Custom LAN page (`LanMenu`): the page's rows as a Custom list (a host form's choices, a browser's rooms, a
//! lobby's players), and round it what the list has no place for: the status line beside the title, the reason
//! something failed on the description line in the danger colour, and the side column (the lobby's chat) on a plate at
//! the right when the window has room for it. The rows stand where the Custom list puts them, so the LAN front end's
//! click map tests them against `layout::MenuFrame` under this look (`frontend/lan/present.rs`).

use blackbox_ui::{Component, DrawList, LayoutCx, Rect};

use super::list::DESCRIPTION;
use super::{Align, GlideCell, Parts, screen};
use crate::ui::raster_text::{CUSTOM_LABEL_FONT_KEY, width_units};
use crate::ui::theme::custom::plate::PlateStyle;
use crate::ui::theme::custom::text::{Label, Sizing, size};
use crate::ui::theme::custom::tokens::keys;
use crate::ui::theme::{ElementId, LanCell, LanPage};

/// The side column: its gap from the rows' plates, the least width it needs, the room inside its plate, and its lines'
/// size and distance, at scale 1.
const SIDE_GAP: f32 = 28.0;
const SIDE_LEAST: f32 = 150.0;
const SIDE_PAD: f32 = 10.0;
const SIDE_SIZE: f32 = 12.0;
const SIDE_PITCH: f32 = 16.0;
/// The heading's height in the side column, at text size 1.
const SIDE_HEADING: f32 = 20.0;

/// The Custom component of the `LanMenu` slot.
pub struct LanMenu {
    cell: LanCell,
    glide: GlideCell,
    parts: Vec<Box<dyn Component>>,
}

impl LanMenu {
    /// The page in `cell`, its selection gliding on `glide`.
    pub fn new(cell: LanCell, glide: GlideCell) -> Self {
        Self { cell, glide, parts: Vec::new() }
    }
}

impl Component for LanMenu {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let slot = ElementId::LanMenu.slot();
        let Some(page) = self.cell.page() else {
            self.glide.forget(slot);
            self.parts = Vec::new();
            return;
        };
        let mut parts = screen(slot, &page.list, cx, &self.glide);
        parts.lan_extras(&page);
        self.parts = parts.finish();
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// `text`, cut short with an ellipsis when it is wider than `room` units in the label face at `size`. A text whose
/// width is not known (the font did not load) is kept whole.
pub(super) fn clipped(text: &str, size: f32, room: f32, sizing: Sizing) -> String {
    let scale = sizing.glyph_scale(size);
    let fits = |s: &str| width_units(CUSTOM_LABEL_FONT_KEY, s, scale).is_none_or(|width| width <= room);
    if fits(text) {
        return text.to_owned();
    }
    let mut cut: Vec<char> = text.chars().collect();
    while cut.pop().is_some() {
        let candidate: String = cut.iter().collect::<String>() + "...";
        if fits(&candidate) {
            return candidate;
        }
    }
    String::new()
}

impl Parts<'_, '_> {
    /// The status line, the failure line and the side column of `page`.
    fn lan_extras(&mut self, page: &LanPage) {
        self.subtitle(&page.subtitle);
        let frame = self.frame();
        if let Some(error) = &page.error {
            let line = Label::label(error.clone(), DESCRIPTION).colour(keys::DANGER);
            self.text(line, (frame.text_left(), frame.footer_y()), Align::Left);
        }
        self.side_column(page);
    }

    /// The lobby's chat on a plate at the right of the rows, newest last, as many lines as fit.
    fn side_column(&mut self, page: &LanPage) {
        if page.side_title.is_empty() {
            return;
        }
        let frame = self.frame();
        let s = frame.scale();
        let sizing = frame.sizing();
        let left = frame.row_right(page.list.has_values()) + SIDE_GAP * s;
        let right = frame.safe().max_x();
        if right - left < SIDE_LEAST * s {
            return;
        }
        let top = frame.list_top();
        let bottom = frame.row_y(super::layout::MAX_ROWS - 1) + frame.pitch() / 2.0;
        let plate = Rect::new((left + right) / 2.0, (top + bottom) / 2.0, right - left, bottom - top);
        self.plate(plate, PlateStyle::SURFACE.with_accent(keys::ACCENT));
        let pad = SIDE_PAD * s;
        let x = left + pad;
        let room = (right - left - 2.0 * pad).max(0.0);
        let heading = Label::heading(page.side_title.to_uppercase(), size::HEADING);
        let heading_y = top + pad + sizing.units(SIDE_HEADING) / 2.0;
        self.text(heading, (x, heading_y), Align::Left);
        let pitch = sizing.units(SIDE_PITCH);
        let first_y = heading_y + sizing.units(SIDE_HEADING) / 2.0 + pitch / 2.0;
        let fit = (((bottom - pad - first_y) / pitch).floor() + 1.0).max(0.0) as usize;
        let shown = page.side.len().min(fit);
        for (i, line) in page.side[page.side.len() - shown..].iter().enumerate() {
            let text = clipped(line, SIDE_SIZE, room, sizing);
            let label = Label::label(text, SIDE_SIZE).colour(keys::TEXT_SECONDARY);
            self.text(label, (x, first_y + pitch * i as f32), Align::Left);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_that_fits_is_kept_and_an_empty_room_leaves_nothing_or_the_whole_text() {
        let sizing = Sizing::default();
        assert_eq!(clipped("Ann: hi", SIDE_SIZE, 1000.0, sizing), "Ann: hi");
        let cut = clipped("Ann: a very long line of chat", SIDE_SIZE, 0.0, sizing);
        // With the font loaded nothing fits in no room; without it the width is unknown and the line is kept.
        assert!(cut.is_empty() || cut == "Ann: a very long line of chat", "{cut:?}");
    }
}
