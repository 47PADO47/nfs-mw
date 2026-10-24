//! The notices' and chips' icons: small line pictograms stroked with `Polylines`, so they need no texture and take the
//! token colour of their plate. Every notice carries one beside its text, so that no state is told by colour alone
//! (`docs/specs/custom-hud/visual-identity.md` section 1, rule 4). The ids are the catalogue's (`hud/notify/catalog.rs`);
//! an id with no pictogram of its own draws a ringed dot.
//!
//! A pictogram is drawn in a square of side 2 centred on the origin, x right and y down, scaled into the icon's
//! rectangle.

use std::f32::consts::PI;

use blackbox_ui::{LineCap, LineJoin, Polyline, Polylines, Rect};
use glam::Vec2;

/// The stroke width as a fraction of the icon's side.
const STROKE: f32 = 0.12;
/// The segments of a full circle.
const CIRCLE_SEGMENTS: usize = 16;

/// One stroke of a pictogram.
enum Stroke {
    /// A line through the points.
    Line(&'static [(f32, f32)]),
    /// An arc round `centre` of `radius`, from `from` to `to` degrees (0 to the right, 90 down).
    Arc { centre: (f32, f32), radius: f32, from: f32, to: f32 },
}

/// A full circle.
const fn circle(centre: (f32, f32), radius: f32) -> Stroke {
    Stroke::Arc { centre, radius, from: 0.0, to: 360.0 }
}

const WARNING: &[Stroke] = &[
    Stroke::Line(&[(0.0, -0.9), (0.95, 0.8), (-0.95, 0.8), (0.0, -0.9)]),
    Stroke::Line(&[(0.0, -0.3), (0.0, 0.25)]),
    Stroke::Line(&[(0.0, 0.45), (0.0, 0.6)]),
];
const ROADBLOCK: &[Stroke] = &[
    Stroke::Line(&[(-0.95, -0.5), (0.95, -0.5), (0.95, 0.1), (-0.95, 0.1), (-0.95, -0.5)]),
    Stroke::Line(&[(-0.5, 0.1), (-0.1, -0.5)]),
    Stroke::Line(&[(0.1, 0.1), (0.5, -0.5)]),
    Stroke::Line(&[(-0.6, 0.1), (-0.6, 0.9)]),
    Stroke::Line(&[(0.6, 0.1), (0.6, 0.9)]),
];
const BUSTED: &[Stroke] = &[Stroke::Line(&[(-0.8, -0.8), (0.8, 0.8)]), Stroke::Line(&[(0.8, -0.8), (-0.8, 0.8)])];
const HEAT: &[Stroke] =
    &[Stroke::Line(&[(-0.8, 0.1), (0.0, -0.6), (0.8, 0.1)]), Stroke::Line(&[(-0.8, 0.8), (0.0, 0.1), (0.8, 0.8)])];
const HELICOPTER: &[Stroke] = &[
    Stroke::Line(&[(-0.95, -0.7), (0.95, -0.7)]),
    Stroke::Line(&[(0.0, -0.7), (0.0, -0.3)]),
    circle((0.0, 0.1), 0.4),
    Stroke::Line(&[(0.4, 0.1), (0.95, 0.1)]),
];
const EYE_OFF: &[Stroke] = &[
    Stroke::Line(&[
        (-0.9, 0.0),
        (-0.45, -0.45),
        (0.0, -0.55),
        (0.45, -0.45),
        (0.9, 0.0),
        (0.45, 0.45),
        (0.0, 0.55),
        (-0.45, 0.45),
        (-0.9, 0.0),
    ]),
    circle((0.0, 0.0), 0.2),
    Stroke::Line(&[(-0.8, -0.8), (0.8, 0.8)]),
];
const TIMER: &[Stroke] = &[
    circle((0.0, 0.1), 0.8),
    Stroke::Line(&[(0.0, 0.1), (0.0, -0.4)]),
    Stroke::Line(&[(0.0, 0.1), (0.35, 0.3)]),
    Stroke::Line(&[(-0.25, -0.9), (0.25, -0.9)]),
];
const ESCAPED: &[Stroke] =
    &[Stroke::Line(&[(-0.9, 0.0), (0.8, 0.0)]), Stroke::Line(&[(0.3, -0.5), (0.85, 0.0), (0.3, 0.5)])];
const CHECK: &[Stroke] = &[Stroke::Line(&[(-0.8, 0.0), (-0.25, 0.6), (0.85, -0.6)])];
const WRONG_WAY: &[Stroke] = &[
    Stroke::Line(&[(0.5, 0.9), (0.5, -0.3), (0.3, -0.7), (0.0, -0.85), (-0.3, -0.7), (-0.5, -0.3), (-0.5, 0.6)]),
    Stroke::Line(&[(-0.85, 0.25), (-0.5, 0.7), (-0.15, 0.25)]),
];
const FLAG: &[Stroke] = &[
    Stroke::Line(&[(-0.7, 0.95), (-0.7, -0.9)]),
    Stroke::Line(&[(-0.7, -0.9), (0.8, -0.9), (0.8, -0.1), (-0.7, -0.1)]),
    Stroke::Line(&[(0.05, -0.9), (0.05, -0.1)]),
    Stroke::Line(&[(-0.7, -0.5), (0.8, -0.5)]),
];
const UP: &[Stroke] =
    &[Stroke::Line(&[(0.0, 0.9), (0.0, -0.8)]), Stroke::Line(&[(-0.55, -0.25), (0.0, -0.85), (0.55, -0.25)])];
const NEAR_MISS: &[Stroke] = &[Stroke::Line(&[(-0.9, 0.5), (0.2, -0.6)]), Stroke::Line(&[(-0.2, 0.9), (0.9, -0.2)])];
const GAUGE: &[Stroke] = &[
    Stroke::Arc { centre: (0.0, 0.2), radius: 0.85, from: 150.0, to: 390.0 },
    Stroke::Line(&[(0.0, 0.2), (0.5, -0.35)]),
];
const DIAMOND: &[Stroke] = &[
    Stroke::Line(&[(0.0, -0.9), (0.7, 0.0), (0.0, 0.9), (-0.7, 0.0), (0.0, -0.9)]),
    Stroke::Line(&[(-0.7, 0.0), (0.7, 0.0)]),
];
const TYRE: &[Stroke] = &[circle((0.0, 0.0), 0.85), circle((0.0, 0.0), 0.35)];
const BOLT: &[Stroke] =
    &[Stroke::Line(&[(0.2, -0.95), (-0.5, 0.1), (0.05, 0.1), (-0.2, 0.95), (0.55, -0.15), (0.0, -0.15), (0.2, -0.95)])];
const DOT: &[Stroke] = &[circle((0.0, 0.0), 0.8), circle((0.0, 0.0), 0.15)];

/// The pictogram of the catalogue's icon `id`; `None` for an id without one.
fn strokes(id: &str) -> Option<&'static [Stroke]> {
    Some(match id {
        "warning" | "damage" => WARNING,
        "roadblock" => ROADBLOCK,
        "busted" => BUSTED,
        "heat" => HEAT,
        "helicopter" => HELICOPTER,
        "eye-off" => EYE_OFF,
        "timer" => TIMER,
        "escaped" => ESCAPED,
        "checkpoint" => CHECK,
        "wrong-way" => WRONG_WAY,
        "lap" => FLAG,
        "position" => UP,
        "near-miss" => NEAR_MISS,
        "speed-trap" => GAUGE,
        "bounty" => DIAMOND,
        "tyre" => TYRE,
        "nitrous" => BOLT,
        _ => return None,
    })
}

/// Whether `id` has a pictogram of its own (not the fallback dot).
#[cfg(test)]
pub fn known(id: &str) -> bool {
    strokes(id).is_some()
}

/// The icon `id` stroked in `colour` (straight RGBA) inside `rect` (canvas units), at depth `z`.
pub fn icon(id: &str, rect: Rect, colour: [u8; 4], z: f32) -> Polylines {
    let half = rect.w.min(rect.h) / 2.0;
    let to_canvas = |(x, y): (f32, f32)| Vec2::new(rect.x + x * half, rect.y + y * half);
    let width = STROKE * half * 2.0;
    let mut lines = Polylines::with_capacity(4);
    for stroke in strokes(id).unwrap_or(DOT) {
        let points = match stroke {
            Stroke::Line(points) => points.iter().copied().map(to_canvas).collect(),
            Stroke::Arc { centre, radius, from, to } => arc(*centre, *radius, *from, *to).map(to_canvas).collect(),
        };
        let line = Polyline::new(points, width, colour)
            .with_join(LineJoin::Mitre { limit: 2.0 })
            .with_cap(LineCap::Square)
            .with_z(z);
        lines.push(line);
    }
    lines
}

/// The points of an arc in the pictogram's square: [`CIRCLE_SEGMENTS`] per full turn, at least two.
fn arc(centre: (f32, f32), radius: f32, from: f32, to: f32) -> impl Iterator<Item = (f32, f32)> {
    let sweep = to - from;
    let segments = ((sweep.abs() / 360.0 * CIRCLE_SEGMENTS as f32).ceil() as usize).max(2);
    (0..=segments).map(move |k| {
        let angle = (from + sweep * k as f32 / segments as f32) * PI / 180.0;
        (centre.0 + radius * angle.cos(), centre.1 + radius * angle.sin())
    })
}
