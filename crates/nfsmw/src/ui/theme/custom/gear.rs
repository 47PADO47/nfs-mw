//! The Custom gear indicator: the gear in a rounded box at the right of the cluster, and a strip of pips under the box with
//! the current gear lit (reverse, neutral, then first to sixth). The letter turns red while the shift light is on. It is
//! the Custom `GearIndicator` slot, so the Original gear digit is left out (the tachometer's Original scope excludes it).
//!
//! State comes from the feed (`drive_feed.rs`). `layout` reads it and lays the parts out; `build` emits them.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect, Rounded, Text, justify};

use super::drive_feed::{DriveFeed, DriveFeedCell};
use super::drive_layout::{self, PIPS};
use super::skin::drive;

/// The gear letter's glyph scale.
const GEAR_SCALE: f32 = 3.0;
/// The gear box's corner radius, and the pips' (the pips are pills).
const GEAR_RADIUS: f32 = 6.0;
const PIP_RADIUS: f32 = 1.5;

/// The Custom `GearIndicator` component. Built per resolve from the shared feed; it holds no state of its own.
pub struct GearCard {
    feed: DriveFeedCell,
    body: Option<Body>,
}

impl GearCard {
    /// The gear indicator that reads `feed` (the cell the Custom theme holds).
    pub fn new(feed: DriveFeedCell) -> Self {
        Self { feed, body: None }
    }
}

impl Component for GearCard {
    /// The indicator does not use `bounds`: it sits in the cluster's card (`drive_layout`).
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.body = Some(Body::of(&self.feed.snapshot(), cx));
    }

    fn build(&self, out: &mut DrawList) {
        let Some(body) = &self.body else { return };
        body.box_.build(out);
        body.letter.build(out);
        for pip in &body.pips {
            pip.build(out);
        }
    }
}

/// The laid-out parts: the gear's box, its letter, then the pips.
struct Body {
    box_: Rounded,
    letter: Text,
    pips: Vec<Rounded>,
}

impl Body {
    fn of(feed: &DriveFeed, cx: &LayoutCx<'_>) -> Self {
        let box_ = Rounded::new(drive::GEAR_BOX, drive_layout::gear(feed.corner), GEAR_RADIUS, cx);
        let letter_colour = if feed.shift_light { drive::GEAR_SHIFT } else { drive::GEAR_LETTER };
        let mut letter = Text::new(drive::FONT, gear_label(feed.gear))
            .with_colour_key(letter_colour)
            .with_scale(GEAR_SCALE)
            .with_justification(justify::H_CENTRE | justify::V_CENTRE);
        letter.layout(cx, drive_layout::gear(feed.corner));
        let lit = pip_index(feed.gear);
        let pips = (0..PIPS)
            .map(|index| {
                let colour = if Some(index) == lit { drive::PIP_ON } else { drive::PIP_OFF };
                Rounded::new(colour, drive_layout::pip(feed.corner, index), PIP_RADIUS, cx)
            })
            .collect();
        Self { box_, letter, pips }
    }
}

/// The pip that lights for `gear`: 0 for reverse, 1 for neutral, `gear + 1` for the forward gears. A gear past the last
/// pip lights the last one, and nothing lights for a gear below reverse.
fn pip_index(gear: i32) -> Option<usize> {
    let index = usize::try_from(gear + 1).ok()?;
    Some(index.min(PIPS - 1))
}

/// The gear as the digit the driver reads: `R` for reverse, `N` for neutral, else the number.
fn gear_label(gear: i32) -> String {
    match gear {
        -1 => "R".into(),
        0 => "N".into(),
        n => n.to_string(),
    }
}
