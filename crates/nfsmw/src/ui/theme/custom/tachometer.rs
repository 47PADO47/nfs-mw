//! The Custom tachometer: an arc-shaped rev gauge sweeping from the lower-left to the upper-right of the cluster card.
//! The arc is a curved line of small dots, with major tick marks at each 1000 RPM and number labels (1–N) outside the
//! ticks. Dots and ticks are coloured by zone: cyan up to the warning fraction, amber to the red line, red past it.
//!
//! The tachometer draws the card background, since it is the first slot in the HUD order.

use blackbox_ui::{Component, DrawList, LayoutCx, Panel, Rect, Text, justify, token_keys};

use super::drive_feed::{DriveFeed, DriveFeedCell};
use super::drive_layout::{self, ARC_DOTS};
use super::rounded::Rounded;
use super::skin::drive;

/// The card's corner radius, in FEng units, when the theme declares none.
pub(super) const CARD_RADIUS: f32 = 6.0;

/// Where the lit dots turn amber, as a fraction of the scale (before the red line).
pub(super) const WARN_FRACTION: f32 = 0.85;

/// The Custom `Tachometer` component.
pub struct TachBar {
    feed: DriveFeedCell,
    body: Option<Body>,
}

impl TachBar {
    pub fn new(feed: DriveFeedCell) -> Self {
        Self { feed, body: None }
    }
}

impl Component for TachBar {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.body = Some(Body::of(&self.feed.snapshot(), cx));
    }

    fn build(&self, out: &mut DrawList) {
        let Some(body) = &self.body else { return };
        body.background.build(out);
        for dot in &body.dots {
            dot.build(out);
        }
        for tick in &body.ticks {
            tick.build(out);
        }
        for label in &body.labels {
            label.build(out);
        }
    }
}

struct Body {
    background: Rounded,
    dots: Vec<Panel>,
    ticks: Vec<Panel>,
    labels: Vec<Text>,
}

impl Body {
    fn of(feed: &DriveFeed, cx: &LayoutCx<'_>) -> Self {
        let radius = cx.skin.metric(token_keys::RADIUS).unwrap_or(CARD_RADIUS);
        let background = Rounded::new(drive::BACKGROUND, drive_layout::card(feed.corner), radius, cx);
        let ticks_count = tick_count(feed.scale_rpm);
        let rpm_start = 1.0 / ticks_count as f32;
        let dots = arc_dots(feed, ticks_count, rpm_start, cx);
        let ticks = arc_ticks(feed, ticks_count, rpm_start, cx);
        let labels = arc_labels(feed, ticks_count, cx);
        Self { background, dots, ticks, labels }
    }
}

/// How many major ticks to draw: one per 1000 RPM on the scale, clamped to 4–12.
fn tick_count(scale_rpm: f32) -> usize {
    if scale_rpm <= 0.0 {
        return 9;
    }
    (scale_rpm / 1000.0).round().clamp(4.0, 12.0) as usize
}

/// The RPM fraction for a given arc fraction `t`, where the arc spans from tick "1" to tick "N".
fn arc_to_rpm(t: f32, rpm_start: f32) -> f32 {
    rpm_start + t * (1.0 - rpm_start)
}

/// The dots along the arc line: lit up to the RPM fraction, dim past it.
fn arc_dots(feed: &DriveFeed, ticks: usize, rpm_start: f32, cx: &LayoutCx<'_>) -> Vec<Panel> {
    (0..ARC_DOTS)
        .map(|i| {
            let t = i as f32 / (ARC_DOTS - 1) as f32;
            let rpm_frac = arc_to_rpm(t, rpm_start);
            let colour = dot_colour(feed, rpm_frac, ticks);
            let (x, y) = drive_layout::arc_point(feed.corner, t);
            let mut panel = Panel::new(colour);
            panel.layout(cx, drive_layout::arc_dot_rect(x, y));
            panel
        })
        .collect()
}

/// The tick marks at each 1000 RPM position.
fn arc_ticks(feed: &DriveFeed, ticks: usize, rpm_start: f32, cx: &LayoutCx<'_>) -> Vec<Panel> {
    (0..ticks)
        .map(|i| {
            let t = drive_layout::tick_fraction(i, ticks);
            let rpm_frac = arc_to_rpm(t, rpm_start);
            let colour = dot_colour(feed, rpm_frac, ticks);
            let rect = drive_layout::tick_rect(feed.corner, t, drive_layout::TICK_MAJOR);
            let mut panel = Panel::new(colour);
            panel.layout(cx, rect);
            panel
        })
        .collect()
}

/// The number labels (1, 2, … N) outside each tick mark.
fn arc_labels(feed: &DriveFeed, ticks: usize, cx: &LayoutCx<'_>) -> Vec<Text> {
    (0..ticks)
        .map(|i| {
            let (x, y) = drive_layout::number_pos(feed.corner, i, ticks);
            let mut label = Text::new(drive::FONT, (i + 1).to_string())
                .with_colour_key(drive::ARC_NUMBER)
                .with_scale(0.9)
                .with_justification(justify::H_CENTRE | justify::V_CENTRE);
            label.layout(cx, Rect::new(x, y, 12.0, 12.0));
            label
        })
        .collect()
}

/// The skin colour key for a dot or tick at the given RPM fraction of the full scale.
fn dot_colour(feed: &DriveFeed, rpm_frac: f32, _ticks: usize) -> &'static str {
    if feed.rpm < rpm_frac {
        drive::TRACK
    } else if rpm_frac >= feed.red_line {
        drive::REV_REDLINE
    } else if rpm_frac >= WARN_FRACTION {
        drive::REV_WARN
    } else {
        drive::REV_LIT
    }
}
