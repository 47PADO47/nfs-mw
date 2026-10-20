//! The Remastered pursuit panel: the chase's time in large numbers at the top right on a dark right trapezoid, with the minutes
//! and seconds in the accent colour and the hundredths in white; under it the cop cars the chase has called, how many of them
//! are damaged and how many are wrecked; and the two bars of the chase's end: the meter of being busted or escaping, and the
//! cool-down once the player has got away.

use blackbox_ui::{Component, DrawList, LayoutCx, LinearProgress, Panel, Rect, Text, Trapezoid, justify};

use super::skin::keys;
use crate::hud::PursuitHud;
use crate::ui::raster_text::{RASTER_TIMER_FONT_KEY, width_units};
use crate::ui::theme::custom::{DriveFeed, DriveFeedCell};

/// The plate's right edge, in FEng units, which is also where the cluster's card ends, its width and its top, and how far its
/// left edge slants in from top to bottom.
const RIGHT: f32 = 370.0;
const PLATE_WIDTH: f32 = 220.0;
const PLATE_TOP: f32 = -232.0;
const SLANT: f32 = 24.0;
/// The plate fades from clear at its left to dark at its right.
const PLATE_STOPS: [(f32, f32); 3] = [(0.0, 0.3), (0.4, 0.78), (1.0, 0.93)];
/// The thin lines along the plate's top and bottom, and the accent bar down its right edge.
const EDGE_LINE: f32 = 0.8;
const SIDE_BAR: f32 = 2.5;
/// The inset of the content from the plate's right edge, and from the slanted edge's foot.
const INSET: f32 = 14.0;
const FOOT_INSET: f32 = 10.0;
/// The timer's and its label's heights below the plate's top, and their sizes.
const TIMER_Y: f32 = 22.0;
const LABEL_Y: f32 = 50.0;
const TIMER_SCALE: f32 = 3.5;
const LABEL_SCALE: f32 = 1.45;
/// The head of the plate, down to the cop counts; the counts' row, and the labels and numbers in it.
const HEAD: f32 = 62.0;
const COUNTS_HEIGHT: f32 = 34.0;
const COUNT_LABEL_Y: f32 = 6.0;
const COUNT_VALUE_Y: f32 = 21.0;
const COUNT_LABEL_SCALE: f32 = 1.05;
const COUNT_VALUE_SCALE: f32 = 2.3;
/// A bar's row: its height, and the label above its bar and the bar's place in it.
const BAR_ROW: f32 = 24.0;
const BAR_LABEL_Y: f32 = 7.0;
const BAR_Y: f32 = 17.0;
const BAR_HEIGHT: f32 = 6.0;
const BAR_LABEL_SCALE: f32 = 1.2;
/// The room under the last row.
const FOOT: f32 = 6.0;
/// The signed meter is shown from this much of it, and is full at one.
const METER_SHOWN: f32 = 0.01;

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
        let feed = self.feed.snapshot();
        self.parts = match feed.pursuit {
            Some(pursuit) => parts(&feed, &pursuit, cx),
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

/// The signed meter as the panel shows it: the label and the fill of the side it is on, or `None` while it rests at zero. A
/// positive value is the busted side (the red bar), a negative one the escaping side (the accent).
pub(super) fn meter(busted: f32) -> Option<(&'static str, f32)> {
    let fill = busted.abs().clamp(0.0, 1.0);
    match busted {
        t if t >= 1.0 => Some(("BUSTED", 1.0)),
        t if t >= METER_SHOWN => Some(("BUSTED", fill)),
        t if t <= -METER_SHOWN => Some(("ESCAPING", fill)),
        _ => None,
    }
}

/// How tall the plate is for what the chase shows: the head, the counts, and a row for each bar that is up.
fn plate_height(meter: bool, cooldown: bool) -> f32 {
    HEAD + COUNTS_HEIGHT + BAR_ROW * (usize::from(meter) + usize::from(cooldown)) as f32 + FOOT
}

fn parts(feed: &DriveFeed, pursuit: &PursuitHud, cx: &LayoutCx<'_>) -> Vec<Box<dyn Component>> {
    let signed = meter(pursuit.busted);
    let cooling = pursuit.cooldown_shown();
    let height = plate_height(signed.is_some(), cooling);
    let plate = Rect::new(RIGHT - PLATE_WIDTH / 2.0, PLATE_TOP + height / 2.0, PLATE_WIDTH, height);
    let mut parts: Vec<Box<dyn Component>> =
        vec![Box::new(Trapezoid::at_default_depth(keys::CLUSTER_BG, plate, SLANT, &PLATE_STOPS, cx))];
    let mut line = |key: &str, rect: Rect| {
        let mut panel = Panel::new(key.to_owned());
        panel.layout(cx, rect);
        parts.push(Box::new(panel));
    };
    line(keys::BAR_BORDER, Rect::new(plate.x, plate.min_y() + EDGE_LINE / 2.0, plate.w, EDGE_LINE));
    let foot = plate.min_x() + SLANT;
    line(
        keys::BAR_BORDER,
        Rect::new((foot + plate.max_x()) / 2.0, plate.max_y() - EDGE_LINE / 2.0, plate.max_x() - foot, EDGE_LINE),
    );
    let side =
        if signed.is_some_and(|(label, _)| label == "BUSTED") { keys::PURSUIT_BUSTED } else { keys::UNIT_HIGHLIGHT };
    line(side, Rect::new(plate.max_x() - SIDE_BAR / 2.0, plate.y, SIDE_BAR, height));

    // The timer's right edge is the plate's less the inset; the hundredths take their width from it and the rest ends where
    // they begin, so the two colours meet.
    let edge = RIGHT - INSET;
    let top = plate.min_y();
    let (main, hundredths) = time_parts(pursuit.secs);
    let cents_width = width_units(RASTER_TIMER_FONT_KEY, &hundredths, TIMER_SCALE).unwrap_or(30.0);
    let at = |y: f32| top + y;
    parts.push(text(cx, &hundredths, keys::VALUE, keys::FONT_TIMER, TIMER_SCALE, (edge, at(TIMER_Y)), Align::Right));
    parts.push(text(
        cx,
        &main,
        keys::UNIT_HIGHLIGHT,
        keys::FONT_TIMER,
        TIMER_SCALE,
        (edge - cents_width, at(TIMER_Y)),
        Align::Right,
    ));
    parts.push(text(cx, "PURSUIT TIME", keys::VALUE, keys::FONT_MENU, LABEL_SCALE, (edge, at(LABEL_Y)), Align::Right));

    // The cop cars: called, damaged, wrecked.
    let content = (plate.min_x() + SLANT + FOOT_INSET, edge);
    let cell = (content.1 - content.0) / 3.0;
    let counts = [
        ("CALLED", feed.cops_engaged, keys::PURSUIT_CALLED),
        ("DAMAGED", feed.cops_damaged, keys::PURSUIT_DAMAGED),
        ("WRECKED", pursuit.cops_destroyed, keys::PURSUIT_WRECKED),
    ];
    for (i, (label, count, colour)) in counts.into_iter().enumerate() {
        let x = content.0 + cell * (i as f32 + 0.5);
        let y = top + HEAD;
        parts.push(text(
            cx,
            label,
            keys::UNIT,
            keys::FONT_MENU,
            COUNT_LABEL_SCALE,
            (x, y + COUNT_LABEL_Y),
            Align::Centre,
        ));
        let colour = if count == 0 { keys::UNIT } else { colour };
        parts.push(text(
            cx,
            &count.to_string(),
            colour,
            keys::FONT_MENU_TITLE,
            COUNT_VALUE_SCALE,
            (x, y + COUNT_VALUE_Y),
            Align::Centre,
        ));
    }

    // The bars of the chase's end, one under the other.
    let mut row = top + HEAD + COUNTS_HEIGHT;
    if let Some((label, fill)) = signed {
        let key = if label == "BUSTED" { keys::PURSUIT_BUSTED } else { keys::UNIT_HIGHLIGHT };
        bar(&mut parts, cx, label, key, fill, content, row);
        row += BAR_ROW;
    }
    if cooling {
        bar(&mut parts, cx, "COOLDOWN", keys::VALUE, pursuit.evade, content, row);
    }
    parts
}

/// A bar with its label above its left end and its share above its right end, in the row that starts at `row`.
fn bar(
    parts: &mut Vec<Box<dyn Component>>,
    cx: &LayoutCx<'_>,
    label: &str,
    colour: &'static str,
    fill: f32,
    content: (f32, f32),
    row: f32,
) {
    parts.push(text(cx, label, colour, keys::FONT_MENU, BAR_LABEL_SCALE, (content.0, row + BAR_LABEL_Y), Align::Left));
    let share = format!("{}%", (fill.clamp(0.0, 1.0) * 100.0).round() as u32);
    parts.push(text(
        cx,
        &share,
        keys::UNIT,
        keys::FONT_MENU,
        BAR_LABEL_SCALE,
        (content.1, row + BAR_LABEL_Y),
        Align::Right,
    ));
    let mut bar = LinearProgress::new(keys::BAR_TRACK, colour, keys::BAR_BORDER).with_value(fill);
    bar.layout(cx, Rect::new((content.0 + content.1) / 2.0, row + BAR_Y, content.1 - content.0, BAR_HEIGHT));
    parts.push(Box::new(bar));
}

/// Which side of its point a string is placed on.
#[derive(Clone, Copy)]
enum Align {
    Left,
    Centre,
    Right,
}

/// A string at `at` (its left, middle or right edge, centred on the height), as one part.
fn text(
    cx: &LayoutCx<'_>,
    text: &str,
    colour: &str,
    font: &str,
    scale: f32,
    at: (f32, f32),
    align: Align,
) -> Box<dyn Component> {
    let width = 200.0;
    let (x, flags) = match align {
        Align::Left => (at.0 + width / 2.0, justify::V_CENTRE),
        Align::Centre => (at.0, justify::H_CENTRE | justify::V_CENTRE),
        Align::Right => (at.0 - width / 2.0, justify::H_RIGHT | justify::V_CENTRE),
    };
    let mut part = Text::new(font, text).with_colour_key(colour).with_scale(scale).with_justification(flags);
    part.layout(cx, Rect::new(x, at.1, width, 14.0));
    Box::new(part)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_time_drops_the_leading_zero_of_the_minutes() {
        assert_eq!(time_parts(137.43), ("2:17.".into(), "43".into()));
        assert_eq!(time_parts(0.0), ("0:00.".into(), "00".into()));
        assert_eq!(time_parts(605.07), ("10:05.".into(), "07".into()));
        assert_eq!(time_parts(-3.0), ("0:00.".into(), "00".into()));
    }

    #[test]
    fn the_signed_meter_is_busted_above_zero_escaping_below_it_and_hidden_at_rest() {
        assert_eq!(meter(0.0), None);
        assert_eq!(meter(0.004), None);
        assert_eq!(meter(0.4), Some(("BUSTED", 0.4)));
        assert_eq!(meter(1.0), Some(("BUSTED", 1.0)));
        assert_eq!(meter(-0.25), Some(("ESCAPING", 0.25)));
        assert_eq!(meter(-3.0), Some(("ESCAPING", 1.0)), "a value past the end is a full bar");
    }

    #[test]
    fn the_plate_grows_by_a_row_for_each_bar() {
        let (rest, one, two) = (plate_height(false, false), plate_height(true, false), plate_height(true, true));
        assert_eq!(one - rest, BAR_ROW);
        assert_eq!(two - one, BAR_ROW);
        assert_eq!(plate_height(false, true), one);
    }
}
