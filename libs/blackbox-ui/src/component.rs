//! The base component trait, and the empty component the resolver falls back to.

use crate::context::{EventCx, LayoutCx};
use crate::draw::DrawList;
use crate::geom::Rect;
use crate::input::InputEvent;

/// One piece of UI. A host calls `layout` when the bounds or state change, `build` every frame it draws, and
/// `handle` for each input event until one is consumed.
pub trait Component {
    /// Measures and positions this component (and any children) inside `bounds`, in FEng units with the origin
    /// at the centre. Components cache what `build` needs here, because `build` takes no context.
    fn layout(&mut self, cx: &LayoutCx<'_>, bounds: Rect);

    /// Appends the draw nodes for the current state to `out`.
    fn build(&self, out: &mut DrawList);

    /// Handles an input event. Returns `true` when consumed. The default consumes nothing.
    fn handle(&mut self, _cx: &mut EventCx<'_>, _ev: &InputEvent) -> bool {
        false
    }
}

/// A component with no size and no output. The resolver returns it when no theme, not even Original, provides
/// a slot, so a missing slot draws nothing instead of panicking.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Empty;

impl Component for Empty {
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, _out: &mut DrawList) {}
}

#[cfg(test)]
mod tests {
    use super::{Component, Empty};
    use crate::context::{EventCx, LayoutCx};
    use crate::draw::DrawList;
    use crate::geom::Rect;
    use crate::input::{InputEvent, PadButton};
    use crate::skin::Skin;

    #[test]
    fn empty_draws_nothing_and_consumes_nothing() {
        let skin = Skin::new();
        let mut c = Empty;
        c.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
        let mut out = DrawList::new();
        c.build(&mut out);
        assert!(out.is_empty());
        let mut cx = EventCx::new(&skin);
        let ev = InputEvent::Pad { button: PadButton::Confirm, pressed: true };
        assert!(!c.handle(&mut cx, &ev));
        assert!(!cx.redraw_requested());
    }
}
