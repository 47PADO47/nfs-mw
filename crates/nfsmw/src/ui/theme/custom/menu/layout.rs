//! Where the Custom menus put things (`docs/specs/custom-hud/menus.md` section 2), in canvas units: 480 high, origin at
//! the centre, y down. The list sits left-aligned in the content box (`custom/layout.rs`: the window, or its central
//! 16:9 part on a wider one), starting at 30% of the box's height, with the title above it, the description line at
//! the bottom left and the footer's hints at the bottom right.
//!
//! The menus have their own scale (1, times the layout's narrow-window factor) and take `[custom_hud] text_size`, never
//! the HUD scale. The row pitch grows with the text size until eight rows fill the room above the footer.
//!
//! The geometry is shared with the frontend's hit tests (`frontend/screens/geometry.rs`), so a click lands on what is
//! drawn.

use blackbox_ui::Rect;
use glam::Vec2;

use crate::ui::present::{FENG_HEIGHT, Screen};
use crate::ui::theme::custom::layout::HudBox;
use crate::ui::theme::custom::text::Sizing;

/// How many entries show at once; a longer list scrolls. The frontend windows its lists by the same count
/// (`ui/theme/remastered/menu/layout.rs` `window_start`), so the two looks show the same entries.
pub const MAX_ROWS: usize = 8;
/// The menus' own scale, before the narrow-window factor.
pub const MENU_SCALE: f32 = 1.0;

/// The selected row's plate starts this far in from the safe area's left edge, and the text this far in from the plate.
const PLATE_LEFT: f32 = 8.0;
const TEXT_INSET: f32 = 16.0;
/// The accent bar on the selected row's left.
pub const BAR_WIDTH: f32 = 3.0;
/// The list's top, as a share of the content box's height from its top.
const LIST_TOP: f32 = 0.30;
/// The room kept under the list for the description line and the hints.
const FOOTER_ROOM: f32 = 40.0;
/// A row's pitch at text size 1, and the gap between two rows' plates.
const ROW_PITCH: f32 = 28.0;
const ROW_GAP: f32 = 4.0;
/// A row's plate: an entry's, and a settings row's, which holds the value column.
const ROW_WIDTH: f32 = 300.0;
const WIDE_ROW_WIDTH: f32 = 500.0;
/// The value column: its width and its right edge's inset from the row's right.
const VALUE_WIDTH: f32 = 200.0;
const VALUE_INSET: f32 = 12.0;
/// How wide the chevron zone at each end of the value column is, for a click.
pub const CHEVRON_ZONE: f32 = 24.0;
/// The space kept between a label and the value column.
const LABEL_GAP: f32 = 10.0;
/// The title's centre above the list's top, at text size 1.
const TITLE_GAP: f32 = 30.0;
/// The footer line's centre above the safe area's bottom.
const FOOTER_LINE: f32 = 12.0;

/// Where a click on a settings row lands, by its place along the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// On the left chevron.
    Left,
    /// Anywhere else on the row.
    Middle,
    /// On the right chevron.
    Right,
}

/// How many of `count` entries show: all of them up to [`MAX_ROWS`].
pub fn shown(count: usize) -> usize {
    count.min(MAX_ROWS)
}

/// The menus' frame for one window shape and one text size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuFrame {
    window: Rect,
    content: Rect,
    safe: Rect,
    scale: f32,
    text_size: f32,
}

impl MenuFrame {
    /// The frame of a window `aspect` wide (width over height) at `text_size`. A text size that is not a positive
    /// number counts as 1.
    pub fn new(aspect: f32, text_size: f32) -> Self {
        let aspect = match aspect.is_finite() && aspect > 0.0 {
            true => aspect,
            false => 4.0 / 3.0,
        };
        let screen = Screen { width: FENG_HEIGHT * aspect, height: FENG_HEIGHT, pixels_per_point: 1.0 };
        let frame = HudBox::new(screen, MENU_SCALE);
        let text_size = match text_size.is_finite() && text_size > 0.0 {
            true => text_size,
            false => 1.0,
        };
        Self {
            window: Rect::new(0.0, 0.0, FENG_HEIGHT * aspect, FENG_HEIGHT),
            content: frame.content(),
            safe: frame.safe(),
            scale: frame.scale(),
            text_size,
        }
    }

    /// The whole window.
    pub fn window(&self) -> Rect {
        self.window
    }

    /// The content box less the safe inset.
    pub fn safe(&self) -> Rect {
        self.safe
    }

    /// The effective scale: the menus' own, times the narrow-window factor.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// The text sizing: the menus' scale and `[custom_hud] text_size`.
    pub fn sizing(&self) -> Sizing {
        Sizing::new(self.scale, self.text_size)
    }

    /// The left edge of a row's plate.
    pub fn plate_left(&self) -> f32 {
        self.safe.min_x() + PLATE_LEFT * self.scale
    }

    /// Where the title, the labels and the description line start.
    pub fn text_left(&self) -> f32 {
        self.plate_left() + TEXT_INSET * self.scale
    }

    /// The top of the first row.
    pub fn list_top(&self) -> f32 {
        self.content.min_y() + LIST_TOP * self.content.h
    }

    /// The lowest the last row may reach.
    fn list_bottom(&self) -> f32 {
        self.safe.max_y() - FOOTER_ROOM * self.scale
    }

    /// The distance between two rows' centres.
    pub fn pitch(&self) -> f32 {
        let room = (self.list_bottom() - self.list_top()) / MAX_ROWS as f32;
        (ROW_PITCH * self.scale * self.text_size).min(room)
    }

    /// The y of the middle of the `i`th shown row.
    pub fn row_y(&self, i: usize) -> f32 {
        self.list_top() + self.pitch() * (i as f32 + 0.5)
    }

    /// How wide a row's plate is: a settings row (`wide`) holds the value column. It never runs past the safe area.
    pub fn row_width(&self, wide: bool) -> f32 {
        let width = match wide {
            true => WIDE_ROW_WIDTH,
            false => ROW_WIDTH,
        };
        (width * self.scale).min(self.safe.max_x() - self.plate_left())
    }

    /// The right edge of a row's plate.
    pub fn row_right(&self, wide: bool) -> f32 {
        self.plate_left() + self.row_width(wide)
    }

    /// The plate of the row whose middle is at `y`.
    pub fn row_rect(&self, y: f32, wide: bool) -> Rect {
        let width = self.row_width(wide);
        let height = self.pitch() - ROW_GAP * self.scale;
        Rect::new(self.plate_left() + width / 2.0, y, width, height)
    }

    /// The column of a settings row's value, at `y`.
    pub fn value_column(&self, y: f32) -> Rect {
        let right = self.row_right(true) - VALUE_INSET * self.scale;
        let width = (VALUE_WIDTH * self.scale).min(right - self.text_left());
        Rect::new(right - width / 2.0, y, width, self.pitch() - ROW_GAP * self.scale)
    }

    /// How far a label may run from [`MenuFrame::text_left`] before it meets the value column (`wide`) or the plate's
    /// end.
    pub fn label_room(&self, wide: bool) -> f32 {
        let end = match wide {
            true => self.value_column(0.0).min_x(),
            false => self.row_right(false),
        };
        (end - LABEL_GAP * self.scale - self.text_left()).max(0.0)
    }

    /// The y of the title's middle.
    pub fn title_y(&self) -> f32 {
        self.list_top() - TITLE_GAP * self.scale * self.text_size
    }

    /// The y of the footer line: the description at the left, the hints at the right.
    pub fn footer_y(&self) -> f32 {
        self.safe.max_y() - FOOTER_LINE * self.scale
    }

    /// The row (an index into the shown ones) under a point, for a list that shows `shown` rows. The whole band of a
    /// row answers, from the window's left edge to the end of its plate.
    pub fn row_at(&self, point: Vec2, shown: usize, wide: bool) -> Option<usize> {
        if point.x > self.row_right(wide) {
            return None;
        }
        let half = self.pitch() / 2.0;
        (0..shown).find(|&i| (point.y - self.row_y(i)).abs() <= half)
    }

    /// The side of a settings row the point at `x` is on.
    pub fn side_at(&self, x: f32) -> Side {
        let column = self.value_column(0.0);
        let zone = CHEVRON_ZONE * self.scale;
        if (column.min_x()..column.min_x() + zone).contains(&x) {
            return Side::Left;
        }
        if (column.max_x() - zone..=column.max_x()).contains(&x) {
            return Side::Right;
        }
        Side::Middle
    }

    /// How far along the value column the point at `x` is, 0 at its left and 1 at its right, clamped.
    pub fn along_value(&self, x: f32) -> f32 {
        let column = self.value_column(0.0);
        ((x - column.min_x()) / column.w).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
