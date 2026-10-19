//! The Remastered gauges under the dial: nitrous (N2O) and the speedbreaker (SPEEDBREAKER) are a label and a rounded
//! `LinearProgress` bar, filled from the left by its amount. The supercharger (SUPERCHARGER) is a label and a row of
//! segments, lit from the left, so it reads apart from the bars.

use blackbox_ui::{Component, DrawList, LayoutCx, LinearProgress, Rect, Rounded, Text, justify, token_keys};

use super::layout::{at, size};
use super::skin::keys;
use crate::ui::theme::custom::DriveFeedCell;

const BAR_LEFT_PX: f32 = 250.0;
const BAR_RIGHT_PX: f32 = 600.0;
const BAR_HEIGHT_PX: f32 = 18.0;
const LABEL_X_PX: f32 = 135.0;
const LABEL_BOX_PX: (f32, f32) = (220.0, 18.0);
/// The centre of the first bar's row. It sits below the tachometer's lower end with room to spare.
const ROW_Y_PX: f32 = 322.0;
const ROW_STEP_PX: f32 = 26.0;
/// The supercharger reads full at 15 psi, which the feed reports as half its fraction.
const SUPERCHARGER_FULL_PSI: f32 = 15.0;
/// The supercharger is a row of segments, not a bar: this many, with this gap between them.
const SEGMENTS: usize = 5;
const SEGMENT_GAP_PX: f32 = 4.0;

/// The picture's y of the bottom edge of the lowest bar row, which the cluster's card wraps.
pub(super) fn bottom_px() -> f32 {
    ROW_Y_PX + 2.0 * ROW_STEP_PX + BAR_HEIGHT_PX / 2.0
}

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
}

pub struct Bar {
    feed: DriveFeedCell,
    gauge: Gauge,
    body: Option<Body>,
}

impl Bar {
    pub fn new(feed: DriveFeedCell, gauge: Gauge) -> Self {
        Self { feed, gauge, body: None }
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
        self.body = amount.map(|amount| Body::of(self.gauge, amount, cx));
    }

    fn build(&self, out: &mut DrawList) {
        let Some(body) = &self.body else { return };
        body.track.build(out);
        body.label.build(out);
    }
}

/// What fills the row: the bar, or the supercharger's segments.
enum Track {
    Bar(LinearProgress),
    Segments(Vec<Rounded>),
}

impl Track {
    fn build(&self, out: &mut DrawList) {
        match self {
            Track::Bar(bar) => bar.build(out),
            Track::Segments(segments) => {
                for segment in segments {
                    segment.build(out);
                }
            }
        }
    }
}

struct Body {
    track: Track,
    label: Text,
}

impl Body {
    fn of(gauge: Gauge, amount: f32, cx: &LayoutCx<'_>) -> Self {
        let left = at(BAR_LEFT_PX, gauge.row()).0;
        let right = at(BAR_RIGHT_PX, gauge.row()).0;
        let (_, y) = at(0.0, gauge.row());
        let (_, height) = size(0.0, BAR_HEIGHT_PX);
        let width = right - left;

        let track = match gauge {
            Gauge::Supercharger => Track::Segments(segments(amount, left, width, y, height, cx)),
            Gauge::Nitrous | Gauge::Speedbreaker => {
                let mut progress =
                    LinearProgress::new(keys::BAR_TRACK, keys::BAR_FILL, keys::BAR_BORDER).with_value(amount);
                progress.layout(cx, Rect::new(left + width / 2.0, y, width, height));
                Track::Bar(progress)
            }
        };

        let label_scale = cx.skin.metric(keys::SCALE_BAR_LABEL).unwrap_or(1.0);
        let (label_x, label_y) = at(LABEL_X_PX, gauge.row());
        let (label_w, label_h) = size(LABEL_BOX_PX.0, LABEL_BOX_PX.1);
        let mut label = Text::new(keys::FONT_TEXT, gauge.label())
            .with_colour_key(keys::BAR_LABEL)
            .with_scale(label_scale)
            .with_justification(justify::H_CENTRE | justify::V_CENTRE);
        label.layout(cx, Rect::new(label_x, label_y, label_w, label_h));
        Self { track, label }
    }
}

/// The segments across the row, from the left edge `left` of width `width`, centred on `y`. A segment is lit while the
/// amount reaches its share of the row.
fn segments(amount: f32, left: f32, width: f32, y: f32, height: f32, cx: &LayoutCx<'_>) -> Vec<Rounded> {
    let radius = cx.skin.metric(token_keys::RADIUS).unwrap_or(4.0);
    let gap = size(SEGMENT_GAP_PX, 0.0).0;
    let segment_w = (width - gap * (SEGMENTS as f32 - 1.0)) / SEGMENTS as f32;
    (0..SEGMENTS)
        .map(|index| {
            let lit = (index as f32) < amount * SEGMENTS as f32;
            let colour = if lit { keys::BAR_FILL } else { keys::BAR_TRACK };
            let x = left + index as f32 * (segment_w + gap) + segment_w / 2.0;
            Rounded::new(colour, Rect::new(x, y, segment_w, height), radius, cx)
        })
        .collect()
}
