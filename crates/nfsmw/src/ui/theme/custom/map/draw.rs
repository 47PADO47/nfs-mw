//! The overlay's shapes: the marks (`marks.rs`), the GPS and race routes and the player's arrow, each in a token colour from the
//! skin, faded by its alpha.
//!
//! Every shape is quads: a solid mark is a `Rounded` disc or square, a hollow one a ring of `Polyline` strokes. A mark
//! outside the ellipse is not drawn, and one in the soft edge fades with the roads, except the marks that stay in view
//! (the next checkpoint, the route's end): those are held just inside the edge.

use blackbox_ui::{Clip, Component, Image, LayoutCx, LineCap, LineJoin, Polyline, Polylines, Rect, Rounded, UiTree};
use glam::Vec2;

use super::icons;
use super::marks::{Mark, MarkKind};
use super::view::{MapView, pin};
use crate::hud::{NavRoute, RaceRoute, TICK_SECONDS};
use crate::ui::theme::custom::tokens::{fade, faded_skin, keys};

/// The route's width at scale 1, HUD units.
pub const ROUTE_WIDTH: f32 = 3.0;
/// The player's arrow, from its middle to its tip, at scale 1.
pub const ARROW_SIZE: f32 = 9.0;
/// The arrow's strokes and the dark edge round them, at scale 1.
const ARROW_STROKE: f32 = 2.5;
const ARROW_EDGE: f32 = 2.0;
/// A hollow mark's stroke at scale 1, and the points of its ring.
const RING_WIDTH: f32 = 1.5;
const RING_POINTS: usize = 16;

/// What every shape of one frame shares: the projection, the clip and its soft edge, the ellipse's half-axes and the
/// HUD scale.
#[derive(Clone, Copy, Debug)]
pub struct Stage<'a> {
    pub view: &'a MapView,
    pub clip: Clip,
    pub band: f32,
    pub radii: Vec2,
    pub scale: f32,
}

/// The game's icons for the marks and the arrow: the frame's HUD tree they are found in, and the clock the cop's light
/// bar runs on (held at one colour with motion `reduced`). Without it every mark is its shape.
#[derive(Clone, Copy, Debug)]
pub struct Art<'a> {
    pub tree: &'a UiTree,
    pub elapsed_s: f32,
    pub reduced: bool,
}

impl Art<'_> {
    /// The light bar's tick (`hud::TICK_SECONDS` long).
    fn tick(&self) -> u32 {
        if self.reduced {
            return 0;
        }
        (self.elapsed_s.max(0.0) / TICK_SECONDS) as u32
    }
}

/// The next checkpoint's icon is this much larger than the others, so it stands out among the checkpoints.
const NEXT_GAIN: f32 = 1.25;

/// The colour of the skin key `key`, opaque white when the skin lacks it (a skin without the Custom tokens).
pub fn colour(cx: &LayoutCx<'_>, key: &str) -> [u8; 4] {
    cx.skin.colour(key).unwrap_or([255, 255, 255, 255])
}

/// A mark's shape, sizes at scale 1 in HUD units.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Shape {
    Dot(f32),
    Square(f32),
    Ring(f32),
    /// A ring with a dot in it.
    Target(f32),
    Diamond(f32),
    Bar(f32),
}

/// The shape and the token colour of a kind of mark.
fn look(kind: MarkKind) -> (Shape, &'static str) {
    match kind {
        MarkKind::NextCheckpoint => (Shape::Target(9.0), keys::NAV),
        MarkKind::Checkpoint => (Shape::Dot(4.0), keys::NAV),
        MarkKind::SpeedTrap => (Shape::Dot(4.0), keys::CAUTION),
        MarkKind::Racer => (Shape::Dot(4.0), keys::TEXT_SECONDARY),
        MarkKind::Target => (Shape::Diamond(7.0), keys::NAV),
        MarkKind::Race => (Shape::Diamond(6.0), keys::ACCENT),
        MarkKind::SafeHouse => (Shape::Square(6.0), keys::SUCCESS),
        MarkKind::Shop => (Shape::Dot(5.0), keys::TEXT),
        MarkKind::Cop => (Shape::Dot(6.0), keys::DANGER),
        MarkKind::Helicopter => (Shape::Target(8.0), keys::DANGER),
        MarkKind::LastKnown => (Shape::Ring(8.0), keys::DANGER),
        MarkKind::Roadblock => (Shape::Bar(10.0), keys::CAUTION),
    }
}

/// Pushes the shapes of `marks` onto `out`, each at its own alpha times `alpha`, at depth `z`.
///
/// With `art` a mark is the game's own icon where the package has one for it, and its shape otherwise.
pub fn draw_marks(
    marks: &[Mark],
    stage: &Stage<'_>,
    (alpha, z): (f32, f32),
    art: Option<&Art<'_>>,
    cx: &LayoutCx<'_>,
    out: &mut Vec<Box<dyn Component>>,
) {
    for mark in marks {
        let projected = stage.view.to_screen(mark.at);
        let (at, edge) = match mark.kind.pinned() {
            true => (pin(projected, stage.view.centre, stage.radii), 1.0),
            false => (projected, stage.clip.fade(projected, stage.band)),
        };
        if !mark.kind.pinned() && !stage.clip.contains(projected) {
            continue;
        }
        let alpha = mark.alpha * alpha * edge;
        if alpha <= 0.0 {
            continue;
        }
        if let Some(art) = art
            && icon_part(mark, (at, stage), (alpha, z), (art, cx), out)
        {
            continue;
        }
        mark_parts(mark.kind, at, (alpha, z), stage.scale, cx, out);
    }
}

/// The clockwise turn from up, in radians, of the canvas direction `direction`.
pub fn clockwise_from_up(direction: Vec2) -> f32 {
    direction.x.atan2(-direction.y)
}

/// Pushes the game's icon for `mark` at `at`, and says whether the package had one. The checkpoints and the route's end
/// take the route's colour, a cop its light bar's, and the interiors the game's tint where the package's icon is plain
/// white; anything else keeps the icon's own colour.
fn icon_part(
    mark: &Mark,
    (at, stage): (Vec2, &Stage<'_>),
    (alpha, z): (f32, f32),
    (art, cx): (&Art<'_>, &LayoutCx<'_>),
    out: &mut Vec<Box<dyn Component>>,
) -> bool {
    let Some(name) = icons::object(mark.kind, mark.variant) else { return false };
    let Some(icon) = icons::find(art.tree, &name) else { return false };
    let package = [icon.colour[0], icon.colour[1], icon.colour[2], 255];
    let colour = match mark.kind {
        MarkKind::NextCheckpoint | MarkKind::Checkpoint | MarkKind::Target => colour(cx, keys::NAV),
        MarkKind::Cop => icons::tint(mark.kind, mark.variant, art.tick()).unwrap_or(package),
        _ if package[..3] == [255; 3] => icons::tint(mark.kind, mark.variant, art.tick()).unwrap_or(package),
        _ => package,
    };
    let gain = match mark.kind {
        MarkKind::NextCheckpoint => NEXT_GAIN,
        _ => 1.0,
    };
    let turn = mark.heading.map_or(0.0, |heading| clockwise_from_up(stage.view.direction(heading)));
    let size = icon.size * stage.scale * gain;
    let mut image = Image::raw(icon.texture).with_tint(fade(colour, alpha)).with_z(z).with_rotation(turn);
    image.layout(cx, Rect::new(at.x, at.y, size.x, size.y));
    out.push(Box::new(image));
    true
}

/// Pushes the shape of a mark of `kind` at `at` (canvas units) onto `out`, at `alpha` and depth `z`, sized for the
/// scale `scale`: what [`draw_marks`] draws for one mark, and what a legend shows beside the kind's name.
pub fn mark_parts(
    kind: MarkKind,
    at: Vec2,
    (alpha, z): (f32, f32),
    scale: f32,
    cx: &LayoutCx<'_>,
    out: &mut Vec<Box<dyn Component>>,
) {
    let (shape, key) = look(kind);
    shape_parts(shape, key, at, (alpha, z), scale, cx, out);
}

/// The parts of one shape at `at`.
fn shape_parts(
    shape: Shape,
    key: &'static str,
    at: Vec2,
    (alpha, z): (f32, f32),
    scale: f32,
    cx: &LayoutCx<'_>,
    out: &mut Vec<Box<dyn Component>>,
) {
    let stroke = fade(colour(cx, key), alpha);
    match shape {
        Shape::Dot(size) => out.push(Box::new(disc(key, at, size * scale, size * scale / 2.0, (alpha, z), cx))),
        Shape::Square(size) => out.push(Box::new(disc(key, at, size * scale, scale, (alpha, z), cx))),
        Shape::Ring(size) => out.push(Box::new(ring(at, size * scale / 2.0, RING_WIDTH * scale, stroke, z))),
        Shape::Target(size) => {
            out.push(Box::new(ring(at, size * scale / 2.0, RING_WIDTH * scale, stroke, z)));
            out.push(Box::new(disc(key, at, size * scale / 3.0, size * scale / 6.0, (alpha, z), cx)));
        }
        Shape::Diamond(size) => {
            let r = size * scale / 2.0;
            let corners = [Vec2::NEG_Y, Vec2::X, Vec2::Y, Vec2::NEG_X, Vec2::NEG_Y, Vec2::X];
            let points = corners.iter().map(|corner| at + *corner * r).collect();
            let line = Polyline::new(points, 2.0 * scale, stroke).with_join(LineJoin::Mitre { limit: 3.0 });
            out.push(Box::new(line.with_z(z)));
        }
        Shape::Bar(size) => {
            let half = Vec2::new(size * scale / 2.0, 0.0);
            let line = Polyline::new(vec![at - half, at + half], 3.0 * scale, stroke).with_cap(LineCap::Square);
            out.push(Box::new(line.with_z(z)));
        }
    }
}

/// A filled rounded square `size` across with corners of `radius` round `at`, in the skin colour `key` at `alpha`: a
/// disc when the radius is half the size.
fn disc(key: &'static str, at: Vec2, size: f32, radius: f32, (alpha, z): (f32, f32), cx: &LayoutCx<'_>) -> Rounded {
    let skin = faded_skin(cx.skin, &[key], &[], alpha);
    Rounded::at_depth(key, Rect::new(at.x, at.y, size, size), radius, z, &LayoutCx::new(&skin, cx.time_s))
}

/// A ring of `radius` round `at`, stroked `width` wide in `colour`, at depth `z`.
pub fn ring(at: Vec2, radius: f32, width: f32, colour: [u8; 4], z: f32) -> Polyline {
    let points = (0..=RING_POINTS + 1)
        .map(|i| {
            let angle = i as f32 / RING_POINTS as f32 * std::f32::consts::TAU;
            at + Vec2::new(angle.cos(), angle.sin()) * radius
        })
        .collect();
    Polyline::new(points, width, colour).with_join(LineJoin::Bevel).with_z(z)
}

/// The GPS route split where the car is: the stretch behind it (start to the car) and ahead of it (the car to the
/// end), both in render space. A route of fewer than two points is all ahead.
pub fn split_route(path: &[[f32; 2]], car: [f32; 2]) -> (Vec<[f32; 2]>, Vec<[f32; 2]>) {
    if path.len() < 2 {
        return (Vec::new(), path.to_vec());
    }
    let (piece, at) = nearest(path, car);
    let mut behind = path[..=piece].to_vec();
    behind.push(at);
    let mut ahead = vec![at];
    ahead.extend_from_slice(&path[piece + 1..]);
    (behind, ahead)
}

/// The piece of `path` nearest `point` (the index of its first point) and the nearest point on it.
fn nearest(path: &[[f32; 2]], point: [f32; 2]) -> (usize, [f32; 2]) {
    let mut best = (0, path[0], f32::INFINITY);
    for (piece, pair) in path.windows(2).enumerate() {
        let (a, b) = (pair[0], pair[1]);
        let d = [b[0] - a[0], b[1] - a[1]];
        let span = d[0] * d[0] + d[1] * d[1];
        let t = match span > 0.0 {
            true => (((point[0] - a[0]) * d[0] + (point[1] - a[1]) * d[1]) / span).clamp(0.0, 1.0),
            false => 0.0,
        };
        let on = [a[0] + d[0] * t, a[1] + d[1] * t];
        let gap = (point[0] - on[0]).hypot(point[1] - on[1]);
        if gap < best.2 {
            best = (piece, on, gap);
        }
    }
    (best.0, best.1)
}

/// Pushes the route's strokes onto `lines`: the stretch ahead in `ahead`, the stretch behind in `behind` (skipped when
/// transparent), both [`ROUTE_WIDTH`] wide at depth `z`.
pub fn route_lines(
    route: &NavRoute,
    car: [f32; 2],
    stage: &Stage<'_>,
    (ahead, behind): ([u8; 4], [u8; 4]),
    z: f32,
    lines: &mut Polylines,
) {
    let (driven, to_go) = split_route(&route.path, car);
    push_route([(driven, behind), (to_go, ahead)], stage, z, lines);
}

/// Pushes the race route's strokes onto `lines`: split `along` metres into the lap when the player's place on it is
/// known, the stretch ahead in `ahead` and the stretch driven in `driven`; the whole lap in `ahead` otherwise. A lap
/// race's line is already closed (`hud/map_roads/race_route.rs`).
pub fn race_route_lines(
    route: &RaceRoute,
    along: Option<f32>,
    stage: &Stage<'_>,
    (ahead, driven): ([u8; 4], [u8; 4]),
    z: f32,
    lines: &mut Polylines,
) {
    let (behind, to_go) = match along {
        Some(along) => route.split(along),
        None => (Vec::new(), route.points().to_vec()),
    };
    push_route([(behind, driven), (to_go, ahead)], stage, z, lines);
}

/// Pushes route stretches (render-space points and a colour) as [`ROUTE_WIDTH`] strokes; a transparent or one-point
/// stretch is skipped.
fn push_route(stretches: [(Vec<[f32; 2]>, [u8; 4]); 2], stage: &Stage<'_>, z: f32, lines: &mut Polylines) {
    let width = ROUTE_WIDTH * stage.scale;
    for (points, colour) in stretches {
        if colour[3] == 0 || points.len() < 2 {
            continue;
        }
        let points = points.iter().map(|p| stage.view.to_screen(*p)).collect();
        let line = Polyline::new(points, width, colour).with_join(LineJoin::Mitre { limit: 2.0 });
        lines.push(line.with_clip(stage.clip, stage.band).with_z(z));
    }
}

/// The player's arrow at `at` pointing along `direction`, as the game's own arrow (`PLAYERCARINDICATOR`) in the first
/// tone, or as [`arrow`]'s strokes when the package has none.
pub fn arrow_parts(
    art: Option<&Art<'_>>,
    (at, direction): (Vec2, Vec2),
    scale: f32,
    tones: ([u8; 4], [u8; 4]),
    (z, cx): (f32, &LayoutCx<'_>),
) -> Vec<Box<dyn Component>> {
    let icon = art.and_then(|art| icons::find(art.tree, icons::PLAYER_ARROW));
    let Some(icon) = icon else {
        let strokes = arrow(at, direction, scale, tones, z);
        return strokes.into_iter().map(|line| Box::new(line) as Box<dyn Component>).collect();
    };
    let size = icon.size * scale;
    let mut image = Image::raw(icon.texture).with_tint(tones.0).with_z(z).with_rotation(clockwise_from_up(direction));
    image.layout(cx, Rect::new(at.x, at.y, size.x, size.y));
    vec![Box::new(image)]
}

/// The player's arrow at `at` pointing along `direction` (a canvas unit vector): a pointed chevron with a spine, in
/// `colour`, over a dark edge in `edge` just behind it.
pub fn arrow(at: Vec2, direction: Vec2, scale: f32, (colour, edge): ([u8; 4], [u8; 4]), z: f32) -> Vec<Polyline> {
    let size = ARROW_SIZE * scale;
    let across = direction.perp();
    let tip = at + direction * size;
    let back = at - direction * (0.6 * size);
    let (left, right) = (back + across * (0.65 * size), back - across * (0.65 * size));
    let notch = at - direction * (0.25 * size);
    let outline = [tip, right, notch, left, tip, right];
    let spine = [notch, tip];
    let join = LineJoin::Mitre { limit: 4.0 };
    let stroke = |points: &[Vec2], width: f32, colour: [u8; 4], z: f32| {
        Polyline::new(points.to_vec(), width * scale, colour).with_join(join).with_z(z)
    };
    vec![
        stroke(&outline[..], ARROW_STROKE + ARROW_EDGE, edge, z + 0.5),
        stroke(&spine[..], ARROW_STROKE + ARROW_EDGE, edge, z + 0.5),
        stroke(&outline[..], ARROW_STROKE, colour, z),
        stroke(&spine[..], ARROW_STROKE, colour, z),
    ]
}
