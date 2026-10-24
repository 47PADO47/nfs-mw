//! The Custom speedometer: the speed in big digits, and the unit beside it. It draws over the card that the tachometer
//! draws (`tachometer.rs`), in the boxes of `drive_layout`.
//!
//! State comes from the feed (`drive_feed.rs`). `layout` reads it and lays the two texts out; `build` emits them.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect, Text, justify};

use super::drive_feed::{DriveFeed, DriveFeedCell};
use super::drive_layout;
use super::skin::drive;
use crate::hud::{LABEL_KMH, LABEL_MPH, game_word};

/// The speed's glyph scale and the unit's (the unit is smaller).
const VALUE_SCALE: f32 = 2.8;
const UNIT_SCALE: f32 = 1.0;

/// The Custom `Speedometer` component. Built per resolve from the shared feed; it holds no state of its own.
pub struct SpeedCard {
    feed: DriveFeedCell,
    body: Option<Body>,
}

impl SpeedCard {
    /// The speedometer that reads `feed` (the cell the Custom theme holds).
    pub fn new(feed: DriveFeedCell) -> Self {
        Self { feed, body: None }
    }
}

impl Component for SpeedCard {
    /// The readout does not use `bounds`: it sits in the cluster's card (`drive_layout`).
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.body = Some(Body::of(&self.feed.snapshot(), cx));
    }

    fn build(&self, out: &mut DrawList) {
        let Some(body) = &self.body else { return };
        body.speed.build(out);
        body.unit.build(out);
    }
}

/// The laid-out texts: the speed, then the unit.
struct Body {
    speed: Text,
    unit: Text,
}

impl Body {
    fn of(feed: &DriveFeed, cx: &LayoutCx<'_>) -> Self {
        // Left-aligned and centred on the line, so the digits and the unit share a baseline.
        let mut speed = Text::new(drive::FONT, feed.speed.to_string())
            .with_colour_key(drive::VALUE)
            .with_scale(VALUE_SCALE)
            .with_justification(justify::V_CENTRE);
        speed.layout(cx, drive_layout::speed(feed.corner));
        let label = match feed.mph {
            true => game_word(LABEL_MPH, "MPH"),
            false => game_word(LABEL_KMH, "KM/H"),
        };
        let mut unit = Text::new(drive::FONT, label)
            .with_colour_key(drive::UNIT)
            .with_scale(UNIT_SCALE)
            .with_justification(justify::V_CENTRE);
        unit.layout(cx, drive_layout::unit(feed.corner));
        Self { speed, unit }
    }
}
