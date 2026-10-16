//! `interiors/<id>.toml`: a place a mod adds behind an entry zone (docs/specs/mod-world-content.md, section 3). The entry
//! zone and the exit zone are shapes in render space; the place itself is a scenery pack of the same mod.
//!
//! This module checks the schema and the values. Whether the pack exists and is accepted, and whether the spawn has
//! ground under it, are for the game to decide.

use serde::Deserialize;

use super::car::checked_text;
use super::{ContentError, is_id};

/// The largest zone radius, metres. A zone wider than this is almost certainly a unit mistake.
const MAX_RADIUS: f32 = 200.0;

/// A checked `interiors/<id>.toml`.
#[derive(Debug, Clone, PartialEq)]
pub struct InteriorManifest {
    /// The interior key: the file name without `.toml`.
    pub key: String,
    /// The name the entry card shows, trimmed.
    pub label: String,
    /// The entry card's line. `None` keeps the game's own wording.
    pub prompt: Option<String>,
    /// Where the entry is: driving into it offers the prompt.
    pub entry: Shape,
    /// The scenery pack of the same mod that is the place: its file name without `.toml`.
    pub pack: String,
    pub spawn: Pose,
    pub exit: Exit,
    /// The clock inside. `None` keeps the clock as it is.
    pub time: Option<TimeOverride>,
    /// What stops while the player is inside.
    pub suspend: Suspend,
}

/// A zone in render space. A sphere is a centre and a radius; a box is a centre, half sizes and a turn about up, in
/// degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    Sphere { centre: [f32; 3], radius: f32 },
    Box { centre: [f32; 3], half: [f32; 3], yaw: f32 },
}

impl Shape {
    /// Whether `point` (render space) is inside the zone. A sphere and a box both hold their surface.
    pub fn contains(&self, point: [f32; 3]) -> bool {
        match *self {
            Shape::Sphere { centre, radius } => {
                let d = [0, 1, 2].map(|axis| point[axis] - centre[axis]);
                d[0] * d[0] + d[1] * d[1] + d[2] * d[2] <= radius * radius
            }
            Shape::Box { centre, half, yaw } => {
                let d = [0, 1, 2].map(|axis| point[axis] - centre[axis]);
                // The point in the box's own axes: turned back by its yaw.
                let (sin, cos) = yaw.to_radians().sin_cos();
                let local = [cos * d[0] + sin * d[1], -sin * d[0] + cos * d[1], d[2]];
                local.iter().zip(half).all(|(v, h)| v.abs() <= h)
            }
        }
    }
}

/// The exit zone and its card.
#[derive(Debug, Clone, PartialEq)]
pub struct Exit {
    pub shape: Shape,
    /// The exit card's line. `None` keeps the game's own wording.
    pub prompt: Option<String>,
}

/// A place and a heading, render space: the car's spawn inside the interior.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub pos: [f32; 3],
    /// Degrees from +x, anticlockwise about up.
    pub heading: f32,
}

/// The clock inside the interior. The hour is 0 to 24; a frozen clock keeps that hour until the player leaves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeOverride {
    pub hour: f32,
    pub freeze: bool,
    /// The headlights. Not applied in this delivery: they follow the hour (the game logs it).
    pub lights: Option<bool>,
}

/// What stops inside the interior. The default is all three.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Suspend {
    pub traffic: bool,
    pub cops: bool,
    pub pursuit: bool,
}

impl Default for Suspend {
    fn default() -> Self {
        Self { traffic: true, cops: true, pursuit: true }
    }
}

/// The file as serde reads it. Unknown keys are an error.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    interior: RawInterior,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawInterior {
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    label: String,
    #[serde(default)]
    prompt: Option<String>,
    zone: RawZone,
    pack: String,
    spawn: RawSpawn,
    exit: RawExit,
    #[serde(default)]
    time: Option<RawTime>,
    #[serde(default)]
    suspend: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawZone {
    shape: String,
    pos: [f32; 3],
    #[serde(default)]
    radius: Option<f32>,
    #[serde(default)]
    half: Option<[f32; 3]>,
    #[serde(default)]
    yaw: Option<f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawExit {
    shape: String,
    pos: [f32; 3],
    #[serde(default)]
    radius: Option<f32>,
    #[serde(default)]
    half: Option<[f32; 3]>,
    #[serde(default)]
    yaw: Option<f32>,
    #[serde(default)]
    prompt: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSpawn {
    pos: [f32; 3],
    heading: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTime {
    hour: f32,
    #[serde(default = "yes")]
    freeze: bool,
    #[serde(default)]
    lights: Option<bool>,
}

fn yes() -> bool {
    true
}

/// Parse `interiors/<stem>.toml`. `stem` is the file name without its extension and must equal the `key`, if one is
/// given.
pub fn parse(stem: &str, text: &str) -> Result<InteriorManifest, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    validate(stem, raw.interior)
}

fn validate(stem: &str, raw: RawInterior) -> Result<InteriorManifest, ContentError> {
    if !is_id(stem) {
        return Err(ContentError::InvalidId(stem.to_owned()));
    }
    if raw.key.as_deref().is_some_and(|key| key != stem) {
        let key = raw.key.unwrap_or_default();
        return Err(ContentError::KeyMismatch { file: stem.to_owned(), key });
    }
    kind(raw.kind.as_deref())?;
    let label = checked_text(Some(raw.label), ContentError::InvalidText)?.unwrap_or_default();
    let prompt = checked_text(raw.prompt, ContentError::InvalidText)?;
    let entry = shape("zone", raw.zone.shape, raw.zone.pos, raw.zone.radius, raw.zone.half, raw.zone.yaw)?;
    if !is_id(&raw.pack) {
        return Err(ContentError::InvalidId(raw.pack));
    }
    let spawn = spawn(raw.spawn)?;
    let exit = exit(raw.exit)?;
    let time = raw.time.map(time).transpose()?;
    let suspend = suspend(raw.suspend)?;
    Ok(InteriorManifest { key: stem.to_owned(), label, prompt, entry, pack: raw.pack, spawn, exit, time, suspend })
}

/// Only the area kind exists in this delivery: a place the car drives to. The reuse kinds are in the spec for later.
fn kind(kind: Option<&str>) -> Result<(), ContentError> {
    match kind {
        None | Some("area") => Ok(()),
        Some(other) => Err(ContentError::KindNotInV1(other.to_owned())),
    }
}

/// A shape: `sphere` with a radius, or `box` with half sizes and a turn (default 0).
fn shape(
    field: &'static str,
    kind: String,
    pos: [f32; 3],
    radius: Option<f32>,
    half: Option<[f32; 3]>,
    yaw: Option<f32>,
) -> Result<Shape, ContentError> {
    if !pos.iter().all(|v| v.is_finite()) {
        return Err(ContentError::Field { field, reason: "pos is not finite" });
    }
    match kind.as_str() {
        "sphere" => sphere(field, pos, radius, half.is_some() || yaw.is_some()),
        "box" => boxed(field, pos, radius.is_some(), half, yaw.unwrap_or(0.0)),
        _ => Err(ContentError::UnknownShape(kind.clone())),
    }
}

fn sphere(field: &'static str, pos: [f32; 3], radius: Option<f32>, has_box_keys: bool) -> Result<Shape, ContentError> {
    if has_box_keys {
        return Err(ContentError::Field { field, reason: "a sphere takes a radius only (no half or yaw)" });
    }
    let Some(radius) = radius else {
        return Err(ContentError::Field { field, reason: "a sphere needs a radius" });
    };
    if !(radius > 0.0 && radius <= MAX_RADIUS) {
        return Err(ContentError::Field { field, reason: "the radius is above 0 and at most 200 m" });
    }
    Ok(Shape::Sphere { centre: pos, radius })
}

fn boxed(
    field: &'static str,
    pos: [f32; 3],
    has_radius: bool,
    half: Option<[f32; 3]>,
    yaw: f32,
) -> Result<Shape, ContentError> {
    if has_radius {
        return Err(ContentError::Field { field, reason: "a box takes half sizes, not a radius" });
    }
    let Some(half) = half else {
        return Err(ContentError::Field { field, reason: "a box needs half sizes" });
    };
    let sizes_ok = half.iter().all(|v| *v > 0.0 && *v <= MAX_RADIUS);
    if !sizes_ok {
        return Err(ContentError::Field { field, reason: "each half size is above 0 and at most 200 m" });
    }
    if !yaw.is_finite() {
        return Err(ContentError::Field { field, reason: "yaw is not finite" });
    }
    Ok(Shape::Box { centre: pos, half, yaw })
}

fn exit(raw: RawExit) -> Result<Exit, ContentError> {
    let shape = shape("exit", raw.shape, raw.pos, raw.radius, raw.half, raw.yaw)?;
    let prompt = checked_text(raw.prompt, ContentError::InvalidText)?;
    Ok(Exit { shape, prompt })
}

fn spawn(raw: RawSpawn) -> Result<Pose, ContentError> {
    let finite = raw.pos.iter().all(|v| v.is_finite()) && raw.heading.is_finite();
    if !finite {
        return Err(ContentError::Field { field: "spawn", reason: "pos or heading is not finite" });
    }
    Ok(Pose { pos: raw.pos, heading: raw.heading })
}

fn time(raw: RawTime) -> Result<TimeOverride, ContentError> {
    if !(raw.hour >= 0.0 && raw.hour < 24.0) {
        return Err(ContentError::Field { field: "time", reason: "hour is from 0 up to 24" });
    }
    Ok(TimeOverride { hour: raw.hour, freeze: raw.freeze, lights: raw.lights })
}

/// The list of what stops. Names are `traffic`, `cops` and `pursuit`, each once. A list left out means all three.
fn suspend(names: Option<Vec<String>>) -> Result<Suspend, ContentError> {
    let Some(names) = names else { return Ok(Suspend::default()) };
    let mut suspend = Suspend { traffic: false, cops: false, pursuit: false };
    for name in names {
        let flag = match name.as_str() {
            "traffic" => &mut suspend.traffic,
            "cops" => &mut suspend.cops,
            "pursuit" => &mut suspend.pursuit,
            _ => return Err(ContentError::UnknownSuspend(name.clone())),
        };
        if *flag {
            return Err(ContentError::RepeatedSuspend(name));
        }
        *flag = true;
    }
    Ok(suspend)
}
