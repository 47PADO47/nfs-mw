//! `Stack`: lays child components out in a row or a column. It emits no nodes of its own, so each child builds its own.

use crate::component::Component;
use crate::context::{EventCx, LayoutCx};
use crate::draw::DrawList;
use crate::geom::Rect;
use crate::input::InputEvent;

/// The direction a `Stack` lays its children out in: `Row` runs left to right, `Column` runs top to bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackAxis {
    Row,
    Column,
}

/// A stack that splits its bounds equally along its axis, with a gap between neighbours. Each child gets the full
/// size across the axis. The gap is `with_gap`, unless `with_gap_metric` names a skin metric, which then wins.
///
/// Input goes to one child at a time. A pointer event goes to the child whose rectangle holds the point. A pad event
/// goes to the first child that consumes it.
pub struct Stack {
    axis: StackAxis,
    gap: f32,
    gap_key: Option<String>,
    children: Vec<Box<dyn Component>>,
    rects: Vec<Rect>,
}

impl Stack {
    /// An empty stack with no gap.
    pub fn new(axis: StackAxis) -> Self {
        Self { axis, gap: 0.0, gap_key: None, children: Vec::new(), rects: Vec::new() }
    }

    /// Sets the gap between neighbours, in FEng units.
    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Reads the gap from the skin metric `key` at layout. A key the skin lacks keeps the fixed gap.
    pub fn with_gap_metric(mut self, key: impl Into<String>) -> Self {
        self.gap_key = Some(key.into());
        self
    }

    /// Appends a child. Children are laid out and built in the order they were added.
    pub fn with_child(mut self, child: impl Component + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// The rectangle each child received at the last layout, in child order.
    pub fn child_rects(&self) -> &[Rect] {
        &self.rects
    }
}

/// The rectangle of child `index` out of `count`, splitting `bounds` along `axis` with `gap` between neighbours.
/// `count` must be at least 1.
fn slot(axis: StackAxis, index: usize, count: usize, gap: f32, bounds: Rect) -> Rect {
    let span = match axis {
        StackAxis::Row => bounds.w,
        StackAxis::Column => bounds.h,
    };
    let each = ((span - gap * (count - 1) as f32) / count as f32).max(0.0);
    let edge = match axis {
        StackAxis::Row => bounds.min_x(),
        StackAxis::Column => bounds.min_y(),
    };
    let start = edge + index as f32 * (each + gap);
    match axis {
        StackAxis::Row => Rect::new(start + each / 2.0, bounds.y, each, bounds.h),
        StackAxis::Column => Rect::new(bounds.x, start + each / 2.0, bounds.w, each),
    }
}

impl Component for Stack {
    fn layout(&mut self, cx: &LayoutCx<'_>, bounds: Rect) {
        self.rects.clear();
        let count = self.children.len();
        if count == 0 {
            return;
        }
        let gap = self.gap_key.as_deref().and_then(|key| cx.skin.metric(key)).unwrap_or(self.gap);
        for (index, child) in self.children.iter_mut().enumerate() {
            let rect = slot(self.axis, index, count, gap, bounds);
            child.layout(cx, rect);
            self.rects.push(rect);
        }
    }

    fn build(&self, out: &mut DrawList) {
        for child in &self.children {
            child.build(out);
        }
    }

    fn handle(&mut self, cx: &mut EventCx<'_>, ev: &InputEvent) -> bool {
        match ev {
            InputEvent::Pointer { x, y, .. } => {
                for (child, rect) in self.children.iter_mut().zip(&self.rects) {
                    if rect.contains(*x, *y) {
                        return child.handle(cx, ev);
                    }
                }
                false
            }
            InputEvent::Pad { .. } => {
                for child in &mut self.children {
                    if child.handle(cx, ev) {
                        return true;
                    }
                }
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Stack, StackAxis};
    use crate::component::Component;
    use crate::components::testing::{drawn, lay_out};
    use crate::components::{Image, Panel};
    use crate::context::EventCx;
    use crate::geom::Rect;
    use crate::input::{InputEvent, PadButton, PointerKind};
    use crate::skin::Skin;
    use std::cell::Cell;
    use std::rc::Rc;

    /// Counts the events it receives and consumes them when `consume` is set.
    struct Probe {
        calls: Rc<Cell<u32>>,
        consume: bool,
    }

    impl Component for Probe {
        fn layout(&mut self, _cx: &crate::context::LayoutCx<'_>, _bounds: Rect) {}

        fn build(&self, _out: &mut crate::draw::DrawList) {}

        fn handle(&mut self, _cx: &mut EventCx<'_>, _ev: &InputEvent) -> bool {
            self.calls.set(self.calls.get() + 1);
            self.consume
        }
    }

    #[test]
    fn row_puts_two_children_side_by_side_with_the_gap_between_them() {
        let skin = Skin::new();
        let mut s = Stack::new(StackAxis::Row).with_gap(10.0).with_child(Panel::new("a")).with_child(Panel::new("b"));
        lay_out(&mut s, &skin, Rect::new(0.0, 0.0, 200.0, 40.0));
        assert_eq!(s.child_rects(), &[Rect::new(-52.5, 0.0, 95.0, 40.0), Rect::new(52.5, 0.0, 95.0, 40.0)]);
        let out = drawn(&s);
        assert_eq!(out.len(), 2);
        let xs: Vec<f32> = out.iter().map(|n| n.world.w_axis.x).collect();
        assert_eq!(xs, vec![-52.5, 52.5]);
    }

    #[test]
    fn column_stacks_children_top_to_bottom() {
        let skin = Skin::new();
        let mut s = Stack::new(StackAxis::Column)
            .with_gap(4.0)
            .with_child(Image::new("a"))
            .with_child(Image::new("b"))
            .with_child(Image::new("c"));
        lay_out(&mut s, &skin, Rect::new(0.0, 0.0, 50.0, 116.0));
        let heights: Vec<f32> = s.child_rects().iter().map(|r| r.h).collect();
        assert_eq!(heights, vec![36.0, 36.0, 36.0]);
        let ys: Vec<f32> = s.child_rects().iter().map(|r| r.y).collect();
        assert_eq!(ys, vec![-40.0, 0.0, 40.0]);
        assert!(s.child_rects().iter().all(|r| r.w == 50.0));
    }

    #[test]
    fn gap_metric_from_the_skin_overrides_the_fixed_gap() {
        let skin = Skin::new().with_metric("gap", 20.0);
        let mut s = Stack::new(StackAxis::Row)
            .with_gap(0.0)
            .with_gap_metric("gap")
            .with_child(Panel::new("a"))
            .with_child(Panel::new("b"));
        lay_out(&mut s, &skin, Rect::new(0.0, 0.0, 100.0, 10.0));
        assert_eq!(s.child_rects()[0], Rect::new(-30.0, 0.0, 40.0, 10.0));
        assert_eq!(s.child_rects()[1], Rect::new(30.0, 0.0, 40.0, 10.0));
    }

    #[test]
    fn empty_stack_builds_nothing() {
        let skin = Skin::new();
        let mut s = Stack::new(StackAxis::Row);
        lay_out(&mut s, &skin, Rect::SCREEN);
        assert!(s.child_rects().is_empty());
        assert!(drawn(&s).is_empty());
    }

    #[test]
    fn pointer_goes_only_to_the_child_under_it() {
        let skin = Skin::new();
        let (left, right) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let mut s = Stack::new(StackAxis::Row)
            .with_child(Probe { calls: left.clone(), consume: true })
            .with_child(Probe { calls: right.clone(), consume: true });
        lay_out(&mut s, &skin, Rect::new(0.0, 0.0, 200.0, 40.0));
        let mut cx = EventCx::new(&skin);
        let ev = InputEvent::Pointer { x: 50.0, y: 0.0, kind: PointerKind::Press };
        assert!(s.handle(&mut cx, &ev));
        assert_eq!((left.get(), right.get()), (0, 1));
    }

    #[test]
    fn pad_goes_to_the_first_child_that_consumes_it() {
        let skin = Skin::new();
        let (first, second) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let mut s = Stack::new(StackAxis::Column)
            .with_child(Probe { calls: first.clone(), consume: false })
            .with_child(Probe { calls: second.clone(), consume: true });
        lay_out(&mut s, &skin, Rect::new(0.0, 0.0, 10.0, 10.0));
        let mut cx = EventCx::new(&skin);
        let ev = InputEvent::Pad { button: PadButton::Down, pressed: true };
        assert!(s.handle(&mut cx, &ev));
        assert_eq!((first.get(), second.get()), (1, 1));
    }
}
