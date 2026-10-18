//! The Remastered gauge bars under the dial: nitrous (N2O), the speed breaker (BRAKE) and the supercharger's boost. Each is
//! a label and a thin bar, filled from the left by its amount in one solid colour.
//!
//! The nitrous bar shows only on a car that has nitrous, and the boost bar only on a car with forced induction.
//!
//! State comes from the feed (`custom::DriveFeedCell`). `layout` reads it and lays the panels out; `build` emits them.

use blackbox_ui::{Component, DrawList, LayoutCx, Panel, Rect, Text, justify};

use super::layout::{at, size};
use super::skin::keys;
use crate::ui::theme::custom::DriveFeedCell;

/// The bar's left and right ends in reference picture points, and its height. Each row sits on its own line of the
/// picture, `ROW_STEP_PX` below the one before it, so the bars do not touch.
const BAR_LEFT_PX: f32 = 130.0;
const BAR_RIGHT_PX: f32 = 565.0;
const BAR_HEIGHT_PX: f32 = 12.0;
const LABEL_X_PX: f32 = 72.0;
const LABEL_BOX_PX: (f32, f32) = (60.0, 14.0);
const LABEL_SCALE: f32 = 1.0;
const ROW_Y_PX: f32 = 301.0;
const ROW_STEP_PX: f32 = 38.0;
/// The boost that fills the bar, in psi.
const BOOST_FULL_PSI: f32 = 30.0;

/// Which bar: its row under the dial, its label, and where its amount comes from.
#[derive(Clone, Copy)]
pub enum Gauge {
    Nitrous,
    Speedbreaker,
    Boost,
}

impl Gauge {
    fn row(self) -> f32 {
        let index = match self {
            Gauge::Nitrous => 0.0,
            Gauge::Speedbreaker => 1.0,
            Gauge::Boost => 2.0,
        };
        ROW_Y_PX + index * ROW_STEP_PX
    }

    fn label(self) -> &'static str {
        match self {
            Gauge::Nitrous => "N2O",
            Gauge::Speedbreaker => "BRAKE",
            Gauge::Boost => "BOOST",
        }
    }
}

/// The Remastered gauge bar. Built per resolve from the shared feed; it holds no state of its own.
pub struct Bar {
    feed: DriveFeedCell,
    gauge: Gauge,
    body: Option<Body>,
}

impl Bar {
    /// The bar for `gauge` that reads `feed` (the cell the Remastered theme holds).
    pub fn new(feed: DriveFeedCell, gauge: Gauge) -> Self {
        Self { feed, gauge, body: None }
    }
}

impl Component for Bar {
    /// The bar does not use `bounds`: it sits in its own row under the dial (see `layout`).
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let feed = self.feed.snapshot();
        let amount = match self.gauge {
            Gauge::Nitrous => feed.has_nos.then_some(feed.nos),
            Gauge::Speedbreaker => Some(feed.speedbreaker),
            Gauge::Boost => feed.has_turbo.then(|| (feed.boost_psi / BOOST_FULL_PSI).clamp(0.0, 1.0)),
        };
        self.body = amount.map(|amount| Body::of(self.gauge, amount, cx));
    }

    fn build(&self, out: &mut DrawList) {
        let Some(body) = &self.body else { return };
        body.track.build(out);
        body.fill.build(out);
        body.label.build(out);
    }
}

/// The laid-out parts of the bar: the track, the fill over it, and the label.
struct Body {
    track: Panel,
    fill: Panel,
    label: Text,
}

impl Body {
    fn of(gauge: Gauge, amount: f32, cx: &LayoutCx<'_>) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        let left = at(BAR_LEFT_PX, gauge.row()).0;
        let right = at(BAR_RIGHT_PX, gauge.row()).0;
        let (_, y) = at(0.0, gauge.row());
        let (_, height) = size(0.0, BAR_HEIGHT_PX);
        let width = right - left;
        let fill_width = width * amount;

        let mut track = Panel::new(keys::BAR_TRACK);
        track.layout(cx, Rect::new(left + width / 2.0, y, width, height));
        let mut fill = Panel::new(keys::BAR_FILL);
        fill.layout(cx, Rect::new(left + fill_width / 2.0, y, fill_width, height));

        let (label_x, label_y) = at(LABEL_X_PX, gauge.row());
        let (label_w, label_h) = size(LABEL_BOX_PX.0, LABEL_BOX_PX.1);
        let mut label = Text::new(keys::FONT_TEXT, gauge.label())
            .with_colour_key(keys::BAR_LABEL)
            .with_scale(LABEL_SCALE)
            .with_justification(justify::H_CENTRE | justify::V_CENTRE);
        label.layout(cx, Rect::new(label_x, label_y, label_w, label_h));
        Self { track, fill, label }
    }
}
