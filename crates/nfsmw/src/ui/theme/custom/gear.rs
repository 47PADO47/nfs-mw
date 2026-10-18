//! The Custom gear indicator: the gear number at the right of the cluster, next to the speed readout. The number turns
//! red while the shift light is on. No box or pips — the reference design shows just the digit.
//!
//! State comes from the feed (`drive_feed.rs`). `layout` reads it and lays the text out; `build` emits it.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect, Text, justify};

use super::drive_feed::{DriveFeed, DriveFeedCell};
use super::drive_layout;
use super::skin::drive;

/// The gear digit's glyph scale.
const GEAR_SCALE: f32 = 2.4;

/// The Custom `GearIndicator` component.
pub struct GearCard {
    feed: DriveFeedCell,
    body: Option<Body>,
}

impl GearCard {
    pub fn new(feed: DriveFeedCell) -> Self {
        Self { feed, body: None }
    }
}

impl Component for GearCard {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.body = Some(Body::of(&self.feed.snapshot(), cx));
    }

    fn build(&self, out: &mut DrawList) {
        let Some(body) = &self.body else { return };
        body.letter.build(out);
    }
}

struct Body {
    letter: Text,
}

impl Body {
    fn of(feed: &DriveFeed, cx: &LayoutCx<'_>) -> Self {
        let colour = if feed.shift_light { drive::GEAR_SHIFT } else { drive::GEAR_LETTER };
        let mut letter = Text::new(drive::FONT, gear_label(feed.gear))
            .with_colour_key(colour)
            .with_scale(GEAR_SCALE)
            .with_justification(justify::H_CENTRE | justify::V_CENTRE);
        letter.layout(cx, drive_layout::gear(feed.corner));
        Self { letter }
    }
}

/// The gear as the digit the driver reads: `R` for reverse, `N` for neutral, else the number.
fn gear_label(gear: i32) -> String {
    match gear {
        -1 => "R".into(),
        0 => "N".into(),
        n => n.to_string(),
    }
}
