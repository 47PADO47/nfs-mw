//! `Switch`: a control that cycles through a list of option labels, with optional left and right arrows. It emits one
//! `Text` node for the selected label, and one `Image` node per arrow (left first). Pad Left and Right step the
//! selection, wrapping at both ends. A press on an arrow steps it the same way.

use crate::component::Component;
use crate::components::{Image, Text, justify};
use crate::context::{EventCx, LayoutCx};
use crate::draw::DrawList;
use crate::geom::Rect;
use crate::input::{InputEvent, PadButton, PointerKind};

/// The two arrows of a switch: their images and the rectangles they were laid out in.
struct Arrows {
    left: Image,
    right: Image,
    left_rect: Rect,
    right_rect: Rect,
}

/// A switch over a list of option labels. It holds the selected index, and a `changed` flag that input sets. The
/// label is drawn centred in the rectangle (between the arrows, if any). Each arrow is a square as high as the
/// rectangle, or half its width when that is less.
pub struct Switch {
    options: Vec<String>,
    index: usize,
    changed: bool,
    label: Text,
    arrows: Option<Arrows>,
}

impl Switch {
    /// A switch over `options`, with the first option selected. Its labels use the skin font `font_key`. With no
    /// options it shows an empty label and ignores input.
    pub fn new<S: Into<String>>(font_key: impl Into<String>, options: impl IntoIterator<Item = S>) -> Self {
        let options: Vec<String> = options.into_iter().map(Into::into).collect();
        let first = options.first().cloned().unwrap_or_default();
        let label = Text::new(font_key, first).with_justification(justify::H_CENTRE | justify::V_CENTRE);
        Self { options, index: 0, changed: false, label, arrows: None }
    }

    /// A two-option switch for off and on, with the given labels. It starts off. `is_on` and `set_on` read and write
    /// it.
    pub fn on_off(font_key: impl Into<String>, off: impl Into<String>, on: impl Into<String>) -> Self {
        Self::new(font_key, [off.into(), on.into()])
    }

    /// Selects the option at `index` at construction, clamped to the last option.
    pub fn with_selected(mut self, index: usize) -> Self {
        self.set_selected(index);
        self
    }

    /// Adds the arrows: `left_key` and `right_key` are skin texture keys. A key the skin lacks draws texture 0.
    pub fn with_arrows(mut self, left_key: impl Into<String>, right_key: impl Into<String>) -> Self {
        self.arrows = Some(Arrows {
            left: Image::new(left_key),
            right: Image::new(right_key),
            left_rect: Rect::ZERO,
            right_rect: Rect::ZERO,
        });
        self
    }

    /// The index of the selected option.
    pub fn selected(&self) -> usize {
        self.index
    }

    /// The label of the selected option, or `None` when there are no options.
    pub fn selected_label(&self) -> Option<&str> {
        self.options.get(self.index).map(String::as_str)
    }

    /// Whether input changed the selection since the last call, and clears the flag. A host polls it after handling
    /// events. `set_selected` and `set_on` do not set it.
    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    /// Selects the option at `index`, clamped to the last option. Does nothing when there are no options. It does not
    /// set the `changed` flag.
    pub fn set_selected(&mut self, index: usize) {
        let Some(last) = self.options.len().checked_sub(1) else {
            return;
        };
        self.index = index.min(last);
        self.refresh_label();
    }

    /// Whether the second option (index 1) is selected. This reads as on for a switch made by `on_off`.
    pub fn is_on(&self) -> bool {
        self.index == 1
    }

    /// Selects the second option when `on`, and the first otherwise.
    pub fn set_on(&mut self, on: bool) {
        self.set_selected(usize::from(on));
    }

    /// Moves the selection by `delta` steps, wrapping at both ends. Returns whether the selection changed.
    fn step(&mut self, delta: isize) -> bool {
        let Some(count) = isize::try_from(self.options.len()).ok().filter(|count| *count > 0) else {
            return false;
        };
        let next = (self.index as isize + delta).rem_euclid(count) as usize;
        if next == self.index {
            return false;
        }
        self.index = next;
        self.changed = true;
        self.refresh_label();
        true
    }

    /// Shows the selected option in the label.
    fn refresh_label(&mut self) {
        let text = self.selected_label().unwrap_or_default().to_owned();
        self.label.set_text(text);
    }

    /// The step a press at `(x, y)` asks for: -1 on the left arrow, +1 on the right arrow, and none elsewhere.
    fn arrow_step_at(&self, x: f32, y: f32) -> Option<isize> {
        let arrows = self.arrows.as_ref()?;
        if arrows.left_rect.contains(x, y) {
            return Some(-1);
        }
        if arrows.right_rect.contains(x, y) {
            return Some(1);
        }
        None
    }
}

impl Component for Switch {
    fn layout(&mut self, cx: &LayoutCx<'_>, bounds: Rect) {
        let Some(arrows) = self.arrows.as_mut() else {
            self.label.layout(cx, bounds);
            return;
        };
        let arrow_w = bounds.h.min(bounds.w / 2.0).max(0.0);
        arrows.left_rect = Rect::new(bounds.min_x() + arrow_w / 2.0, bounds.y, arrow_w, bounds.h);
        arrows.right_rect = Rect::new(bounds.max_x() - arrow_w / 2.0, bounds.y, arrow_w, bounds.h);
        arrows.left.layout(cx, arrows.left_rect);
        arrows.right.layout(cx, arrows.right_rect);
        self.label.layout(cx, Rect::new(bounds.x, bounds.y, bounds.w - 2.0 * arrow_w, bounds.h));
    }

    fn build(&self, out: &mut DrawList) {
        if let Some(arrows) = &self.arrows {
            arrows.left.build(out);
            arrows.right.build(out);
        }
        self.label.build(out);
    }

    fn handle(&mut self, cx: &mut EventCx<'_>, ev: &InputEvent) -> bool {
        if self.options.is_empty() {
            return false;
        }
        let delta = match ev {
            InputEvent::Pad { button: PadButton::Left, pressed: true } => -1,
            InputEvent::Pad { button: PadButton::Right, pressed: true } => 1,
            InputEvent::Pointer { x, y, kind: PointerKind::Press } => match self.arrow_step_at(*x, *y) {
                Some(delta) => delta,
                None => return false,
            },
            _ => return false,
        };
        if self.step(delta) {
            cx.request_redraw();
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Switch;
    use crate::component::Component;
    use crate::components::justify;
    use crate::components::testing::{drawn, lay_out};
    use crate::context::EventCx;
    use crate::draw::DrawList;
    use crate::geom::Rect;
    use crate::input::{InputEvent, PadButton, PointerKind};
    use crate::skin::Skin;
    use crate::tree::{NodeKind, UiNode};
    use glam::Vec3;

    fn skin() -> Skin {
        Skin::new()
            .with_font("body", 7)
            .with_colour("text", [1, 2, 3, 255])
            .with_texture("arrow.left", 3)
            .with_texture("arrow.right", 4)
    }

    fn pad(button: PadButton) -> InputEvent {
        InputEvent::Pad { button, pressed: true }
    }

    fn texts(out: &DrawList) -> Vec<String> {
        out.iter().filter_map(|n| n.text.clone()).collect()
    }

    #[test]
    fn right_and_left_step_the_selection_and_wrap_at_both_ends() {
        let skin = skin();
        let mut s = Switch::new("body", ["low", "mid", "high"]);
        lay_out(&mut s, &skin, Rect::new(0.0, 0.0, 200.0, 40.0));
        let mut cx = EventCx::new(&skin);
        assert!(s.handle(&mut cx, &pad(PadButton::Right)));
        assert_eq!(s.selected(), 1);
        assert!(s.handle(&mut cx, &pad(PadButton::Right)));
        assert!(s.handle(&mut cx, &pad(PadButton::Right)));
        assert_eq!(s.selected(), 0, "Right past the last option wraps to the first");
        assert!(s.handle(&mut cx, &pad(PadButton::Left)));
        assert_eq!(s.selected(), 2, "Left past the first option wraps to the last");
        assert_eq!(s.selected_label(), Some("high"));
    }

    #[test]
    fn the_label_shows_the_selected_option() {
        let skin = skin();
        let mut s = Switch::new("body", ["low", "high"]);
        lay_out(&mut s, &skin, Rect::new(0.0, 0.0, 200.0, 40.0));
        assert_eq!(texts(&drawn(&s)), vec!["low".to_owned()]);
        let mut cx = EventCx::new(&skin);
        s.handle(&mut cx, &pad(PadButton::Right));
        assert_eq!(texts(&drawn(&s)), vec!["high".to_owned()]);
    }

    #[test]
    fn a_released_button_and_unrelated_buttons_do_nothing() {
        let skin = skin();
        let mut s = Switch::new("body", ["a", "b"]);
        lay_out(&mut s, &skin, Rect::SCREEN);
        let mut cx = EventCx::new(&skin);
        let release = InputEvent::Pad { button: PadButton::Right, pressed: false };
        assert!(!s.handle(&mut cx, &release));
        assert!(!s.handle(&mut cx, &pad(PadButton::Confirm)));
        assert_eq!(s.selected(), 0);
        assert!(!s.take_changed());
        assert!(!cx.redraw_requested());
    }

    #[test]
    fn changed_is_set_by_input_and_cleared_when_read() {
        let skin = skin();
        let mut s = Switch::new("body", ["a", "b", "c"]);
        let mut cx = EventCx::new(&skin);
        s.handle(&mut cx, &pad(PadButton::Right));
        assert!(s.take_changed());
        assert!(!s.take_changed(), "the flag clears once read");
        s.set_selected(2);
        assert!(!s.take_changed(), "a programmatic change does not set the flag");
    }

    #[test]
    fn a_changing_step_asks_for_a_redraw_and_a_one_option_switch_does_not_change() {
        let skin = skin();
        let mut two = Switch::new("body", ["a", "b"]);
        let mut cx = EventCx::new(&skin);
        assert!(two.handle(&mut cx, &pad(PadButton::Right)));
        assert!(cx.redraw_requested());

        let mut one = Switch::new("body", ["only"]);
        let mut cx = EventCx::new(&skin);
        assert!(one.handle(&mut cx, &pad(PadButton::Right)), "the switch still consumes its own input");
        assert!(!cx.redraw_requested());
        assert!(!one.take_changed());
    }

    #[test]
    fn on_off_flips_between_its_two_labels_and_reads_back() {
        let skin = skin();
        let mut s = Switch::on_off("body", "Off", "On");
        lay_out(&mut s, &skin, Rect::new(0.0, 0.0, 200.0, 40.0));
        assert!(!s.is_on());
        assert_eq!(texts(&drawn(&s)), vec!["Off".to_owned()]);
        let mut cx = EventCx::new(&skin);
        s.handle(&mut cx, &pad(PadButton::Right));
        assert!(s.is_on());
        assert_eq!(texts(&drawn(&s)), vec!["On".to_owned()]);
        s.set_on(false);
        assert!(!s.is_on());
        assert_eq!(s.selected_label(), Some("Off"));
    }

    #[test]
    fn with_selected_starts_at_the_given_option_and_clamps() {
        assert!(Switch::on_off("body", "Off", "On").with_selected(1).is_on());
        assert_eq!(Switch::new("body", ["a", "b", "c"]).with_selected(9).selected(), 2);
    }

    #[test]
    fn set_selected_clamps_and_an_empty_switch_ignores_input() {
        let skin = skin();
        let mut s = Switch::new("body", ["a", "b", "c"]);
        s.set_selected(9);
        assert_eq!(s.selected(), 2);

        let mut empty = Switch::new("body", Vec::<String>::new());
        lay_out(&mut empty, &skin, Rect::SCREEN);
        let mut cx = EventCx::new(&skin);
        assert!(!empty.handle(&mut cx, &pad(PadButton::Right)));
        assert_eq!(empty.selected_label(), None);
        assert_eq!(empty.selected(), 0);
    }

    #[test]
    fn arrows_are_pointer_targets_at_the_two_ends() {
        let skin = skin();
        let mut s = Switch::new("body", ["a", "b", "c"]).with_arrows("arrow.left", "arrow.right");
        lay_out(&mut s, &skin, Rect::new(0.0, 0.0, 200.0, 40.0));
        let mut cx = EventCx::new(&skin);
        let press = |x: f32| InputEvent::Pointer { x, y: 0.0, kind: PointerKind::Press };
        assert!(!s.handle(&mut cx, &press(0.0)), "the middle of the switch takes no press");
        assert!(s.handle(&mut cx, &press(-80.0)), "the left arrow sits at x = -80");
        assert_eq!(s.selected(), 2);
        assert!(s.handle(&mut cx, &press(80.0)), "the right arrow sits at x = 80");
        assert_eq!(s.selected(), 0);
        let release = InputEvent::Pointer { x: 80.0, y: 0.0, kind: PointerKind::Release };
        assert!(!s.handle(&mut cx, &release));
    }

    #[test]
    fn build_emits_the_arrows_then_the_label_between_them() {
        let skin = skin();
        let mut s = Switch::new("body", ["a", "b"]).with_arrows("arrow.left", "arrow.right");
        lay_out(&mut s, &skin, Rect::new(0.0, 0.0, 200.0, 40.0));
        let out = drawn(&s);
        let nodes: Vec<&UiNode> = out.iter().collect();
        assert_eq!(nodes.len(), 3);
        assert!(matches!(nodes[0].kind, NodeKind::Image { texture: 3, .. }));
        assert!(matches!(nodes[1].kind, NodeKind::Image { texture: 4, .. }));
        assert_eq!(nodes[0].world.w_axis.truncate(), Vec3::new(-80.0, 0.0, 1.0));
        assert_eq!(nodes[1].world.w_axis.truncate(), Vec3::new(80.0, 0.0, 1.0));
        assert_eq!(
            nodes[2].kind,
            NodeKind::Text {
                font: 7,
                justification: justify::H_CENTRE | justify::V_CENTRE,
                leading: 0,
                max_width: 120,
            }
        );
        assert_eq!(nodes[2].text.as_deref(), Some("a"));
        assert_eq!(nodes[2].world.w_axis.truncate(), Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn without_arrows_the_label_takes_the_whole_rectangle() {
        let skin = skin();
        let mut s = Switch::new("body", ["a"]);
        lay_out(&mut s, &skin, Rect::new(0.0, 0.0, 200.0, 40.0));
        let out = drawn(&s);
        assert_eq!(out.len(), 1);
        assert!(matches!(out.iter().next().map(|n| &n.kind), Some(NodeKind::Text { max_width: 200, .. })));
    }
}
