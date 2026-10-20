//! The Remastered pursuit board: the chase's time in large numbers at the top right, with the minutes and seconds in the
//! accent colour and the hundredths in white, and "PURSUIT TIME" under it on a dark plate. While the cops give up the
//! plate grows a slim bar for the evade; the cop cars wrecked so far are counted beside the label.

use blackbox_ui::{Component, DrawList, LayoutCx, LinearProgress, Panel, Rect, Slant, Text, justify};

use super::skin::keys;
use crate::hud::PursuitHud;
use crate::ui::raster_text::{RASTER_TIMER_FONT_KEY, width_units};
use crate::ui::theme::custom::DriveFeedCell;

/// The plate's right edge, in FEng units, which is also where the cluster's card ends, and its width and top.
const RIGHT: f32 = 370.0;
const PLATE_WIDTH: f32 = 196.0;
const PLATE_TOP: f32 = -232.0;
const PLATE_HEIGHT: f32 = 62.0;
/// The plate fades from clear at its left to dark at its right, and has a thin light line along its top and bottom.
const PLATE_STOPS: [(f32, f32); 3] = [(0.0, 0.25), (0.45, 0.75), (1.0, 0.92)];
const EDGE_LINE: f32 = 0.8;
/// The inset of the text from the plate's right edge.
const INSET: f32 = 14.0;
const TIMER_Y: f32 = -210.0;
const LABEL_Y: f32 = -182.0;
const TIMER_SCALE: f32 = 3.5;
const LABEL_SCALE: f32 = 1.45;
/// The evade bar sits under the plate's text and the plate grows to hold it.
/// The row for the wrecked cop cars, under the label.
const WRECKED_ROW: f32 = 14.0;
const BAR_HEIGHT: f32 = 6.0;
const BAR_GAP: f32 = 8.0;
const BAR_INSET: f32 = 12.0;
const CENTRED: u32 = justify::V_CENTRE;

pub struct PursuitTimer {
    feed: DriveFeedCell,
    parts: Vec<Box<dyn Component>>,
}

impl PursuitTimer {
    pub fn new(feed: DriveFeedCell) -> Self {
        Self { feed, parts: Vec::new() }
    }
}

impl Component for PursuitTimer {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.parts = match self.feed.snapshot().pursuit {
            Some(pursuit) => parts(&pursuit, cx),
            None => Vec::new(),
        };
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// The time as the board writes it: minutes without a leading zero, then seconds and hundredths.
pub(super) fn time_parts(secs: f32) -> (String, String) {
    // A little over, so a time that is 137.43 s in the sum (and 137.42999 in a float) still reads .43.
    let hundredths = (secs.max(0.0) * 100.0 + 0.01) as u64;
    let main = format!("{}:{:02}.", hundredths / 6000, (hundredths / 100) % 60);
    (main, format!("{:02}", hundredths % 100))
}

fn parts(pursuit: &PursuitHud, cx: &LayoutCx<'_>) -> Vec<Box<dyn Component>> {
    let evading = pursuit.cooldown_shown();
    let wrecked = pursuit.cops_destroyed > 0;
    let height =
        PLATE_HEIGHT + if wrecked { WRECKED_ROW } else { 0.0 } + if evading { BAR_HEIGHT + BAR_GAP } else { 0.0 };
    let plate = Rect::new(RIGHT - PLATE_WIDTH / 2.0, PLATE_TOP + height / 2.0, PLATE_WIDTH, height);
    let mut parts: Vec<Box<dyn Component>> =
        vec![Box::new(Slant::at_default_depth(keys::CLUSTER_BG, plate, 0.0, &PLATE_STOPS, cx))];
    for y in [plate.min_y(), plate.max_y() - EDGE_LINE] {
        let mut line = Panel::new(keys::BAR_BORDER);
        line.layout(cx, Rect::new(plate.x, y + EDGE_LINE / 2.0, plate.w, EDGE_LINE));
        parts.push(Box::new(line));
    }

    // The timer's right edge is the plate's less the inset; the hundredths take their width from it and the rest ends where
    // they begin, so the two colours meet.
    let edge = RIGHT - INSET;
    let (main, hundredths) = time_parts(pursuit.secs);
    let cents_width = width_units(RASTER_TIMER_FONT_KEY, &hundredths, TIMER_SCALE).unwrap_or(30.0);
    parts.push(text(cx, &hundredths, keys::VALUE, keys::FONT_TIMER, TIMER_SCALE, edge, TIMER_Y));
    parts.push(text(cx, &main, keys::UNIT_HIGHLIGHT, keys::FONT_TIMER, TIMER_SCALE, edge - cents_width, TIMER_Y));
    parts.push(text(cx, "PURSUIT TIME", keys::VALUE, keys::FONT_MENU, LABEL_SCALE, edge, LABEL_Y));
    if wrecked {
        let wrecked = format!("{} WRECKED", pursuit.cops_destroyed);
        parts.push(text(cx, &wrecked, keys::UNIT, keys::FONT_MENU, LABEL_SCALE, edge, LABEL_Y + WRECKED_ROW));
    }
    if evading {
        let width = PLATE_WIDTH - 2.0 * BAR_INSET;
        let y = PLATE_TOP + height - BAR_HEIGHT / 2.0 - BAR_GAP / 2.0;
        let mut bar = LinearProgress::new(keys::BAR_TRACK, keys::BAR_FILL, keys::BAR_BORDER).with_value(pursuit.evade);
        bar.layout(cx, Rect::new(RIGHT - PLATE_WIDTH / 2.0, y, width, BAR_HEIGHT));
        parts.push(Box::new(bar));
    }
    parts
}

/// A string whose right edge is at `right`, centred on `y`.
fn text(cx: &LayoutCx<'_>, text: &str, colour: &str, font: &str, scale: f32, right: f32, y: f32) -> Box<dyn Component> {
    let width = 200.0;
    let mut part =
        Text::new(font, text).with_colour_key(colour).with_scale(scale).with_justification(justify::H_RIGHT | CENTRED);
    part.layout(cx, Rect::new(right - width / 2.0, y, width, 14.0));
    Box::new(part)
}

#[cfg(test)]
mod tests {
    use super::time_parts;

    #[test]
    fn the_time_drops_the_leading_zero_of_the_minutes() {
        assert_eq!(time_parts(137.43), ("2:17.".into(), "43".into()));
        assert_eq!(time_parts(0.0), ("0:00.".into(), "00".into()));
        assert_eq!(time_parts(605.07), ("10:05.".into(), "07".into()));
        assert_eq!(time_parts(-3.0), ("0:00.".into(), "00".into()));
    }
}
