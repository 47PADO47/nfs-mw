//! The Remastered speedometer: the speed as a large number inside the arc, the unit under it with
//! the H of MPH in gold (secondary), and the gear to the right.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect, Text, justify};

use super::layout::{at, size};
use super::skin::keys;
use crate::ui::theme::custom::DriveFeedCell;

/// The speed sits under the arch's apex, inside the arch, and the unit under it.
const VALUE_PX: (f32, f32) = (360.0, 205.0);
const VALUE_BOX: (f32, f32) = (170.0, 60.0);
const UNIT_PX: (f32, f32) = (339.0, 272.0);
const UNIT_BOX: (f32, f32) = (80.0, 22.0);
const GEAR_PX: (f32, f32) = (514.0, 230.0);
const GEAR_BOX: (f32, f32) = (50.0, 60.0);
const CENTRED: u32 = justify::H_CENTRE | justify::V_CENTRE;

pub struct Readout {
    feed: DriveFeedCell,
    body: Option<Body>,
}

impl Readout {
    pub fn new(feed: DriveFeedCell) -> Self {
        Self { feed, body: None }
    }
}

impl Component for Readout {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let feed = self.feed.snapshot();
        let speed_scale = cx.skin.metric(keys::SCALE_SPEED).unwrap_or(4.5);
        let unit_scale = cx.skin.metric(keys::SCALE_UNIT).unwrap_or(1.6);
        let gear_scale = cx.skin.metric(keys::SCALE_GEAR).unwrap_or(3.2);

        let mut texts = vec![
            placed(
                feed.speed.to_string(),
                keys::VALUE,
                keys::FONT_NUMBERS,
                speed_scale,
                VALUE_PX,
                VALUE_BOX,
                CENTRED,
                cx,
            ),
            placed(gear_label(feed.gear), keys::GEAR, keys::FONT_NUMBERS, gear_scale, GEAR_PX, GEAR_BOX, CENTRED, cx),
        ];
        if feed.mph {
            texts.extend(unit_pair("MP", "H", unit_scale, cx));
        } else {
            texts.extend(unit_pair("KM", "/H", unit_scale, cx));
        }
        self.body = Some(Body { texts });
    }

    fn build(&self, out: &mut DrawList) {
        let Some(body) = &self.body else { return };
        for text in &body.texts {
            text.build(out);
        }
    }
}

struct Body {
    texts: Vec<Text>,
}

/// The unit's two parts, the highlighted one (gold) and the rest (muted). They meet at the unit's centre line: the
/// highlighted part ends there and the rest starts there, so a wider font cannot make them overlap.
fn unit_pair(highlight: &str, rest: &str, scale: f32, cx: &LayoutCx<'_>) -> [Text; 2] {
    let (x, y) = UNIT_PX;
    let half = UNIT_BOX.0 / 2.0;
    [
        placed(
            highlight.into(),
            keys::UNIT_HIGHLIGHT,
            keys::FONT_TEXT,
            scale,
            (x - half, y),
            UNIT_BOX,
            justify::H_RIGHT | justify::V_CENTRE,
            cx,
        ),
        placed(rest.into(), keys::UNIT, keys::FONT_TEXT, scale, (x + half, y), UNIT_BOX, justify::V_CENTRE, cx),
    ]
}

/// A text of `content` in its box, placed at `at_px` (its centre) and justified by `justification`.
#[allow(clippy::too_many_arguments)]
fn placed(
    content: String,
    colour: &'static str,
    font: &'static str,
    scale: f32,
    at_px: (f32, f32),
    box_px: (f32, f32),
    justification: u32,
    cx: &LayoutCx<'_>,
) -> Text {
    let (x, y) = at(at_px.0, at_px.1);
    let (width, height) = size(box_px.0, box_px.1);
    let mut text = Text::new(font, content).with_colour_key(colour).with_scale(scale).with_justification(justification);
    text.layout(cx, Rect::new(x, y, width, height));
    text
}

fn gear_label(gear: i32) -> String {
    match gear {
        -1 => "R".into(),
        0 => "N".into(),
        n => n.to_string(),
    }
}
