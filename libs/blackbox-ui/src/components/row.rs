//! `Row`: one labelled row. Its title sits in a left column, left-justified and vertically centred, and a control sits
//! in the right column. The control is any component (a `Switch`, or a `Text` for a read-only value). The row emits
//! what its title and control emit, and it sends pad events to the control. A pointer event goes to the control only
//! inside the right column.

use crate::component::{Component, Empty};
use crate::components::{Text, justify};
use crate::context::{EventCx, LayoutCx};
use crate::draw::DrawList;
use crate::geom::Rect;
use crate::input::InputEvent;

/// A title column and a control column. The title column is `with_title_width` (fixed) or the skin metric named by
/// `with_title_width_metric`, which wins when the skin has it. Without either, the title column is half the row.
pub struct Row {
    title: Text,
    control: Box<dyn Component>,
    title_width: Option<f32>,
    title_width_key: Option<String>,
    title_rect: Rect,
    control_rect: Rect,
}

impl Row {
    /// A row titled `title`, in the skin font `font_key`, with no control yet. The control column is empty until
    /// `with_control` sets one.
    pub fn new(font_key: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            title: Text::new(font_key, title).with_justification(justify::V_CENTRE),
            control: Box::new(Empty),
            title_width: None,
            title_width_key: None,
            title_rect: Rect::ZERO,
            control_rect: Rect::ZERO,
        }
    }

    /// Sets the right-hand control. Any component works.
    pub fn with_control(self, control: impl Component + 'static) -> Self {
        self.with_boxed_control(Box::new(control))
    }

    /// Sets the right-hand control from an already boxed component.
    pub fn with_boxed_control(mut self, control: Box<dyn Component>) -> Self {
        self.control = control;
        self
    }

    /// Fixes the title column width, in FEng units. The control column takes the rest of the row.
    pub fn with_title_width(mut self, width: f32) -> Self {
        self.title_width = Some(width);
        self
    }

    /// Reads the title column width from the skin metric `key` at layout. A key the skin lacks keeps the fixed width
    /// (or half the row).
    pub fn with_title_width_metric(mut self, key: impl Into<String>) -> Self {
        self.title_width_key = Some(key.into());
        self
    }

    /// The title's rectangle at the last layout.
    pub fn title_rect(&self) -> Rect {
        self.title_rect
    }

    /// The control's rectangle at the last layout.
    pub fn control_rect(&self) -> Rect {
        self.control_rect
    }
}

impl Component for Row {
    fn layout(&mut self, cx: &LayoutCx<'_>, bounds: Rect) {
        let width = self
            .title_width_key
            .as_deref()
            .and_then(|key| cx.skin.metric(key))
            .or(self.title_width)
            .unwrap_or(bounds.w / 2.0)
            .clamp(0.0, bounds.w.max(0.0));
        self.title_rect = Rect::new(bounds.min_x() + width / 2.0, bounds.y, width, bounds.h);
        self.control_rect =
            Rect::new(bounds.min_x() + width + (bounds.w - width) / 2.0, bounds.y, bounds.w - width, bounds.h);
        self.title.layout(cx, self.title_rect);
        self.control.layout(cx, self.control_rect);
    }

    fn build(&self, out: &mut DrawList) {
        self.title.build(out);
        self.control.build(out);
    }

    fn handle(&mut self, cx: &mut EventCx<'_>, ev: &InputEvent) -> bool {
        match ev {
            InputEvent::Pad { .. } => self.control.handle(cx, ev),
            InputEvent::Pointer { x, y, .. } => {
                if self.control_rect.contains(*x, *y) {
                    return self.control.handle(cx, ev);
                }
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Row;
    use crate::component::Component;
    use crate::components::testing::{drawn, lay_out};
    use crate::components::{Switch, Text, justify};
    use crate::context::EventCx;
    use crate::geom::Rect;
    use crate::input::{InputEvent, PadButton, PointerKind};
    use crate::skin::Skin;
    use glam::Vec3;

    fn skin() -> Skin {
        Skin::new().with_font("body", 7).with_colour("text", [1, 2, 3, 255])
    }

    fn pad(button: PadButton) -> InputEvent {
        InputEvent::Pad { button, pressed: true }
    }

    fn last_text(c: &dyn Component) -> Option<String> {
        drawn(c).iter().last().and_then(|n| n.text.clone())
    }

    #[test]
    fn title_is_in_the_left_column_and_control_in_the_right() {
        let skin = skin();
        let mut row = Row::new("body", "Radio").with_control(Switch::on_off("body", "Off", "On"));
        lay_out(&mut row, &skin, Rect::new(0.0, 0.0, 300.0, 40.0));
        assert_eq!(row.title_rect(), Rect::new(-75.0, 0.0, 150.0, 40.0));
        assert_eq!(row.control_rect(), Rect::new(75.0, 0.0, 150.0, 40.0));
        let out = drawn(&row);
        let nodes: Vec<_> = out.iter().collect();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].text.as_deref(), Some("Radio"));
        assert_eq!(nodes[0].world.w_axis.truncate(), Vec3::new(-150.0, 0.0, 1.0), "title starts at the left edge");
        assert_eq!(nodes[1].text.as_deref(), Some("Off"));
        assert_eq!(nodes[1].world.w_axis.truncate(), Vec3::new(75.0, 0.0, 1.0), "switch label centred in the right");
    }

    #[test]
    fn a_text_value_control_sits_at_the_right_edge_of_its_column() {
        let skin = skin();
        let value = Text::new("body", "120").with_justification(justify::H_RIGHT | justify::V_CENTRE);
        let mut row = Row::new("body", "Speed").with_control(value);
        lay_out(&mut row, &skin, Rect::new(0.0, 0.0, 300.0, 40.0));
        let out = drawn(&row);
        let value_node = out.iter().nth(1).expect("the value node");
        assert_eq!(value_node.text.as_deref(), Some("120"));
        assert_eq!(value_node.world.w_axis.truncate(), Vec3::new(150.0, 0.0, 1.0));
    }

    #[test]
    fn title_width_comes_from_the_skin_metric_and_falls_back_to_the_fixed_width() {
        let mut row = Row::new("body", "Speed").with_title_width(50.0).with_title_width_metric("column");
        lay_out(&mut row, &skin().with_metric("column", 100.0), Rect::new(0.0, 0.0, 300.0, 40.0));
        assert_eq!(row.title_rect(), Rect::new(-100.0, 0.0, 100.0, 40.0));
        assert_eq!(row.control_rect(), Rect::new(50.0, 0.0, 200.0, 40.0));

        lay_out(&mut row, &skin(), Rect::new(0.0, 0.0, 300.0, 40.0));
        assert_eq!(row.title_rect(), Rect::new(-125.0, 0.0, 50.0, 40.0));
    }

    #[test]
    fn pad_goes_to_the_control_and_a_press_reaches_it_only_in_the_control_column() {
        let skin = skin();
        let switch = Switch::on_off("body", "Off", "On").with_arrows("arrow.left", "arrow.right");
        let mut row = Row::new("body", "Radio").with_control(switch);
        lay_out(&mut row, &skin, Rect::new(0.0, 0.0, 300.0, 40.0));
        let mut cx = EventCx::new(&skin);

        assert!(row.handle(&mut cx, &pad(PadButton::Right)));
        assert_eq!(last_text(&row).as_deref(), Some("On"));

        let on_title = InputEvent::Pointer { x: -75.0, y: 0.0, kind: PointerKind::Press };
        assert!(!row.handle(&mut cx, &on_title), "the title column takes no press");
        assert_eq!(last_text(&row).as_deref(), Some("On"));

        // The left arrow of the switch sits at the left edge of the control column (x = 0), 20 units in.
        let on_arrow = InputEvent::Pointer { x: 20.0, y: 0.0, kind: PointerKind::Press };
        assert!(row.handle(&mut cx, &on_arrow));
        assert_eq!(last_text(&row).as_deref(), Some("Off"));
    }

    #[test]
    fn a_row_without_a_control_consumes_nothing() {
        let skin = skin();
        let mut row = Row::new("body", "Empty");
        lay_out(&mut row, &skin, Rect::SCREEN);
        let mut cx = EventCx::new(&skin);
        assert!(!row.handle(&mut cx, &pad(PadButton::Right)));
        assert_eq!(drawn(&row).len(), 1, "only the title draws");
    }
}
