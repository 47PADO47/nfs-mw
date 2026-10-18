//! `Table`: a vertical list of `Row`s, such as an options or settings table. The optional header and each row take one
//! fixed-height slot, stacked down from the top of the table's rectangle. One row has the focus. Up and Down move the
//! focus (clamped at the ends, so an edge press returns false for a parent to use), Left and Right go to the focused
//! row's control, and a press on a row focuses that row and passes the press on. The focus can be drawn as a
//! highlight panel behind the focused row.

use crate::component::Component;
use crate::components::{Panel, Row, Text, justify};
use crate::context::{EventCx, LayoutCx};
use crate::draw::DrawList;
use crate::geom::Rect;
use crate::input::{InputEvent, PadButton, PointerKind};

/// A vertical list of rows, each one `row_height` tall (or the skin metric named by `with_row_height_metric`).
pub struct Table {
    row_height: f32,
    row_height_key: Option<String>,
    header: Option<Text>,
    header_rect: Rect,
    focus_highlight: Option<Panel>,
    rows: Vec<Row>,
    row_rects: Vec<Rect>,
    focus: usize,
}

/// The rectangle of slot `index` in a stack of `height`-tall slots starting at the top of `bounds`.
fn slot(bounds: Rect, height: f32, index: usize) -> Rect {
    Rect::new(bounds.x, bounds.min_y() + (index as f32 + 0.5) * height, bounds.w, height)
}

impl Table {
    /// An empty table whose rows are `row_height` tall, in FEng units.
    pub fn new(row_height: f32) -> Self {
        Self {
            row_height,
            row_height_key: None,
            header: None,
            header_rect: Rect::ZERO,
            focus_highlight: None,
            rows: Vec::new(),
            row_rects: Vec::new(),
            focus: 0,
        }
    }

    /// Reads the row height from the skin metric `key` at layout. A key the skin lacks keeps the fixed height.
    pub fn with_row_height_metric(mut self, key: impl Into<String>) -> Self {
        self.row_height_key = Some(key.into());
        self
    }

    /// Adds a header row above the rows, titled `title` in the skin font `font_key`. The header takes the first slot,
    /// so the rows start one slot lower.
    pub fn with_header(mut self, font_key: impl Into<String>, title: impl Into<String>) -> Self {
        self.header = Some(Text::new(font_key, title).with_justification(justify::V_CENTRE));
        self
    }

    /// Draws a highlight panel behind the focused row, filled with the skin colour `colour_key`.
    pub fn with_focus_colour(mut self, colour_key: impl Into<String>) -> Self {
        self.focus_highlight = Some(Panel::new(colour_key));
        self
    }

    /// Appends a row. Rows are laid out and built in the order they were added.
    pub fn with_row(mut self, row: Row) -> Self {
        self.rows.push(row);
        self
    }

    /// The number of rows, not counting the header.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// The index of the focused row. It is 0 when the table has no rows.
    pub fn focused(&self) -> usize {
        self.focus
    }

    /// Focuses the row at `index`, clamped to the last row. It takes effect at the next layout, which moves the
    /// highlight. Does nothing when there are no rows.
    pub fn set_focused(&mut self, index: usize) {
        let Some(last) = self.rows.len().checked_sub(1) else {
            return;
        };
        self.focus = index.min(last);
    }

    /// Each row's rectangle at the last layout, in row order (the header is not included).
    pub fn row_rects(&self) -> &[Rect] {
        &self.row_rects
    }

    /// The header's rectangle at the last layout, or `None` when the table has no header.
    pub fn header_rect(&self) -> Option<Rect> {
        self.header.as_ref().map(|_| self.header_rect)
    }

    /// Moves the focus by `delta` rows, clamped at both ends. Returns whether it moved.
    fn move_focus(&mut self, cx: &mut EventCx<'_>, delta: isize) -> bool {
        let Some(last) = self.rows.len().checked_sub(1) else {
            return false;
        };
        let next = (self.focus as isize + delta).clamp(0, last as isize) as usize;
        if next == self.focus {
            return false;
        }
        self.focus = next;
        cx.request_redraw();
        true
    }

    /// Focuses the row under `(x, y)` and passes the press on to it. Returns false when the point is on no row.
    fn press_at(&mut self, cx: &mut EventCx<'_>, ev: &InputEvent, x: f32, y: f32) -> bool {
        let Some(index) = self.row_rects.iter().position(|rect| rect.contains(x, y)) else {
            return false;
        };
        let Some(row) = self.rows.get_mut(index) else {
            return false;
        };
        if index != self.focus {
            self.focus = index;
            cx.request_redraw();
        }
        row.handle(cx, ev);
        true
    }
}

impl Component for Table {
    fn layout(&mut self, cx: &LayoutCx<'_>, bounds: Rect) {
        let height = self.row_height_key.as_deref().and_then(|key| cx.skin.metric(key)).unwrap_or(self.row_height);
        self.header_rect = slot(bounds, height, 0);
        if let Some(header) = self.header.as_mut() {
            header.layout(cx, self.header_rect);
        }
        let first = usize::from(self.header.is_some());
        self.row_rects.clear();
        for (index, row) in self.rows.iter_mut().enumerate() {
            let rect = slot(bounds, height, first + index);
            row.layout(cx, rect);
            self.row_rects.push(rect);
        }
        self.focus = self.focus.min(self.rows.len().saturating_sub(1));
        if let (Some(highlight), Some(rect)) = (self.focus_highlight.as_mut(), self.row_rects.get(self.focus)) {
            highlight.layout(cx, *rect);
        }
    }

    fn build(&self, out: &mut DrawList) {
        let highlight = self.focus_highlight.as_ref().filter(|_| !self.row_rects.is_empty());
        if let Some(highlight) = highlight {
            highlight.build(out);
        }
        if let Some(header) = &self.header {
            header.build(out);
        }
        for row in &self.rows {
            row.build(out);
        }
    }

    fn handle(&mut self, cx: &mut EventCx<'_>, ev: &InputEvent) -> bool {
        match ev {
            InputEvent::Pad { button: PadButton::Up, pressed: true } => self.move_focus(cx, -1),
            InputEvent::Pad { button: PadButton::Down, pressed: true } => self.move_focus(cx, 1),
            InputEvent::Pad { button: PadButton::Left | PadButton::Right, .. } => match self.rows.get_mut(self.focus) {
                Some(row) => row.handle(cx, ev),
                None => false,
            },
            InputEvent::Pointer { x, y, kind: PointerKind::Press } => self.press_at(cx, ev, *x, *y),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Table;
    use crate::component::Component;
    use crate::components::testing::{drawn, lay_out};
    use crate::components::{Row, Switch};
    use crate::context::EventCx;
    use crate::draw::DrawList;
    use crate::geom::Rect;
    use crate::input::{InputEvent, PadButton, PointerKind};
    use crate::skin::Skin;
    use glam::Vec3;

    fn skin() -> Skin {
        Skin::new().with_font("body", 7).with_colour("text", [1, 2, 3, 255]).with_colour("focus", [9, 9, 9, 255])
    }

    fn pad(button: PadButton) -> InputEvent {
        InputEvent::Pad { button, pressed: true }
    }

    fn last_text(out: &DrawList) -> Option<String> {
        out.iter().last().and_then(|n| n.text.clone())
    }

    fn three_plain_rows() -> Table {
        Table::new(30.0)
            .with_row(Row::new("body", "one"))
            .with_row(Row::new("body", "two"))
            .with_row(Row::new("body", "three"))
    }

    #[test]
    fn rows_stack_at_the_fixed_height_from_the_top() {
        let skin = skin();
        let mut t = three_plain_rows();
        lay_out(&mut t, &skin, Rect::new(0.0, 0.0, 300.0, 100.0));
        assert_eq!(
            t.row_rects(),
            &[Rect::new(0.0, -35.0, 300.0, 30.0), Rect::new(0.0, -5.0, 300.0, 30.0), Rect::new(0.0, 25.0, 300.0, 30.0)]
        );
        assert_eq!(t.row_count(), 3);
        assert_eq!(t.header_rect(), None);
    }

    #[test]
    fn a_header_takes_the_first_slot_and_the_rows_follow_it() {
        let skin = skin();
        let mut t = three_plain_rows().with_header("body", "Settings");
        lay_out(&mut t, &skin, Rect::new(0.0, 0.0, 300.0, 120.0));
        assert_eq!(t.header_rect(), Some(Rect::new(0.0, -45.0, 300.0, 30.0)));
        let ys: Vec<f32> = t.row_rects().iter().map(|r| r.y).collect();
        assert_eq!(ys, vec![-15.0, 15.0, 45.0]);
        assert!(t.row_rects().iter().all(|r| r.h == 30.0));
    }

    #[test]
    fn the_row_height_metric_overrides_the_fixed_height() {
        let skin = skin().with_metric("menu.row", 20.0);
        let mut t = Table::new(30.0)
            .with_row_height_metric("menu.row")
            .with_row(Row::new("body", "a"))
            .with_row(Row::new("body", "b"));
        lay_out(&mut t, &skin, Rect::new(0.0, 0.0, 300.0, 100.0));
        assert_eq!(t.row_rects(), &[Rect::new(0.0, -40.0, 300.0, 20.0), Rect::new(0.0, -20.0, 300.0, 20.0)]);
    }

    #[test]
    fn up_and_down_move_the_focus_and_stop_at_the_ends() {
        let skin = skin();
        let mut t = three_plain_rows();
        lay_out(&mut t, &skin, Rect::new(0.0, 0.0, 300.0, 100.0));
        let mut cx = EventCx::new(&skin);
        assert_eq!(t.focused(), 0);
        assert!(!t.handle(&mut cx, &pad(PadButton::Up)), "already at the top");
        assert!(t.handle(&mut cx, &pad(PadButton::Down)));
        assert!(cx.redraw_requested(), "a moved focus asks for a layout");
        assert!(t.handle(&mut cx, &pad(PadButton::Down)));
        assert_eq!(t.focused(), 2);
        assert!(!t.handle(&mut cx, &pad(PadButton::Down)), "already at the bottom");
        assert!(t.handle(&mut cx, &pad(PadButton::Up)));
        assert_eq!(t.focused(), 1);
    }

    #[test]
    fn left_and_right_reach_the_focused_rows_switch() {
        let skin = skin();
        let mut t = Table::new(30.0)
            .with_row(Row::new("body", "Sound"))
            .with_row(Row::new("body", "Radio").with_control(Switch::on_off("body", "Off", "On")));
        lay_out(&mut t, &skin, Rect::new(0.0, 0.0, 300.0, 100.0));
        let mut cx = EventCx::new(&skin);

        assert!(!t.handle(&mut cx, &pad(PadButton::Right)), "the focused row has no control");
        t.handle(&mut cx, &pad(PadButton::Down));
        assert!(t.handle(&mut cx, &pad(PadButton::Right)));
        let out = drawn(&t);
        assert_eq!(last_text(&out).as_deref(), Some("On"));
        assert!(t.handle(&mut cx, &pad(PadButton::Left)));
        assert_eq!(last_text(&drawn(&t)).as_deref(), Some("Off"));
    }

    #[test]
    fn a_press_focuses_the_row_under_it_and_reaches_its_control() {
        let skin = skin();
        let mut t = Table::new(30.0).with_row(Row::new("body", "Sound")).with_row(
            Row::new("body", "Radio")
                .with_control(Switch::on_off("body", "Off", "On").with_arrows("arrow.left", "arrow.right")),
        );
        lay_out(&mut t, &skin, Rect::new(0.0, 0.0, 300.0, 100.0));
        let mut cx = EventCx::new(&skin);

        // Row 1 is centred at y = -5. Its switch's left arrow is 30 units wide, centred at x = 15.
        let press = InputEvent::Pointer { x: 15.0, y: -5.0, kind: PointerKind::Press };
        assert!(t.handle(&mut cx, &press));
        assert_eq!(t.focused(), 1);
        assert_eq!(last_text(&drawn(&t)).as_deref(), Some("On"));

        let gap = InputEvent::Pointer { x: 0.0, y: -200.0, kind: PointerKind::Press };
        assert!(!t.handle(&mut cx, &gap), "a press below every row takes nothing");
    }

    #[test]
    fn the_focus_highlight_is_drawn_behind_the_focused_row() {
        let skin = skin();
        let mut t = three_plain_rows().with_focus_colour("focus");
        t.set_focused(1);
        lay_out(&mut t, &skin, Rect::new(0.0, 0.0, 300.0, 100.0));
        let out = drawn(&t);
        assert_eq!(out.len(), 4, "one highlight plus one title per row");
        let first = out.iter().next().expect("the highlight");
        assert_eq!(first.colour, [9, 9, 9, 255]);
        assert_eq!(first.world.w_axis.truncate(), Vec3::new(0.0, -5.0, 1.0));
    }

    #[test]
    fn an_empty_table_draws_nothing_and_ignores_input() {
        let skin = skin();
        let mut t = Table::new(30.0).with_focus_colour("focus").with_header("body", "Empty");
        lay_out(&mut t, &skin, Rect::SCREEN);
        assert_eq!(drawn(&t).len(), 1, "only the header draws");
        let mut cx = EventCx::new(&skin);
        assert!(!t.handle(&mut cx, &pad(PadButton::Down)));
        assert!(!t.handle(&mut cx, &pad(PadButton::Right)));
        t.set_focused(4);
        assert_eq!(t.focused(), 0);
    }
}
