//! The Custom tachometer: the cluster's card and its rev gauge. The gauge is a thin row of segments, lit up to the rev
//! fraction. The lit segments take a colour by zone: cyan up to [`WARN_FRACTION`] of the scale, amber from there to the
//! red line, and red past it. The tachometer draws the card, because it is the first slot in the HUD order, so the other
//! parts of the cluster draw over it.
//!
//! State comes from the feed (`drive_feed.rs`). `layout` reads it and lays the parts out; `build` emits them.

use blackbox_ui::{Component, DrawList, LayoutCx, Panel, Rect, token_keys};

use super::drive_feed::{DriveFeed, DriveFeedCell};
use super::drive_layout::{self, SEGMENTS};
use super::rounded::Rounded;
use super::skin::drive;

/// The card's corner radius, in FEng units, when the theme declares none.
pub(super) const CARD_RADIUS: f32 = 6.0;

/// Where the lit segments turn amber, as a fraction of the scale (before the red line).
pub(super) const WARN_FRACTION: f32 = 0.85;

/// The Custom `Tachometer` component. Built per resolve from the shared feed; it holds no state of its own.
pub struct TachBar {
    feed: DriveFeedCell,
    body: Option<Body>,
}

impl TachBar {
    /// The tachometer that reads `feed` (the cell the Custom theme holds).
    pub fn new(feed: DriveFeedCell) -> Self {
        Self { feed, body: None }
    }
}

impl Component for TachBar {
    /// The gauge does not use `bounds`: it sits in the cluster's card (`drive_layout`).
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.body = Some(Body::of(&self.feed.snapshot(), cx));
    }

    fn build(&self, out: &mut DrawList) {
        let Some(body) = &self.body else { return };
        body.background.build(out);
        for segment in &body.segments {
            segment.build(out);
        }
    }
}

/// The laid-out parts: the card first, then the segments over it.
struct Body {
    background: Rounded,
    segments: Vec<Panel>,
}

impl Body {
    fn of(feed: &DriveFeed, cx: &LayoutCx<'_>) -> Self {
        // The card's radius is the theme's (the token), so a palette with tighter corners tightens the card too.
        let radius = cx.skin.metric(token_keys::RADIUS).unwrap_or(CARD_RADIUS);
        let background = Rounded::new(drive::BACKGROUND, drive_layout::card(feed.corner), radius, cx);
        let segments = (0..SEGMENTS).map(|index| segment(feed, index, cx)).collect();
        Self { background, segments }
    }
}

/// The segment `index`: lit when the revs pass its centre, and coloured by its zone. `mid` is the segment's centre as a
/// fraction of the scale.
fn segment(feed: &DriveFeed, index: usize, cx: &LayoutCx<'_>) -> Panel {
    let mid = (index as f32 + 0.5) / SEGMENTS as f32;
    let colour = if feed.rpm < mid {
        drive::TRACK
    } else if mid >= feed.red_line {
        drive::REV_REDLINE
    } else if mid >= WARN_FRACTION {
        drive::REV_WARN
    } else {
        drive::REV_LIT
    };
    let mut panel = Panel::new(colour);
    panel.layout(cx, drive_layout::segment(feed.corner, index));
    panel
}
