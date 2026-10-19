//! The Remastered tachometer: a half-circle arch from the card's left edge over the top to its right edge, with the rpm
//! numbered in thousands. The arch lights up as the revs rise and turns gold past the red line. A dark rounded card
//! sits behind the cluster so the numbers stay readable over the game scene.

use blackbox_ui::{Component, DrawList, LayoutCx, Panel, Rect, Rounded, Text, justify, token_keys};

use super::bar;
use super::layout::{at, picture_height, size};
use super::skin::keys;
use crate::ui::theme::custom::DriveFeedCell;

/// The card's left edge and width in reference picture pixels. Its top and bottom follow the tick numbers and the
/// lowest bar, with the theme's padding around them.
const CARD_X_PX: (f32, f32) = (20.0, 680.0);
/// Depths: a lower `z` draws on top. The card sits under the bars, which are at the default depth, and the dial sits over
/// them.
const CARD_Z: f32 = 2.0;
const DIAL_Z: f32 = 0.5;
/// Arch geometry. The centre is the middle of the card, and the arch's ends sit at its left and right edges. The
/// arch's ends are at the centre's height, and its top is `RADIUS_PX / ASPECT` above that.
const CENTRE_PX: (f32, f32) = (360.0, 306.0);
const RADIUS_PX: f32 = 320.0;
const START_DEG: f32 = 180.0;
const END_DEG: f32 = 0.0;
const STEP_PX: f32 = 2.0;
const PANEL_PX: f32 = 5.0;
/// The tick numbers sit this far inside the arch, so they clear both the arc line and the card's edge.
const LABEL_IN_PX: f32 = 22.0;
const LABEL_BOX_PX: (f32, f32) = (28.0, 20.0);
const ASPECT: f32 = 2.0 / 1.5;

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
        body.card.build(out);
        for panel in &body.arc {
            panel.build(out);
        }
        for tick in &body.ticks {
            tick.build(out);
        }
    }
}

struct Body {
    card: Rounded,
    arc: Vec<Panel>,
    ticks: Vec<Text>,
}

impl Body {
    fn of(rpm: f32, red_line: f32, scale_rpm: f32, cx: &LayoutCx<'_>) -> Self {
        let card = card(cx);
        let tick_scale = cx.skin.metric(keys::SCALE_TICK).unwrap_or(1.4);

        let arc = (0..step_count())
            .map(|index| {
                let fraction = step_fraction(index);
                let (x, y) = point(fraction, RADIUS_PX);
                let (width, height) = size(PANEL_PX, PANEL_PX);
                let mut panel = Panel::new(arc_colour(fraction, rpm, red_line)).with_z(DIAL_Z);
                panel.layout(cx, Rect::new(x, y, width, height));
                panel
            })
            .collect();

        let ticks = tick_fractions(scale_rpm)
            .map(|(number, fraction)| {
                let (x, y) = point(fraction, RADIUS_PX - LABEL_IN_PX);
                let (width, height) = size(LABEL_BOX_PX.0, LABEL_BOX_PX.1);
                let colour = if fraction >= red_line { keys::TICK_REDLINE } else { keys::TICK };
                let mut tick = Text::new(keys::FONT_NUMBERS, number.to_string())
                    .with_z(DIAL_Z)
                    .with_colour_key(colour)
                    .with_scale(tick_scale)
                    .with_justification(justify::H_CENTRE | justify::V_CENTRE);
                tick.layout(cx, Rect::new(x, y, width, height));
                tick
            })
            .collect();

        Self { card, arc, ticks }
    }
}

/// The rounded card behind the cluster, from the top of the highest tick number to the bottom of the lowest bar.
fn card(cx: &LayoutCx<'_>) -> Rounded {
    let radius = cx.skin.metric(token_keys::RADIUS).unwrap_or(0.0);
    let pad = picture_height(cx.skin.metric(token_keys::PADDING).unwrap_or(0.0));
    let top_px = CENTRE_PX.1 - RADIUS_PX / ASPECT - PANEL_PX / 2.0 - pad;
    let bottom_px = bar::bottom_px() + pad;
    let (x, y) = at(CARD_X_PX.0, top_px);
    let (w, h) = size(CARD_X_PX.1, bottom_px - top_px);
    let centre = Rect::new(x + w / 2.0, y + h / 2.0, w, h);
    Rounded::at_depth(keys::CLUSTER_BG, centre, radius, CARD_Z, cx)
}

fn point(fraction: f32, radius: f32) -> (f32, f32) {
    let angle = (START_DEG + (END_DEG - START_DEG) * fraction).to_radians();
    at(CENTRE_PX.0 + radius * angle.cos(), CENTRE_PX.1 - (radius / ASPECT) * angle.sin())
}

fn step_count() -> usize {
    ((START_DEG - END_DEG).abs().to_radians() * RADIUS_PX / STEP_PX).round() as usize
}

fn step_fraction(index: usize) -> f32 {
    (index as f32 + 0.5) / step_count() as f32
}

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

fn tick_fractions(scale_rpm: f32) -> impl Iterator<Item = (u32, f32)> {
    let top = (scale_rpm / 1000.0).floor() as u32;
    (1..=top).map(move |number| (number, number as f32 * 1000.0 / scale_rpm))
}
