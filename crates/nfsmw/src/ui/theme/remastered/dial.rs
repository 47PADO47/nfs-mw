//! The Remastered tachometer: a solid arc from the lower left, over the top, to the upper right, with the rpm numbered on
//! it in thousands. The arc lights up as the revs rise, and the part past the red line turns gold.
//!
//! The arc is a row of small `Panel`s, one per step along the sweep, so it reads as a solid line.

use blackbox_ui::{Component, DrawList, LayoutCx, Panel, Rect, Text, justify};

use super::layout::{at, size};
use super::skin::keys;
use crate::ui::theme::custom::DriveFeedCell;

/// The arc's centre and radius, the angles (degrees, counter-clockwise from the right) of its start (rpm 0) and its end
/// (the scale's top), all in reference picture points.
const CENTRE_PX: (f32, f32) = (370.0, 380.0);
const RADIUS_PX: f32 = 340.0;
const START_DEG: f32 = 170.0;
const END_DEG: f32 = 25.0;
/// The gap between the arc's panels along the sweep, and the size of one panel.
const STEP_PX: f32 = 4.0;
const PANEL_PX: f32 = 7.0;
/// The numbers sit this far outside the arc, and their box.
const LABEL_OUT_PX: f32 = 22.0;
const LABEL_BOX_PX: (f32, f32) = (24.0, 18.0);
const LABEL_SCALE: f32 = 1.6;

/// The Remastered `Tachometer` component. Built per resolve from the shared feed; it holds no state of its own.
pub struct Dial {
    feed: DriveFeedCell,
    body: Option<Body>,
}

impl Dial {
    /// The dial that reads `feed` (the cell the Remastered theme holds).
    pub fn new(feed: DriveFeedCell) -> Self {
        Self { feed, body: None }
    }
}

impl Component for Dial {
    /// The dial does not use `bounds`: it sits in its own place on the canvas (see `layout`).
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let feed = self.feed.snapshot();
        self.body = Some(Body::of(feed.rpm, feed.red_line, feed.scale_rpm, cx));
    }

    fn build(&self, out: &mut DrawList) {
        let Some(body) = &self.body else { return };
        for panel in &body.arc {
            panel.build(out);
        }
        for tick in &body.ticks {
            tick.build(out);
        }
    }
}

/// The laid-out parts of the dial: the arc, then the numbers.
struct Body {
    arc: Vec<Panel>,
    ticks: Vec<Text>,
}

impl Body {
    fn of(rpm: f32, red_line: f32, scale_rpm: f32, cx: &LayoutCx<'_>) -> Self {
        let arc = (0..step_count())
            .map(|index| {
                let fraction = step_fraction(index);
                let (x, y) = point(fraction, RADIUS_PX);
                let (width, height) = size(PANEL_PX, PANEL_PX);
                let mut panel = Panel::new(arc_colour(fraction, rpm, red_line));
                panel.layout(cx, Rect::new(x, y, width, height));
                panel
            })
            .collect();
        let ticks = tick_fractions(scale_rpm)
            .map(|(number, fraction)| {
                let (x, y) = point(fraction, RADIUS_PX + LABEL_OUT_PX);
                let (width, height) = size(LABEL_BOX_PX.0, LABEL_BOX_PX.1);
                let colour = if fraction >= red_line { keys::TICK_REDLINE } else { keys::TICK };
                let mut tick = Text::new(keys::FONT_NUMBERS, number.to_string())
                    .with_colour_key(colour)
                    .with_scale(LABEL_SCALE)
                    .with_justification(justify::H_CENTRE | justify::V_CENTRE);
                tick.layout(cx, Rect::new(x, y, width, height));
                tick
            })
            .collect();
        Self { arc, ticks }
    }
}

/// The point at `fraction` (0 to 1 along the sweep) on a circle of `radius` picture points around the arc's centre, on
/// the canvas.
fn point(fraction: f32, radius: f32) -> (f32, f32) {
    let angle = (START_DEG + (END_DEG - START_DEG) * fraction).to_radians();
    at(CENTRE_PX.0 + radius * angle.cos(), CENTRE_PX.1 - radius * angle.sin())
}

/// How many panels make the arc: one per `STEP_PX` of its length.
fn step_count() -> usize {
    ((START_DEG - END_DEG).to_radians() * RADIUS_PX / STEP_PX).round() as usize
}

/// The place of panel `index` along the sweep, as a fraction from 0 to 1.
fn step_fraction(index: usize) -> f32 {
    (index as f32 + 0.5) / step_count() as f32
}

/// The skin key a panel draws in: lit when the revs have reached it, gold past the red line.
fn arc_colour(fraction: f32, rpm: f32, red_line: f32) -> &'static str {
    let lit = fraction <= rpm;
    let red = fraction >= red_line;
    match (lit, red) {
        (true, false) => keys::ARC_LIT,
        (true, true) => keys::ARC_REDLINE,
        (false, false) => keys::ARC_DIM,
        (false, true) => keys::ARC_REDLINE_DIM,
    }
}

/// The numbers and where each sits on the sweep: one per thousand rpm up to the scale, as `(number, fraction)`.
/// An empty feed (scale 0) has none.
fn tick_fractions(scale_rpm: f32) -> impl Iterator<Item = (u32, f32)> {
    let top = (scale_rpm / 1000.0).floor() as u32;
    (1..=top).map(move |number| (number, number as f32 * 1000.0 / scale_rpm))
}
