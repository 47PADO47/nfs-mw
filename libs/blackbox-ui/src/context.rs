//! The context values passed into components. Each one borrows the active skin; none owns the tree.
//! Only generic inputs travel here (skin, time, redraw flag). No host or engine handle does, so the
//! crate stays free of any game or renderer dependency.

use crate::skin::Skin;

/// Passed to `Component::layout`: the skin a component reads its metrics from, and the host clock.
/// Per-frame state that changes over time (a fade, a needle easing) advances here, not in `build`.
#[derive(Clone, Copy, Debug)]
pub struct LayoutCx<'a> {
    pub skin: &'a Skin,
    /// Seconds since the host started. The host sets it; the component never reads a wall clock itself.
    pub time_s: f32,
}

impl<'a> LayoutCx<'a> {
    pub fn new(skin: &'a Skin, time_s: f32) -> Self {
        Self { skin, time_s }
    }
}

/// Passed to `Component::handle`: the skin, plus a way to ask the host for another redraw.
#[derive(Clone, Copy, Debug)]
pub struct EventCx<'a> {
    pub skin: &'a Skin,
    redraw: bool,
}

impl<'a> EventCx<'a> {
    pub fn new(skin: &'a Skin) -> Self {
        Self { skin, redraw: false }
    }

    /// Marks the tree as changed, so the host lays out and builds again before the next frame.
    pub fn request_redraw(&mut self) {
        self.redraw = true;
    }

    pub fn redraw_requested(&self) -> bool {
        self.redraw
    }
}

/// Passed to `Theme::element` when a slot is resolved: what the theme may read to decide whether
/// it has a component for this slot.
#[derive(Clone, Copy, Debug)]
pub struct BuildCx<'a> {
    pub skin: &'a Skin,
}

impl<'a> BuildCx<'a> {
    pub fn new(skin: &'a Skin) -> Self {
        Self { skin }
    }
}
