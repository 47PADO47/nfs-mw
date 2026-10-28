//! The Custom `Notifications` slot: the notification stack and the feedback lane's chip, drawn from the frame's
//! [`NotificationView`](super::feed::NotificationView) in the Custom feed (`docs/specs/custom-hud/notifications.md`
//! section 6, `feedback.md` section 1). The queue that decides what shows is the HUD's (`hud/notify/`); this module
//! only draws it.
//!
//! - `stack`: up to three plates at the left-middle, the most urgent on top.
//! - `lane`: the chip just above the cluster.
//! - `icon`: the pictograms beside every notice and chip.
//!
//! `layout` does the frame's work, as the other Custom components do: it reads the feed and lays every part out at its
//! fade, and `build` emits what `layout` made. With no notice and no chip it emits nothing.

pub(super) mod icon;
mod lane;
mod stack;
#[cfg(test)]
mod tests;

use blackbox_ui::{Component, DrawList, LayoutCx, Rect};

use super::feed::{CustomFeed, CustomFeedCell};
use super::layout::HudBox;
use super::text::Sizing;
use super::tokens::Motion;

/// The laid-out parts of the slot, in draw order (their depths decide what is on top).
#[derive(Default)]
pub(super) struct Parts(Vec<Box<dyn Component>>);

impl Parts {
    fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, part: impl Component + 'static) {
        self.0.push(Box::new(part));
    }

    fn extend(&mut self, other: Parts) {
        self.0.extend(other.0);
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.0 {
            part.build(out);
        }
    }
}

/// An ease-out of `t` (0 to 1): fast at first, settling at the end.
fn ease(t: f32) -> f32 {
    let rest = 1.0 - t.clamp(0.0, 1.0);
    1.0 - rest * rest
}

/// The Custom `Notifications` component. Built per resolve from the shared feed; it holds no state of its own.
pub struct Notifications {
    feed: CustomFeedCell,
    parts: Parts,
}

impl Notifications {
    /// The stack and the lane that read `feed` (the cell the Custom theme holds).
    pub fn new(feed: CustomFeedCell) -> Self {
        Self { feed, parts: Parts::new() }
    }
}

impl Component for Notifications {
    /// The slot's place comes from the HUD layout in the feed, not from `bounds`.
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.parts = parts(&self.feed.get(), cx);
    }

    fn build(&self, out: &mut DrawList) {
        self.parts.build(out);
    }
}

/// Every part of the stack and the lane for `feed`, laid out against `cx`.
fn parts(feed: &CustomFeed, cx: &LayoutCx<'_>) -> Parts {
    let Some(view) = &feed.notifications else { return Parts::new() };
    let frame = HudBox::new(feed.screen, feed.settings.scale);
    let sizing = Sizing::of(&frame, &feed.settings);
    let motion = Motion::for_settings(&feed.settings);
    let opacity = feed.settings.opacity;
    let mut parts = stack::parts(&view.notices, &frame, sizing, motion, opacity, cx);
    parts.extend(lane::parts(view.chip.as_ref(), &frame, sizing, motion, opacity, cx));
    parts
}
