//! The Remastered speedometer: the speed as a large number under the middle of the arc, the unit under it with the H of
//! MPH in gold, and the gear to the right.
//!
//! State comes from the feed (`custom::DriveFeedCell`). `layout` reads it and lays the texts out; `build` emits them.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect, Text, justify};

use super::layout::{at, size};
use super::skin::keys;
use crate::ui::theme::custom::DriveFeedCell;

/// Centres in reference picture points, and boxes in canvas units.
const VALUE_PX: (f32, f32) = (420.0, 180.0);
const VALUE_BOX: (f32, f32) = (120.0, 60.0);
const VALUE_SCALE: f32 = 4.5;
const UNIT_PX: (f32, f32) = (432.0, 272.0);
const UNIT_BOX: (f32, f32) = (120.0, 22.0);
const UNIT_SCALE: f32 = 1.8;
/// The H of MPH sits this many picture points right of the MP.
const UNIT_H_OFFSET_PX: f32 = 18.0;
const GEAR_PX: (f32, f32) = (595.0, 200.0);
const GEAR_BOX: (f32, f32) = (44.0, 60.0);
const GEAR_SCALE: f32 = 3.5;

/// The Remastered `Speedometer` component. Built per resolve from the shared feed; it holds no state of its own.
pub struct Readout {
    feed: DriveFeedCell,
    body: Option<Body>,
}

impl Readout {
    /// The readout that reads `feed` (the cell the Remastered theme holds).
    pub fn new(feed: DriveFeedCell) -> Self {
        Self { feed, body: None }
    }
}

impl Component for Readout {
    /// The readout does not use `bounds`: it sits in its own place on the canvas (see `layout`).
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let feed = self.feed.snapshot();
        let mut texts = vec![
            Placed::new(feed.speed.to_string(), keys::VALUE, keys::FONT_NUMBERS, VALUE_SCALE, VALUE_PX, VALUE_BOX),
            Placed::new(gear_label(feed.gear), keys::GEAR, keys::FONT_NUMBERS, GEAR_SCALE, GEAR_PX, GEAR_BOX),
        ];
        if feed.mph {
            let (x, y) = UNIT_PX;
            texts.push(Placed::new("MP".into(), keys::UNIT, keys::FONT_TEXT, UNIT_SCALE, (x - 2.0, y), UNIT_BOX));
            texts.push(Placed::new(
                "H".into(),
                keys::UNIT_HIGHLIGHT,
                keys::FONT_TEXT,
                UNIT_SCALE,
                (x + UNIT_H_OFFSET_PX - 2.0, y),
                UNIT_BOX,
            ));
        } else {
            texts.push(Placed::new("KM/H".into(), keys::UNIT, keys::FONT_TEXT, UNIT_SCALE, UNIT_PX, UNIT_BOX));
        }
        self.body = Some(Body { texts: texts.into_iter().map(|placed| placed.laid_out(cx)).collect() });
    }

    fn build(&self, out: &mut DrawList) {
        let Some(body) = &self.body else { return };
        for text in &body.texts {
            text.build(out);
        }
    }
}

/// The laid-out texts of the readout.
struct Body {
    texts: Vec<Text>,
}

/// A string with its place: centred on a picture point, in a box of `size` canvas units.
struct Placed {
    content: String,
    colour: &'static str,
    font: &'static str,
    scale: f32,
    at_px: (f32, f32),
    size: (f32, f32),
}

impl Placed {
    fn new(
        content: String,
        colour: &'static str,
        font: &'static str,
        scale: f32,
        at_px: (f32, f32),
        size: (f32, f32),
    ) -> Self {
        Self { content, colour, font, scale, at_px, size }
    }

    /// The text, laid out at its place.
    fn laid_out(self, cx: &LayoutCx<'_>) -> Text {
        let (x, y) = at(self.at_px.0, self.at_px.1);
        let mut text = Text::new(self.font, self.content)
            .with_colour_key(self.colour)
            .with_scale(self.scale)
            .with_justification(justify::H_CENTRE | justify::V_CENTRE);
        let (width, height) = size(self.size.0, self.size.1);
        text.layout(cx, Rect::new(x, y, width, height));
        text
    }
}

/// The gear as the digit the driver reads: `R` for reverse, `N` for neutral, else the number. The Custom speedometer
/// shows the same label.
fn gear_label(gear: i32) -> String {
    match gear {
        -1 => "R".into(),
        0 => "N".into(),
        n => n.to_string(),
    }
}
