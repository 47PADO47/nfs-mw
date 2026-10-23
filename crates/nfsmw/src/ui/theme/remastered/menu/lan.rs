//! The Remastered LAN page: the vertical menu of the page's rows (a host form's choices, a browser's rooms, a lobby's
//! players), and around it what the list has no place for: the line under the title, the reason something failed at the
//! foot of the list, and the side column (the lobby's chat) at the right.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect};

use super::layout as place;
use super::{Parts, fitted};
use crate::ui::raster_text::{RASTER_MENU_FONT_KEY, width_units};
use crate::ui::theme::LanCell;
use crate::ui::theme::LanPage;
use crate::ui::theme::remastered::skin::keys;

/// Where the foot's failure line stands, and the side column's lines: the list's rows are `ROW_PITCH` apart, the side
/// column's lines closer.
const ERROR_Y: f32 = 176.0;
const SIDE_LEFT: f32 = 540.0;
const SIDE_LINE_Y: f32 = 17.0;
const SIDE_LINES: usize = 12;
const SIDE_SCALE: f32 = 1.35;
/// Narrower than this, the side column has no room and is left out.
const SIDE_MIN_WIDTH: f32 = 140.0;
const SUBTITLE_SCALE: f32 = 1.3;
const SUBTITLE_Y: f32 = place::TITLE_Y + 18.0;

/// The Remastered component of the `LanMenu` slot.
pub struct LanMenu {
    cell: LanCell,
    parts: Vec<Box<dyn Component>>,
}

impl LanMenu {
    pub fn new(cell: LanCell) -> Self {
        Self { cell, parts: Vec::new() }
    }
}

impl Component for LanMenu {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.parts = match self.cell.page() {
            Some(page) => page_parts(cx, &page),
            None => Vec::new(),
        };
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// The parts of `page`, back to front.
fn page_parts(cx: &LayoutCx<'_>, page: &LanPage) -> Vec<Box<dyn Component>> {
    let mut parts = Parts::new(cx).of(&page.list);
    let mut extras = Parts::new(cx);
    extras.lan_extras(page);
    parts.extend(extras.finish());
    parts
}

impl Parts<'_, '_> {
    /// The subtitle, the failure line and the side column of `page`.
    fn lan_extras(&mut self, page: &LanPage) {
        let left = place::left_edge(page.list.aspect);
        if !page.subtitle.is_empty() {
            let at = (left + place::TEXT_LEFT, SUBTITLE_Y);
            self.text(keys::FONT_MENU, &page.subtitle.to_uppercase(), keys::MENU_TAGLINE, SUBTITLE_SCALE, at, false);
        }
        if let Some(error) = &page.error {
            let scale = fitted(error, place::ENTRY_SCALE * 0.8, place::WIDE_BAR_RIGHT - place::TEXT_LEFT);
            self.text(keys::FONT_MENU, error, keys::PURSUIT_DAMAGED, scale, (left + place::TEXT_LEFT, ERROR_Y), false);
        }
        self.side_column(page, left, place::right_edge(page.list.aspect));
    }

    fn side_column(&mut self, page: &LanPage, left: f32, right: f32) {
        if page.side_title.is_empty() {
            return;
        }
        let x = left + SIDE_LEFT;
        let width = right - x - 12.0;
        if width < SIDE_MIN_WIDTH {
            return;
        }
        self.text(
            keys::FONT_MENU_WIDE,
            &page.side_title.to_uppercase(),
            keys::MENU_TAGLINE,
            1.5,
            (x, place::TITLE_Y),
            false,
        );
        let shown = page.side.len().min(SIDE_LINES);
        for (i, line) in page.side[page.side.len() - shown..].iter().enumerate() {
            let line = clipped(line, SIDE_SCALE, width);
            let y = place::row_y(0) - 8.0 + SIDE_LINE_Y * i as f32;
            self.text(keys::FONT_MENU, &line, keys::MENU_TEXT, SIDE_SCALE, (x, y), false);
        }
    }
}

/// `text`, cut short with an ellipsis when it is wider than `width` units at `scale`.
pub(in crate::ui::theme) fn clipped(text: &str, scale: f32, width: f32) -> String {
    let fits = |s: &str| width_units(RASTER_MENU_FONT_KEY, s, scale).is_none_or(|w| w <= width);
    if fits(text) {
        return text.to_owned();
    }
    let mut cut: Vec<char> = text.chars().collect();
    while !cut.is_empty() {
        cut.pop();
        let candidate: String = cut.iter().collect::<String>() + "...";
        if fits(&candidate) {
            return candidate;
        }
    }
    String::new()
}
