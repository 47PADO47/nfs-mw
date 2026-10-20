//! The Remastered tachometer: a quarter-ellipse arc from the bottom left of the card up and over its top, with a short run
//! along the top, the rev numbers set along it one thousand rpm apart with a tick for each, and a dark card inside the arc
//! with a lighter panel at its right for the gear. The arc lights up as the revs rise and turns to the accent colour from
//! the red line. A thin light line under the card stands on the gauges.

use blackbox_ui::{Component, DrawList, LayoutCx, Panel, Rect, Slant, Text, justify};

use super::arc::{self, Path, Point};
use super::layout::{rect, size};
use super::skin::keys;
use crate::ui::theme::custom::DriveFeedCell;

/// Depths: a lower `z` draws on top. The card sits under the bars, which are at the default depth, the gear panel over the
/// card, and the dial over them all.
const CARD_Z: f32 = 2.0;
const PANEL_Z: f32 = 1.9;
const DIAL_Z: f32 = 0.5;
/// The card's rows: left of its lower part, right edge, and bottom, in picture pixels, and the height of one strip.
const CARD_LEFT: f32 = 65.0;
const CARD_RIGHT: f32 = 580.0;
const CARD_BOTTOM: f32 = 298.0;
const STRIP_PX: f32 = 3.0;
/// The gear panel at the card's right.
const GEAR_PANEL: (f32, f32, f32, f32) = (470.0, 100.0, 576.0, 280.0);
/// The light line under the card: its span and height.
const BASE_LINE: (f32, f32, f32, f32) = (245.0, 303.0, 392.0, 305.5);
/// The arc's stroke and the steps it is drawn in, in picture pixels.
const STROKE_PX: f32 = 3.4;
const STEP_PX: f32 = 1.3;
/// A tick starts this far outside the arc and is this long, in dots of `TICK_PX`.
const TICK_FROM_PX: f32 = 3.0;
const TICK_DOTS: usize = 4;
const TICK_PX: f32 = 2.4;
/// The tick numbers sit this far inside the arc.
const LABEL_IN_PX: f32 = 30.0;
const LABEL_BOX_PX: (f32, f32) = (30.0, 24.0);

pub struct Dial {
    feed: DriveFeedCell,
    body: Option<Body>,
}

impl Dial {
    pub fn new(feed: DriveFeedCell) -> Self {
        Self { feed, body: None }
    }
}

impl Component for Dial {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let feed = self.feed.snapshot();
        self.body = Some(Body::of(feed.rpm, feed.red_line, feed.scale_rpm, cx));
    }

    fn build(&self, out: &mut DrawList) {
        let Some(body) = &self.body else { return };
        for part in &body.parts {
            part.build(out);
        }
    }
}

struct Body {
    parts: Vec<Box<dyn Component>>,
}

impl Body {
    fn of(rpm: f32, red_line: f32, scale_rpm: f32, cx: &LayoutCx<'_>) -> Self {
        let tick_scale = cx.skin.metric(keys::SCALE_TICK).unwrap_or(1.2);
        let path = Path::new();
        let top = (scale_rpm / 1000.0).floor().max(2.0) as u32;
        // The numbers run from 1 at the arc's start to the top number at the end of the run, so a rev reading maps to a
        // share of the path by its thousands.
        let share_of = |fraction: f32| (fraction * scale_rpm / 1000.0 - 1.0) / (top - 1) as f32;
        let (lit_to, red_from) = (share_of(rpm), share_of(red_line));

        let mut parts: Vec<Box<dyn Component>> = Vec::new();
        card(&mut parts, cx);
        let mut panel = Panel::new(keys::GEAR_PANEL).with_z(PANEL_Z);
        panel.layout(cx, rect(GEAR_PANEL.0, GEAR_PANEL.1, GEAR_PANEL.2, GEAR_PANEL.3));
        parts.push(Box::new(panel));
        let line = rect(BASE_LINE.0, BASE_LINE.1, BASE_LINE.2, BASE_LINE.3);
        parts.push(Box::new(Slant::new(
            keys::TICK,
            line,
            0.0,
            &[(0.0, 0.0), (0.2, 0.8), (0.8, 0.8), (1.0, 0.0)],
            DIAL_Z,
            cx,
        )));

        let steps = (path.length() / STEP_PX).round() as usize;
        for index in 0..steps {
            let point = path.at((index as f32 + 0.5) / steps as f32);
            let colour = arc_colour(point.share, lit_to, red_from);
            parts.push(dot(point.x, point.y, STROKE_PX, colour, cx));
        }
        for number in 1..=top {
            let share = (number - 1) as f32 / (top - 1) as f32;
            let point = path.at(share);
            let red = share >= red_from;
            tick(&mut parts, point, if red { keys::TICK_REDLINE } else { keys::TICK }, cx);
            parts.push(label(number, point, red, tick_scale, cx));
        }
        Self { parts }
    }
}

/// The dark card: strips as tall as `STRIP_PX`, each starting where the arc is at its height and running to the card's
/// right edge, so the card's top left is the arc.
fn card(parts: &mut Vec<Box<dyn Component>>, cx: &LayoutCx<'_>) {
    let mut top = arc::TOP;
    while top < CARD_BOTTOM {
        let bottom = (top + STRIP_PX).min(CARD_BOTTOM);
        let left = arc::card_left((top + bottom) / 2.0, CARD_LEFT);
        let mut strip = Panel::new(keys::CLUSTER_BG).with_z(CARD_Z);
        strip.layout(cx, rect(left, top, CARD_RIGHT, bottom));
        parts.push(Box::new(strip));
        top = bottom;
    }
}

/// A square dot centred on the picture point `(x, y)`.
fn dot(x: f32, y: f32, side: f32, colour: &'static str, cx: &LayoutCx<'_>) -> Box<dyn Component> {
    let mut panel = Panel::new(colour).with_z(DIAL_Z);
    panel.layout(cx, rect(x - side / 2.0, y - side / 2.0, x + side / 2.0, y + side / 2.0));
    Box::new(panel)
}

/// The tick at a number: a short stroke of dots going out from the arc.
fn tick(parts: &mut Vec<Box<dyn Component>>, point: Point, colour: &'static str, cx: &LayoutCx<'_>) {
    for k in 0..TICK_DOTS {
        let out = TICK_FROM_PX + k as f32 * TICK_PX * 0.9;
        parts.push(dot(point.x - point.inward.0 * out, point.y - point.inward.1 * out, TICK_PX, colour, cx));
    }
}

/// The number, set inside the arc at the number's point.
fn label(number: u32, point: Point, red: bool, scale: f32, cx: &LayoutCx<'_>) -> Box<dyn Component> {
    let colour = if red { keys::TICK_REDLINE } else { keys::TICK };
    let (x, y) = (point.x + point.inward.0 * LABEL_IN_PX, point.y + point.inward.1 * LABEL_IN_PX);
    let (cx_units, cy_units) = super::layout::at(x, y);
    let (width, height) = size(LABEL_BOX_PX.0, LABEL_BOX_PX.1);
    let mut text = Text::new(keys::FONT_NUMBERS, number.to_string())
        .with_z(DIAL_Z)
        .with_colour_key(colour)
        .with_scale(scale)
        .with_justification(justify::H_CENTRE | justify::V_CENTRE);
    text.layout(cx, Rect::new(cx_units, cy_units, width, height));
    Box::new(text)
}

fn arc_colour(share: f32, lit_to: f32, red_from: f32) -> &'static str {
    let lit = share <= lit_to;
    let red = share >= red_from;
    match (lit, red) {
        (true, false) => keys::ARC_LIT,
        (true, true) => keys::ARC_REDLINE,
        (false, false) => keys::ARC_DIM,
        (false, true) => keys::ARC_REDLINE_DIM,
    }
}
