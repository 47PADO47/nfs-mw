//! The Remastered tachometer: a quarter-ellipse arc from its foot at the bottom left up and over the top of the card, with a
//! short run along the top, the rev numbers set along it one thousand rpm apart with a tick for each, and a dark card inside
//! the arc whose sides lean, with a lighter panel at its right for the gear. The arc lights up as the revs rise and turns to
//! the accent colour from the red line. A thin light line under the card stands on the gauges' band.

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
/// Where the card's right side stands at its bottom, and the card's bottom, in picture pixels, and the height of a strip.
const CARD_RIGHT: f32 = 526.0;
const CARD_BOTTOM: f32 = 289.0;
const STRIP_PX: f32 = 2.0;
/// The gear panel: its left edge and top. It runs to the card's bottom and its leaning right side.
const GEAR_LEFT: f32 = 478.0;
const GEAR_TOP: f32 = 108.0;
/// The band under the card that the gauges stand on: its span and height.
const BAND: (f32, f32, f32, f32) = (15.0, 293.0, 585.0, 358.0);
/// The light line under the card.
const BASE_LINE: (f32, f32, f32, f32) = (211.0, 298.0, 338.0, 300.0);
/// How long the arc's foot is, the arc's stroke and the steps it is drawn in, in picture pixels.
const FOOT_LENGTH: f32 = 13.0;
const STROKE_PX: f32 = 2.6;
const STEP_PX: f32 = 1.1;
/// A tick starts this far outside the arc and is this long, in dots of `TICK_PX`.
const TICK_FROM_PX: f32 = 3.0;
const TICK_DOTS: usize = 4;
const TICK_PX: f32 = 2.2;
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
        let mut band = Panel::new(keys::GEAR_BAND).with_z(CARD_Z + 0.1);
        band.layout(cx, rect(BAND.0, BAND.1, BAND.2, BAND.3));
        parts.push(Box::new(band));
        card(&mut parts, cx);
        let line = rect(BASE_LINE.0, BASE_LINE.1, BASE_LINE.2, BASE_LINE.3);
        let ramp = [(0.0, 0.0), (0.2, 0.8), (0.8, 0.8), (1.0, 0.0)];
        parts.push(Box::new(Slant::new(keys::TICK, line, 0.0, &ramp, DIAL_Z, cx)));

        // The foot: the arc turns right at its start, so it stands on a short line.
        for index in 0..(FOOT_LENGTH / STEP_PX) as usize {
            let x = arc::FOOT_X + index as f32 * STEP_PX;
            parts.push(dot(x, arc::FOOT_Y + STROKE_PX / 2.0, STROKE_PX, arc_colour(0.0, lit_to, red_from), cx));
        }
        let steps = (path.length() / STEP_PX).round() as usize;
        for index in 0..steps {
            let point = path.at((index as f32 + 0.5) / steps as f32);
            parts.push(dot(point.x, point.y, STROKE_PX, arc_colour(point.share, lit_to, red_from), cx));
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

/// The dark card and the gear panel on it: strips as tall as `STRIP_PX`, each running from the arc (or the leaning corner)
/// to the card's leaning right side. The gear panel is the strips' right part, from its left edge down to the card's bottom.
fn card(parts: &mut Vec<Box<dyn Component>>, cx: &LayoutCx<'_>) {
    let mut top = arc::TOP;
    while top < CARD_BOTTOM {
        let bottom = (top + STRIP_PX).min(CARD_BOTTOM);
        let middle = (top + bottom) / 2.0;
        let (left, right) = (arc::card_left(middle), arc::card_right(middle, CARD_BOTTOM, CARD_RIGHT));
        let mut strip = Panel::new(keys::CLUSTER_BG).with_z(CARD_Z);
        strip.layout(cx, rect(left, top, right, bottom));
        parts.push(Box::new(strip));
        if top >= GEAR_TOP {
            let mut panel = Panel::new(keys::GEAR_PANEL).with_z(PANEL_Z);
            panel.layout(cx, rect(GEAR_LEFT, top, right, bottom));
            parts.push(Box::new(panel));
        }
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
