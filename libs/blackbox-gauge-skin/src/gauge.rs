//! One dial: its layers back to front, and its needle. Spec: `docs/specs/custom-hud-gauges.md` sections 5 and 6.

use crate::draw::{Quad, Sector, Turn};
use crate::element::{Gauge, Picture};
use crate::needle;
use crate::place::{Placement, Rect};
use crate::value::Colour;

/// How fast a `HideOnMaxValue` gauge fades, per second.
const FADE_PER_SECOND: f32 = 3.0;
const FULL: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

/// What a gauge reads this frame: the plugin's getters for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Feed {
    /// The value the needle shows, in the gauge's own units.
    pub value: f32,
    /// The red line, thousands of rpm: what the near-red-line colours and the red-zone numbers start from.
    pub red_line: Option<f32>,
    /// The end of the red zone's numbers (the gauge's scale value for the tachometer).
    pub scale_end: Option<f32>,
    /// The filled arc's start and end values.
    pub fill: Option<(f32, f32)>,
    /// The gauge is drawn at all.
    pub visible: bool,
    /// The needle takes its perfect-zone colour.
    pub perfect: bool,
}

/// The layers of a gauge, and its needle, which the caller places in the order of the whole overlay.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Drawn {
    pub layers: Vec<Quad>,
    pub needle: Option<Quad>,
}

fn plain(picture: &Picture, rect: Rect, colour: Colour) -> Quad {
    Quad { picture: picture.file.clone(), additive: picture.additive, rect, uv: FULL, colour, turn: None, sector: None }
}

/// The angle of `value` on the gauge's scale, with no clamping.
fn angle_at(gauge: &Gauge, value: f32) -> f32 {
    if gauge.value <= 0.0 {
        return gauge.min_angle;
    }
    gauge.min_angle + (gauge.max_angle - gauge.min_angle) / gauge.value * value
}

/// Moves a gauge's fade toward 0 while its value is at the end of its scale (or exactly 0), toward 1 otherwise.
fn step_fade(fade: &mut f32, scale_end: f32, value: f32, dt: f32) {
    let step = FADE_PER_SECOND * dt;
    *fade = match scale_end <= value || value == 0.0 {
        true => (*fade - step).max(0.0),
        false => (*fade + step).min(1.0),
    };
}

/// The layers and the needle of `gauge` for this frame. `fade` is the gauge's fade, kept from frame to frame.
pub(crate) fn draw(gauge: &Gauge, feed: &Feed, fade: &mut f32, dt: f32, placement: &Placement) -> Drawn {
    let mut drawn = Drawn::default();
    if !feed.visible {
        return drawn;
    }
    let factor = match gauge.hide_on_max {
        true => {
            step_fade(fade, gauge.value, feed.value, dt);
            *fade
        }
        false => 1.0,
    };
    let common = &gauge.common;
    let square = placement.rect(common.size, common.size, common.x, common.y);

    if gauge.background.is_set() {
        drawn.layers.push(plain(&gauge.background, square, gauge.background.colour.faded(factor)));
    }
    if gauge.numbers.is_set() {
        let near = needle::near_redline(gauge.numbers_max_threshold, feed.value, feed.red_line);
        let colour = match near {
            true => gauge.numbers_max_colour,
            false => gauge.numbers.colour,
        };
        drawn.layers.push(plain(&gauge.numbers, square, colour.faded(factor)));
    }
    if let (true, Some((from, to))) = (gauge.arrow_masked.picture.is_set(), feed.fill) {
        let arc = &gauge.arrow_masked;
        let near = needle::near_redline(arc.max_threshold, feed.value, feed.red_line);
        let tint = match near {
            true => arc.max_colour,
            false => arc.picture.colour,
        };
        let sector = Sector {
            start: angle_at(gauge, from) - arc.min_offset,
            end: needle::angle(gauge.min_angle, gauge.max_angle, gauge.value, to, gauge.can_be_negative),
            inside: arc.inside.alpha(),
            outside: arc.outside.alpha(),
            fine: false,
        };
        drawn.layers.push(Quad { sector: Some(sector), ..plain(&arc.picture, square, tint.faded(factor)) });
    }
    if let (true, Some(red_line), Some(end)) = (gauge.numbers_masked.picture.is_set(), feed.red_line, feed.scale_end) {
        let zone = &gauge.numbers_masked;
        let (a, b) = (angle_at(gauge, red_line), angle_at(gauge, end));
        let (low, high) = match a <= b {
            true => (a, b),
            false => (b, a),
        };
        let sector = Sector {
            start: low,
            end: high + zone.max_offset,
            inside: zone.inside.alpha(),
            outside: zone.outside.alpha(),
            fine: true,
        };
        drawn
            .layers
            .push(Quad { sector: Some(sector), ..plain(&zone.picture, square, zone.picture.colour.faded(factor)) });
    }
    if gauge.arrow.picture.is_set() {
        drawn.needle = Some(needle_quad(gauge, feed, factor, placement));
    }
    drawn
}

fn needle_quad(gauge: &Gauge, feed: &Feed, factor: f32, placement: &Placement) -> Quad {
    let arrow = &gauge.arrow;
    let common = &gauge.common;
    let shape = needle::shape(common.size, arrow.scale, arrow.centre_offset);
    let angle = needle::angle(gauge.min_angle, gauge.max_angle, gauge.value, feed.value, gauge.can_be_negative);
    let (cx, cy) = placement.centre(common.size, common.x, common.y);
    let unit = placement.unit();
    let rect = Rect {
        x: cx - (shape.pivot_x - shape.width / 2.0) * unit,
        y: cy,
        w: shape.width * unit,
        h: shape.height * unit,
    };
    let colour = match feed.perfect && !arrow.perfect_colour.is_none() {
        true => arrow.perfect_colour,
        false => arrow.picture.colour,
    };
    Quad {
        turn: Some(Turn { radians: angle.to_radians(), about: (cx, cy) }),
        ..plain(&arrow.picture, rect, colour.faded(factor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::Skin;

    /// A tachometer written for these tests, 330 px at (30, 60).
    fn tachometer() -> Gauge {
        let text = "[TACHOMETER_GAUGE]\nEnabled = 1\nSize = 330\nPositionX = 30\nPositionY = 60\nValue = 10\nArrowTexture = needle.dds\nArrowColor = 0xFFFFFFFF\nArrowPerfectZoneColor = 0xFFFF0000\nArrowMinAngle = -30\nArrowMaxAngle = 210\nBackgroundTexture = dial.dds\nBackgroundColor = 0xFF8ec8ff\nNumbersTexture = numbers.dds\nNumbersColor = 0xFFAAAAAA\nNumbersMaxColor = 0xFFFF1E1E\nNumbersMaxThreshold = 0.5\nNumbersMaskedTexture = numbers.dds\nNumbersMaskedColor = 0xFFFF0000\nNumbersMaskedColor2 = 0xFFFFFFFF\nNumbersMaskedMaxOffset = 15\nArrowMaskedTexture = fill.dds\nArrowMaskedColor = 0xCCFFFFFF\nArrowMaskedColor1 = 0x3C000000\nArrowMaskedColor2 = 0xFFFFFFFF\n";
        Skin::parse(text).tachometer.expect("the sample enables the tachometer")
    }

    fn feed(rpm_thousands: f32) -> Feed {
        Feed {
            value: rpm_thousands,
            red_line: Some(7.5),
            scale_end: Some(10.0),
            fill: Some((0.0, rpm_thousands)),
            visible: true,
            perfect: false,
        }
    }

    fn placement() -> Placement {
        Placement::new(480.0 * 16.0 / 9.0, 480.0, 1.0, 0.0, 0.0)
    }

    #[test]
    fn the_layers_come_back_to_front_and_the_needle_is_left_for_the_caller() {
        let drawn = draw(&tachometer(), &feed(3.0), &mut 1.0, 0.016, &placement());
        let pictures: Vec<&str> = drawn.layers.iter().map(|q| q.picture.as_str()).collect();
        assert_eq!(pictures, vec!["dial.dds", "numbers.dds", "fill.dds", "numbers.dds"]);
        let needle = drawn.needle.expect("the sample has a needle");
        assert_eq!(needle.picture, "needle.dds");
    }

    #[test]
    fn a_hidden_gauge_draws_nothing() {
        let hidden = Feed { visible: false, ..feed(3.0) };
        assert_eq!(draw(&tachometer(), &hidden, &mut 1.0, 0.016, &placement()), Drawn::default());
    }

    #[test]
    fn the_numbers_change_colour_near_the_red_line() {
        let far = draw(&tachometer(), &feed(5.0), &mut 1.0, 0.0, &placement());
        let near = draw(&tachometer(), &feed(7.2), &mut 1.0, 0.0, &placement());
        assert_eq!(far.layers[1].colour, Colour(0xFFAA_AAAA));
        assert_eq!(near.layers[1].colour, Colour(0xFFFF_1E1E));
    }

    #[test]
    fn the_needle_turns_by_the_angle_of_its_value_about_the_dial_centre() {
        let p = placement();
        let drawn = draw(&tachometer(), &feed(5.0), &mut 1.0, 0.0, &p);
        let needle = drawn.needle.expect("needle");
        let turn = needle.turn.expect("a needle turns");
        assert!((turn.radians - 90.0_f32.to_radians()).abs() < 1e-5, "5 of 10 thousand is 90 degrees");
        let centre = p.centre(330.0, 30.0, 60.0);
        assert_eq!(turn.about, centre);
        // At rest the needle turns at 78 % of its width (the default centre offset 0.22) and reaches left.
        assert!((needle.rect.x + needle.rect.w / 2.0 - (centre.0 + 0.22 * needle.rect.w)).abs() < 1e-3);
        assert!((needle.rect.y - centre.1).abs() < 1e-5);
        assert!((needle.rect.w / needle.rect.h - 2.0).abs() < 1e-3);
    }

    #[test]
    fn the_perfect_zone_colours_the_needle_only_when_the_flag_is_set() {
        let normal = draw(&tachometer(), &feed(3.0), &mut 1.0, 0.0, &placement());
        let zone = draw(&tachometer(), &Feed { perfect: true, ..feed(3.0) }, &mut 1.0, 0.0, &placement());
        assert_eq!(normal.needle.expect("needle").colour, Colour(0xFFFF_FFFF));
        assert_eq!(zone.needle.expect("needle").colour, Colour(0xFFFF_0000));
    }

    #[test]
    fn the_filled_arc_runs_from_the_scale_start_to_the_value() {
        let drawn = draw(&tachometer(), &feed(5.0), &mut 1.0, 0.0, &placement());
        let arc = drawn.layers[2].sector.expect("the arc is masked");
        assert_eq!((arc.start, arc.end), (-30.0, 90.0));
        assert_eq!((arc.inside, arc.outside), (255, 0x3C));
        assert_eq!(drawn.layers[2].colour, Colour(0xCCFF_FFFF));
    }

    #[test]
    fn the_red_zone_numbers_cover_the_red_line_to_the_end_plus_the_offset() {
        let drawn = draw(&tachometer(), &feed(5.0), &mut 1.0, 0.0, &placement());
        let zone = drawn.layers[3].sector.expect("the red zone is masked");
        // 7.5 of 10 thousand is 150 degrees, the end of the scale 210, and 15 more.
        assert_eq!((zone.start, zone.end), (150.0, 225.0));
        assert_eq!((zone.inside, zone.outside), (255, 0));
    }

    #[test]
    fn a_gauge_that_hides_at_its_maximum_fades_out_and_back() {
        let mut skin = tachometer();
        skin.hide_on_max = true;
        let p = placement();
        let mut fade = 0.0;
        let low = Feed { value: 2.0, ..feed(2.0) };
        draw(&skin, &low, &mut fade, 0.1, &p);
        assert!((fade - 0.3).abs() < 1e-5, "it fades in at 3 per second");
        let full = Feed { value: 10.0, ..feed(10.0) };
        draw(&skin, &full, &mut fade, 0.05, &p);
        assert!((fade - 0.15).abs() < 1e-5, "it fades out while the value is at the end");
        let drawn = draw(&skin, &low, &mut fade, 1.0, &p);
        assert_eq!(fade, 1.0);
        assert_eq!(drawn.layers[0].colour.alpha(), 0xFF);
    }

    #[test]
    fn a_picture_the_skin_leaves_out_is_not_drawn() {
        let mut bare = tachometer();
        bare.background.file.clear();
        bare.numbers.file.clear();
        let drawn = draw(&bare, &feed(3.0), &mut 1.0, 0.0, &placement());
        assert_eq!(drawn.layers.len(), 2);
    }
}
