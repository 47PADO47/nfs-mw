//! The Remastered gauges under the dial: nitrous (N2O) and the speedbreaker (SPEEDBREAKER) are a label and a pill-shaped
//! `PillBar` (a track with a gradient fill that starts in a point), filled from the left by its amount. The supercharger
//! (SUPERCHARGER) is a label and a row of segments, lit from the left, so it reads apart from the bars.

use blackbox_ui::{Component, DrawList, LayoutCx, PillBar, PillStyle, Rect, Rounded, Text, justify, token_keys};

use super::layout::{at, rect, size};
use super::skin::keys;
use crate::ui::theme::custom::DriveFeedCell;

/// The track: its left and right in picture pixels, and its height.
const BAR_LEFT_PX: f32 = 130.0;
const BAR_RIGHT_PX: f32 = 552.0;
const BAR_HEIGHT_PX: f32 = 28.0;
/// The labels end here, left of the track, and sit in a box this big.
const LABEL_RIGHT_PX: f32 = 120.0;
const LABEL_BOX_PX: (f32, f32) = (160.0, 22.0);
/// The centre of the first row. The rows hang under the card and the light line.
const ROW_Y_PX: f32 = 340.0;
const ROW_STEP_PX: f32 = 36.0;
/// The supercharger reads full at 15 psi, which the feed reports as half its fraction.
const SUPERCHARGER_FULL_PSI: f32 = 15.0;
/// The supercharger is a row of segments, not a bar: this many, with this gap between them.
const SEGMENTS: usize = 5;
const SEGMENT_GAP_PX: f32 = 6.0;

#[derive(Clone, Copy)]
pub enum Gauge {
    Nitrous,
    Speedbreaker,
    Supercharger,
}

impl Gauge {
    fn row(self) -> f32 {
        let index = match self {
            Gauge::Nitrous => 0.0,
            Gauge::Speedbreaker => 1.0,
            Gauge::Supercharger => 2.0,
        };
        ROW_Y_PX + index * ROW_STEP_PX
    }

    fn label(self) -> &'static str {
        match self {
            Gauge::Nitrous => "N2O",
            Gauge::Speedbreaker => "SPEEDBREAKER",
            Gauge::Supercharger => "SUPERCHARGER",
        }
    }

    /// The label's size: the nitrous label is set large, the longer names small.
    fn label_scale(self, base: f32) -> f32 {
        match self {
            Gauge::Nitrous => base * 1.5,
            _ => base * 0.7,
        }
    }
}

pub struct Bar {
    feed: DriveFeedCell,
    gauge: Gauge,
    parts: Vec<Box<dyn Component>>,
}

impl Bar {
    pub fn new(feed: DriveFeedCell, gauge: Gauge) -> Self {
        Self { feed, gauge, parts: Vec::new() }
    }
}

impl Component for Bar {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let feed = self.feed.snapshot();
        let amount = match self.gauge {
            Gauge::Nitrous => feed.has_nos.then_some(feed.nos),
            Gauge::Speedbreaker => Some(feed.speedbreaker),
            Gauge::Supercharger => feed.has_turbo.then(|| (feed.boost_psi / SUPERCHARGER_FULL_PSI).clamp(0.0, 1.0)),
        };
        self.parts = amount.map(|amount| parts(self.gauge, amount, cx)).unwrap_or_default();
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

fn parts(gauge: Gauge, amount: f32, cx: &LayoutCx<'_>) -> Vec<Box<dyn Component>> {
    let y = gauge.row();
    let mut parts: Vec<Box<dyn Component>> = Vec::new();
    match gauge {
        Gauge::Supercharger => segments(&mut parts, amount, y, cx),
        Gauge::Nitrous | Gauge::Speedbreaker => track(&mut parts, amount, y, cx),
    }
    let (right, label_y) = at(LABEL_RIGHT_PX, y);
    let (w, h) = size(LABEL_BOX_PX.0, LABEL_BOX_PX.1);
    let scale = gauge.label_scale(cx.skin.metric(keys::SCALE_BAR_LABEL).unwrap_or(0.8));
    let mut label = Text::new(keys::FONT_TEXT, gauge.label())
        .with_colour_key(keys::VALUE)
        .with_scale(scale)
        .with_justification(justify::H_RIGHT | justify::V_CENTRE);
    label.layout(cx, Rect::new(right - w / 2.0, label_y, w, h));
    parts.push(Box::new(label));
    parts
}

/// The pill: a dark inside and a light border, with the fill up to the amount, starting in a point.
fn track(parts: &mut Vec<Box<dyn Component>>, amount: f32, y: f32, cx: &LayoutCx<'_>) {
    let half = BAR_HEIGHT_PX / 2.0;
    let style = PillStyle { track: keys::CLUSTER_BG, border: keys::BAR_BORDER, fill: keys::BAR_FILL };
    parts.push(Box::new(PillBar::new(style, rect(BAR_LEFT_PX, y - half, BAR_RIGHT_PX, y + half), amount, true, cx)));
}

/// The segments across the row, from the left edge of the track, centred on `y`. A segment is lit while the amount reaches
/// its share of the row.
fn segments(parts: &mut Vec<Box<dyn Component>>, amount: f32, y: f32, cx: &LayoutCx<'_>) {
    let radius = cx.skin.metric(token_keys::RADIUS).unwrap_or(4.0);
    let half = BAR_HEIGHT_PX / 2.0 - 2.0;
    let width = BAR_RIGHT_PX - BAR_LEFT_PX;
    let segment_w = (width - SEGMENT_GAP_PX * (SEGMENTS as f32 - 1.0)) / SEGMENTS as f32;
    for index in 0..SEGMENTS {
        let lit = (index as f32) < amount * SEGMENTS as f32;
        let colour = if lit { keys::BAR_FILL } else { keys::BAR_TRACK };
        let left = BAR_LEFT_PX + index as f32 * (segment_w + SEGMENT_GAP_PX);
        parts.push(Box::new(Rounded::new(colour, rect(left, y - half, left + segment_w, y + half), radius, cx)));
    }
}
