//! The world markers' shapes and labels (`docs/specs/custom-hud/world-markers.md` sections 2 to 4): a diamond for a
//! gate, a ring for a place, a warning triangle for a breaker, a chevron for an edge arrow or a recovery, the expanding
//! ring of a checkpoint pass, and a label on a small plate.
//!
//! Sizes are in HUD units at scale 1, multiplied by the HUD scale and the marker's own size, so a marker is the same
//! apparent size at every resolution. Colours are `custom.*` skin keys faded by `[custom_hud] opacity` and the marker's
//! alpha. With high contrast on, every stroke has a darker outline under it and the label plate its outline.

use blackbox_ui::{
    Component, DEFAULT_Z, DrawList, LayoutCx, LineCap, LineJoin, Polyline, Polylines, Rect, Ring, Text, justify,
};
use glam::Vec2;

use crate::hud::NavMarkerKind;
use crate::i18n::t;
use crate::settings::SpeedUnits;

use super::super::plate::{Plate, PlateStyle, RULE_WIDTH};
use super::super::text::{Label, Sizing, size};
use super::super::tokens::{fade, faded_skin, keys};
use super::rules::{Candidate, Kind, Name};

/// The depth of the markers: behind every other HUD element (a lower `z` draws on top).
pub const MARK_Z: f32 = DEFAULT_Z + 10.0;
const PLATE_Z: f32 = MARK_Z + 0.2;
const OUTLINE_Z: f32 = MARK_Z + 0.1;
const TEXT_Z: f32 = MARK_Z - 0.1;
/// A gate's diamond, half its height, and the width of every stroke.
pub const MARK_HALF: f32 = 7.0;
pub const STROKE: f32 = 2.0;
/// An edge arrow's chevron, half its length.
pub const ARROW_HALF: f32 = 9.0;
/// A race start's ring and a discovered place's, radius.
const ENTRANCE_RADIUS: f32 = 6.0;
const DISCOVERED_RADIUS: f32 = 4.0;
/// The space between a mark and its label, and round the label's text on its plate.
pub const LABEL_GAP: f32 = 4.0;
const LABEL_PAD: f32 = 3.0;
/// The label's text height.
pub const LABEL_SIZE: f32 = size::NOTIFICATION_LINE;
/// About how wide a character of the label is, as a share of its height: enough for the plate to hold the text.
const CHAR_WIDTH: f32 = 0.55;
/// The checkpoint tick's ring grows from the diamond to this many times its size.
const TICK_GROWTH: f32 = 2.2;

/// The skin key a marker of `kind` draws in.
pub fn colour_key(kind: Kind) -> &'static str {
    match kind {
        Kind::ActiveCheckpoint => keys::NAV,
        Kind::Recovery | Kind::Breaker => keys::CAUTION,
        Kind::Entrance => keys::ACCENT,
        Kind::Discovered => keys::TEXT_SECONDARY,
    }
}

/// The label of `candidate` at `sizing`: the gate's number and distance, a place's name, or for an edge arrow the
/// distance alone. `None` for a marker without one.
pub fn label_of(candidate: &Candidate, arrow: bool, units: SpeedUnits, sizing: Sizing) -> Option<MarkerLabel> {
    let distance = distance_text(candidate.distance, units);
    if arrow {
        return Some(MarkerLabel::new(distance, true, sizing));
    }
    match &candidate.name {
        Name::None => None,
        Name::Gate { gate, gates } => {
            let text = t!("hud-marker-gate", gate = *gate, gates = *gates, distance = distance);
            Some(MarkerLabel::new(text, true, sizing))
        }
        Name::Place(kind, name) => Some(MarkerLabel::new(place_text(*kind, name.as_deref()), false, sizing)),
    }
}

/// A map place's name: a race by its career event id, the others by their kind.
fn place_text(kind: NavMarkerKind, name: Option<&str>) -> String {
    match (kind, name) {
        (NavMarkerKind::Race(_), Some(event)) => t!("hud-marker-race", event = event),
        (NavMarkerKind::Race(_), None) => t!("hud-entry-race"),
        (NavMarkerKind::SafeHouse, _) => t!("hud-entry-safe-house"),
        (NavMarkerKind::AutoShop, _) => t!("hud-entry-auto-shop"),
        (NavMarkerKind::CarLot, _) => t!("hud-entry-car-lot"),
    }
}

/// A distance in the player's units: whole metres (in tens past 100 m) or kilometres to a tenth; whole feet (in tens past
/// 100 ft) or miles to a tenth past 1000 ft.
pub fn distance_text(metres: f32, units: SpeedUnits) -> String {
    let metres = metres.max(0.0);
    match units {
        SpeedUnits::Metric if metres < 1000.0 => t!("hud-marker-metres", m = stepped(metres)),
        SpeedUnits::Metric => t!("hud-marker-kilometres", km = format!("{:.1}", metres / 1000.0)),
        SpeedUnits::Imperial if metres * FEET_PER_METRE < 1000.0 => {
            t!("hud-marker-feet", ft = stepped(metres * FEET_PER_METRE))
        }
        SpeedUnits::Imperial => t!("hud-marker-miles", mi = format!("{:.1}", metres / METRES_PER_MILE)),
    }
}

const FEET_PER_METRE: f32 = 3.280_84;
const METRES_PER_MILE: f32 = 1609.344;

/// `value` as a whole number, rounded to tens from 100 up, so a label does not flicker through every unit.
fn stepped(value: f32) -> u32 {
    let step = match value < 100.0 {
        true => 1.0,
        false => 10.0,
    };
    ((value / step).round() * step) as u32
}

/// A marker's label: its text, whether it is a number (the numeric face) or a name (the label face), and about how
/// much room it takes on its plate, canvas units.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkerLabel {
    pub text: String,
    pub numeric: bool,
    pub extent: Vec2,
}

impl MarkerLabel {
    /// The label `text` at `sizing`. Its plate is sized from the character count, wide enough for the text.
    pub fn new(text: String, numeric: bool, sizing: Sizing) -> Self {
        let height = sizing.units(LABEL_SIZE);
        let pad = LABEL_PAD * sizing.scale;
        let extent = Vec2::new(text.chars().count() as f32 * height * CHAR_WIDTH + 2.0 * pad, height + 2.0 * pad);
        Self { text, numeric, extent }
    }
}

/// How the frame's markers paint: `[custom_hud] opacity`, the HUD scale and the text sizing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paint {
    pub opacity: f32,
    pub scale: f32,
    pub sizing: Sizing,
}

/// The laid-out markers of one frame, in draw order: the label plates, the outlines, the strokes and rings, the text.
#[derive(Default)]
pub struct Parts {
    plates: Vec<Plate>,
    outlines: Polylines,
    lines: Polylines,
    rings: Vec<Ring>,
    texts: Vec<Text>,
}

impl Parts {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether nothing was laid out.
    pub fn is_empty(&self) -> bool {
        self.plates.is_empty() && self.lines.is_empty() && self.rings.is_empty() && self.texts.is_empty()
    }

    /// The mark of a marker of `kind` on screen at `at`, `size` times its size, at `alpha`. A recovery is the gate's
    /// diamond with a chevron above it pointing down at it.
    pub fn mark(&mut self, kind: Kind, at: Vec2, size: f32, alpha: f32, paint: Paint, cx: &LayoutCx<'_>) {
        let s = paint.scale * size;
        let key = colour_key(kind);
        let opacity = paint.opacity * alpha;
        match kind {
            Kind::ActiveCheckpoint => self.stroke(diamond(at, MARK_HALF * s), key, STROKE * s, opacity, cx),
            Kind::Recovery => {
                self.stroke(diamond(at, MARK_HALF * s), key, STROKE * s, opacity, cx);
                let above = at - Vec2::Y * (MARK_HALF + LABEL_GAP + ARROW_HALF) * s;
                self.stroke(chevron(above, Vec2::Y, ARROW_HALF * s), key, STROKE * s, opacity, cx);
            }
            Kind::Breaker => self.stroke(triangle(at, MARK_HALF * s), key, STROKE * s, opacity, cx),
            Kind::Entrance => self.ring(key, at, ENTRANCE_RADIUS * s, STROKE * s, opacity, cx),
            Kind::Discovered => self.ring(key, at, DISCOVERED_RADIUS * s, STROKE * 0.75 * s, opacity, cx),
        }
    }

    /// An edge arrow of a marker of `kind` at `at`, pointing along `direction` (a unit vector, y down).
    pub fn arrow(&mut self, kind: Kind, at: Vec2, direction: Vec2, alpha: f32, paint: Paint, cx: &LayoutCx<'_>) {
        let s = paint.scale;
        let points = chevron(at, direction, ARROW_HALF * s);
        self.stroke(points, colour_key(kind), STROKE * 1.25 * s, paint.opacity * alpha, cx);
    }

    /// The checkpoint pass's tick at `at`, `progress` (0 to 1) through it: a ring that grows from the diamond and fades
    /// out, or without motion a ring held still at full alpha.
    pub fn tick(&mut self, at: Vec2, progress: f32, moves: bool, paint: Paint, cx: &LayoutCx<'_>) {
        let start = MARK_HALF * paint.scale;
        let (radius, alpha) = match moves {
            true => (start * (1.0 + (TICK_GROWTH - 1.0) * progress), 1.0 - progress),
            false => (start * 1.5, 1.0),
        };
        self.ring(keys::SUCCESS, at, radius, STROKE * paint.scale, paint.opacity * alpha, cx);
    }

    /// `label` centred on `at` on its plate, at `alpha`.
    pub fn label(&mut self, label: &MarkerLabel, at: Vec2, alpha: f32, paint: Paint, cx: &LayoutCx<'_>) {
        let opacity = paint.opacity * alpha;
        let rect = Rect::new(at.x, at.y, label.extent.x, label.extent.y);
        let style = PlateStyle::SURFACE.scaled(paint.scale).with_opacity(opacity).with_z(PLATE_Z);
        self.plates.push(Plate::new(rect, style, cx));
        let text = match label.numeric {
            true => Label::numeric(label.text.as_str(), LABEL_SIZE),
            false => Label::label(label.text.as_str(), LABEL_SIZE),
        };
        let text = text.justify(justify::H_CENTRE | justify::V_CENTRE).opacity(opacity).z(TEXT_Z);
        self.texts.push(text.laid_out(rect, paint.sizing, cx));
    }

    /// A stroke through `points` in the skin colour `key` at `opacity`, with its high-contrast outline under it.
    fn stroke(&mut self, points: Vec<Vec2>, key: &str, width: f32, opacity: f32, cx: &LayoutCx<'_>) {
        let Some(colour) = cx.skin.colour(key) else { return };
        let join = LineJoin::Mitre { limit: 3.0 };
        if let Some(outline) = cx.skin.colour(keys::OUTLINE) {
            let under = Polyline::new(points.clone(), width + 2.0 * RULE_WIDTH, fade(outline, opacity));
            self.outlines.push(under.with_join(join).with_cap(LineCap::Square).with_z(OUTLINE_Z));
        }
        let line = Polyline::new(points, width, fade(colour, opacity));
        self.lines.push(line.with_join(join).with_cap(LineCap::Square).with_z(MARK_Z));
    }

    /// A full ring in the skin colour `key` at `opacity`, with its high-contrast outline under it.
    fn ring(&mut self, key: &'static str, at: Vec2, radius: f32, width: f32, opacity: f32, cx: &LayoutCx<'_>) {
        let skin = faded_skin(cx.skin, &[key, keys::OUTLINE], &[], opacity);
        let local = LayoutCx::new(&skin, cx.time_s);
        let centre = (at.x, at.y);
        if skin.colour(keys::OUTLINE).is_some() {
            let thick = width + 2.0 * RULE_WIDTH;
            self.rings.push(Ring::at_depth(keys::OUTLINE, centre, radius, thick, (0.0, 360.0), OUTLINE_Z, &local));
        }
        self.rings.push(Ring::at_depth(key, centre, radius, width, (0.0, 360.0), MARK_Z, &local));
    }
}

impl Component for Parts {
    /// The parts are laid out as they are added, so there is nothing to place here.
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        for plate in &self.plates {
            plate.build(out);
        }
        self.outlines.build(out);
        self.lines.build(out);
        for ring in &self.rings {
            ring.build(out);
        }
        for text in &self.texts {
            text.build(out);
        }
    }
}

/// A closed diamond round `at`, `half` from its centre to each point.
fn diamond(at: Vec2, half: f32) -> Vec<Vec2> {
    let (x, y) = (Vec2::X * half, Vec2::Y * half);
    vec![at - y, at + x, at + y, at - x, at - y]
}

/// A closed warning triangle round `at`, point up, `half` from its centre to its top.
fn triangle(at: Vec2, half: f32) -> Vec<Vec2> {
    let top = at - Vec2::Y * half;
    let base = at.y + half * 0.7;
    vec![top, Vec2::new(at.x + half, base), Vec2::new(at.x - half, base), top]
}

/// A chevron at `at` pointing along `direction` (a unit vector), `half` from its centre to its tip.
pub fn chevron(at: Vec2, direction: Vec2, half: f32) -> Vec<Vec2> {
    let side = direction.perp() * half * 0.8;
    let back = at - direction * half * 0.5;
    vec![back + side, at + direction * half, back - side]
}
