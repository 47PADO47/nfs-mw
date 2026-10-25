//! The Compact mode's turn-cue chip: a small plate under the race strip (top centre) with an arrow for the next turn of
//! the GPS route and its distance (`HudState::route`, `next_turn`). No route, or a route too short to have a cue, shows
//! no chip.
//!
//! The distance follows `speed_units`: rounded to 10 m in metric; in imperial to 50 ft below 1000 ft and to 0.1 mi
//! above.

use blackbox_ui::{Component, LayoutCx, LineCap, LineJoin, Polyline, Rect, justify};
use glam::Vec2;

use crate::hud::{NavRoute, TurnDirection};
use crate::i18n::t;
use crate::settings::SpeedUnits;
use crate::settings::hud::HudSettings;
use crate::ui::theme::custom::layout::{Anchor, HudBox};
use crate::ui::theme::custom::plate::{Plate, PlateStyle};
use crate::ui::theme::custom::text::{Label, Sizing, size};
use crate::ui::theme::custom::tokens::{fade, keys};

use super::draw::colour;

/// The chip's size at scale 1, and where it sits: below the top-centre anchor, under the race strip.
pub const CHIP_SIZE: (f32, f32) = (92.0, 24.0);
pub const CHIP_OFFSET: (f32, f32) = (0.0, 40.0);
/// The glyph's stroke at scale 1.
const GLYPH_WIDTH: f32 = 2.0;
const FEET_PER_METRE: f32 = 3.280_84;
const METRES_PER_MILE: f32 = 1609.344;

/// The turn glyphs: a shaft and a head each, in a box from -1 to 1 (x right, y down).
type Glyph = (&'static [[f32; 2]], &'static [[f32; 2]]);
const STRAIGHT: Glyph = (&[[0.0, 1.0], [0.0, -1.0]], &[[-0.6, -0.4], [0.0, -1.0], [0.6, -0.4]]);
const RIGHT: Glyph = (&[[-0.4, 1.0], [-0.4, -0.2], [0.9, -0.2]], &[[0.4, -0.75], [0.9, -0.2], [0.4, 0.35]]);
const LEFT: Glyph = (&[[0.4, 1.0], [0.4, -0.2], [-0.9, -0.2]], &[[-0.4, -0.75], [-0.9, -0.2], [-0.4, 0.35]]);
const U_TURN: Glyph = (
    &[[0.5, 1.0], [0.5, -0.4], [0.3, -0.85], [-0.3, -0.85], [-0.5, -0.4], [-0.5, 0.6]],
    &[[-1.0, 0.1], [-0.5, 0.6], [0.0, 0.1]],
);

/// The parts of the chip for `route` at `alpha` (its fade), or `None` when there is nothing to cue or nothing shows.
pub fn chip(
    route: &NavRoute,
    units: SpeedUnits,
    (frame, settings): (&HudBox, &HudSettings),
    alpha: f32,
    cx: &LayoutCx<'_>,
) -> Option<Vec<Box<dyn Component>>> {
    let turn = route.next_turn?;
    let opacity = settings.opacity * alpha;
    if opacity <= 0.0 {
        return None;
    }
    let scale = frame.scale();
    let rect = frame.place(Anchor::TopCentre, CHIP_OFFSET, CHIP_SIZE);
    let plate = Plate::new(rect, PlateStyle::SURFACE.with_accent(keys::NAV).scaled(scale).with_opacity(opacity), cx);
    let mut parts: Vec<Box<dyn Component>> = vec![Box::new(plate)];
    let glyph_centre = Vec2::new(rect.min_x() + rect.h / 2.0, rect.y);
    let stroke = fade(colour(cx, keys::NAV), opacity);
    for line in glyph(turn.direction, glyph_centre, rect.h * 0.3, GLYPH_WIDTH * scale, stroke) {
        parts.push(Box::new(line));
    }
    let text_rect = Rect::new(rect.x + rect.h / 2.0, rect.y, rect.w - rect.h, rect.h);
    let label = Label::numeric(distance_text(turn.distance_m, units), size::NOTIFICATION_TITLE)
        .justify(justify::H_CENTRE | justify::V_CENTRE)
        .opacity(opacity);
    parts.push(Box::new(label.laid_out(text_rect, Sizing::of(frame, settings), cx)));
    Some(parts)
}

/// The turn's arrow round `centre`, `half` units from the centre to each side of its box, as strokes: a shaft and a
/// head, the head pointing the way to go.
pub fn glyph(direction: TurnDirection, centre: Vec2, half: f32, width: f32, colour: [u8; 4]) -> Vec<Polyline> {
    let (shaft, head) = match direction {
        TurnDirection::Straight => STRAIGHT,
        TurnDirection::Right => RIGHT,
        TurnDirection::Left => LEFT,
        TurnDirection::UTurn => U_TURN,
    };
    let place =
        |points: &[[f32; 2]]| -> Vec<Vec2> { points.iter().map(|p| centre + Vec2::new(p[0], p[1]) * half).collect() };
    let join = LineJoin::Mitre { limit: 3.0 };
    vec![
        Polyline::new(place(shaft), width, colour).with_join(join).with_cap(LineCap::Square),
        Polyline::new(place(head), width, colour).with_join(join).with_cap(LineCap::Square),
    ]
}

/// A distance in the player's unit, rounded as the chip shows it.
pub fn distance_text(metres: f32, units: SpeedUnits) -> String {
    let metres = metres.max(0.0);
    match units {
        SpeedUnits::Metric => t!("hud-map-distance-metres", n = ((metres / 10.0).round() * 10.0) as i64),
        SpeedUnits::Imperial => imperial(metres),
    }
}

/// Feet to the nearest 50 below 1000 ft, else miles to the nearest tenth.
fn imperial(metres: f32) -> String {
    let feet = metres * FEET_PER_METRE;
    if feet < 1000.0 {
        return t!("hud-map-distance-feet", n = ((feet / 50.0).round() * 50.0) as i64);
    }
    t!("hud-map-distance-miles", n = format!("{:.1}", metres / METRES_PER_MILE))
}
