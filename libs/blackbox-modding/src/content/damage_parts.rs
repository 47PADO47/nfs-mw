//! The `[[damage_parts]]` tables of a car manifest: the parts of the car that swing on a hinge, or fall off, when the
//! zones around them are damaged.
//!
//! A part is a solid of the car's own `GEOMETRY.BIN` (named by `model`, without the level-of-detail letter) that the
//! body does not contain. While the car is intact it is drawn where it was modelled; once the damage of one of its
//! `zones` reaches the first of its `stages` it turns about the line through `pivot` along `axis`, further with every
//! stage passed, up to `angle` degrees at the last. From `detach_at` on, or when the car is wrecked, the part leaves the
//! body and falls as a physical object of `mass_kg`; `detach = false` keeps it on the car for good.
//! This module checks the schema only; whether the model exists is for the game to decide, because only the game opens
//! the car's files. A part may also move with the car's speed, as an active spoiler does (`[damage_parts.active]`).
//! Spec: `docs/modding/car-damage-parts.md`.

use serde::Deserialize;

use super::ContentError;

mod play;
pub use play::{Drive, DriveInput, DriveOp, DriveTest, MAX_DRIVES, Track, WING_SLOTS, Wing};
use play::{RawDrive, RawTrack, check_drive, check_track};
pub(in crate::content) use play::{RawWing, check_wing};

/// Most hinged parts a car lists.
pub const MAX_DAMAGE_PARTS: usize = 64;
/// Most stages of one part.
pub const MAX_STAGES: usize = 8;
/// The mass of a part that gives none, in kg, and the range a mass may have.
pub const DEFAULT_MASS_KG: f32 = 25.0;
const MASS_RANGE: (f32, f32) = (0.5, 5000.0);
/// Longest name, in characters, and longest model name, in bytes (the game appends `_A` to `_E`).
const MAX_NAME_CHARS: usize = 32;
const MAX_MODEL_BYTES: usize = 56;
/// The biggest coordinate of a pivot, in metres: far outside any car.
const MAX_COORDINATE: f32 = 50.0;
/// The biggest movement of an active part, in metres, and the speeds and times it may name.
const MAX_TRAVEL: f32 = 5.0;
const MAX_MPH: f32 = 500.0;
const SECONDS_RANGE: (f32, f32) = (0.05, 10.0);

/// A damage zone, in the order the game counts them (`DZ_FRONT` to `DZ_RREAR`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Front,
    Rear,
    Left,
    Right,
    FrontLeft,
    FrontRight,
    RearLeft,
    RearRight,
}

impl Zone {
    /// The zone a manifest names: `front`, `rear`, `left`, `right`, `front_left`, `front_right`, `rear_left` or
    /// `rear_right`.
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "front" => Self::Front,
            "rear" => Self::Rear,
            "left" => Self::Left,
            "right" => Self::Right,
            "front_left" => Self::FrontLeft,
            "front_right" => Self::FrontRight,
            "rear_left" => Self::RearLeft,
            "rear_right" => Self::RearRight,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Front => "front",
            Self::Rear => "rear",
            Self::Left => "left",
            Self::Right => "right",
            Self::FrontLeft => "front_left",
            Self::FrontRight => "front_right",
            Self::RearLeft => "rear_left",
            Self::RearRight => "rear_right",
        }
    }

    /// The position of the zone in the game's zone order: 0 front, 1 rear, 2 left, 3 right, 4 front left, 5 front right,
    /// 6 rear left, 7 rear right.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// How a part moves with the car's speed: it turns by `rotation` about its pivot and slides by `offset` when the car is
/// above `above_mph` and goes back when the car is below `below_mph`, taking `seconds` for the whole way (linear in time).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Active {
    /// The movement at full deployment, in the car's solid space (+x forward, +y left, +z up), metres.
    pub offset: [f32; 3],
    /// The turn at full deployment: a unit axis through the part's pivot (same space) and an angle in degrees, right-handed.
    /// The part turns first and then slides. `None`: no turn.
    pub rotation: Option<([f32; 3], f32)>,
    /// The part deploys above this speed, mph.
    pub above_mph: f32,
    /// It retracts below this speed, mph (not above `above_mph`).
    pub below_mph: f32,
    /// How long the whole movement takes, seconds.
    pub seconds: f32,
    /// The throttle matters: the part deploys only while the gas is pressed (above `above_mph`) and retracts when the gas
    /// is released; the speed alone does not retract it (`below_mph` is then 0).
    pub gas: bool,
    /// An air vent: `(open_after, close_after)` in seconds. It opens once the speed has stayed above `above_mph` for the first,
    /// then plays out; when the speed falls below `below_mph` it waits the second and then closes, without looking at the
    /// speed again until it has closed. `None`: it follows the speed at once.
    pub delay: Option<(f32, f32)>,
}

/// A checked `[[damage_parts]]` entry.
#[derive(Debug, Clone, PartialEq)]
pub struct DamagePart {
    pub name: String,
    /// The part's solid in the car's own `GEOMETRY.BIN`, without the level-of-detail letter.
    pub model: String,
    /// The zones whose damage moves the part: it follows the worst of them. At least one, no repeats.
    pub zones: Vec<Zone>,
    /// A point of the hinge line, in the car's solid space (+x forward, +y left, +z up), metres.
    pub pivot: [f32; 3],
    /// The direction of the hinge line, normalised.
    pub axis: [f32; 3],
    /// How far the part has turned at the last stage, in degrees (right-handed about `axis`; not 0, at most 180).
    pub angle: f32,
    /// The zone damage (0 to 1, ascending) at which each stage starts. The part has turned `angle` times the share of
    /// stages passed. Left out: one stage at the first damage.
    pub stages: Vec<f32>,
    /// The degrees the part has turned at each stage (one per stage), when the stages do not turn it evenly. Empty: it has
    /// turned `angle` times the share of stages passed.
    pub stage_angles: Vec<f32>,
    /// The zone damage (above 0, at most 1) from which the part falls off. `None` (with `detach` true): the last stage,
    /// but not before half the damage the zone can take (`DamagePart::release_level` in the game).
    pub detach_at: Option<f32>,
    /// Whether the part can leave the car at all. `false` keeps it on the car whatever the damage, wreck included.
    pub detach: bool,
    /// What the part weighs once it is a body of its own, in kg.
    pub mass_kg: f32,
    /// How the part moves with the car's speed, if it does (`[damage_parts.active]`).
    pub active: Option<Active>,
    /// The name of the part it hangs from (an earlier one that moves with the speed): its movement rides on the parent's.
    /// Only for a part that moves with the speed itself.
    pub parent: Option<String>,
    /// The animations of the car wing machine that move the part (`[[damage_parts.track]]`), empty for none.
    pub tracks: Vec<Track>,
    /// The conditions that play the part's `active` movement (`[[damage_parts.drive]]`), empty for none.
    pub drives: Vec<Drive>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawActive {
    #[serde(default)]
    offset: [f32; 3],
    #[serde(default)]
    rotation_axis: Option<[f32; 3]>,
    #[serde(default)]
    rotation_deg: Option<f32>,
    above_mph: f32,
    #[serde(default)]
    below_mph: Option<f32>,
    seconds: f32,
    #[serde(default)]
    gas: bool,
    #[serde(default)]
    open_after_seconds: Option<f32>,
    #[serde(default)]
    close_after_seconds: Option<f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Raw {
    name: String,
    model: String,
    zones: Vec<String>,
    pivot: [f32; 3],
    axis: [f32; 3],
    angle: f32,
    #[serde(default)]
    stages: Vec<f32>,
    #[serde(default)]
    stage_angles: Vec<f32>,
    #[serde(default)]
    detach_at: Option<f32>,
    #[serde(default)]
    detach: Option<bool>,
    #[serde(default)]
    mass_kg: Option<f32>,
    #[serde(default)]
    active: Option<RawActive>,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    track: Vec<RawTrack>,
    #[serde(default)]
    drive: Vec<RawDrive>,
}

fn field(reason: &'static str) -> ContentError {
    ContentError::Field { field: "damage_parts", reason }
}

const ZONE_HELP: &str = "a zone is front, rear, left, right, front_left, front_right, rear_left or rear_right";

fn is_model(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= MAX_MODEL_BYTES
        && text.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

/// Check the entries. A car with none has no hinged parts.
pub(super) fn check(raw: Option<Vec<Raw>>) -> Result<Vec<DamagePart>, ContentError> {
    let Some(raw) = raw else { return Ok(Vec::new()) };
    if raw.len() > MAX_DAMAGE_PARTS {
        return Err(field("a car lists at most 64 parts"));
    }
    let mut parts: Vec<DamagePart> = Vec::with_capacity(raw.len());
    for entry in raw {
        let part = check_one(entry)?;
        if parts.iter().any(|known| known.name == part.name) {
            return Err(field("two parts have the same name"));
        }
        let moves = |part: &DamagePart| part.active.is_some() || !part.tracks.is_empty();
        if let Some(parent) = &part.parent {
            let hangs = moves(&part) && parts.iter().any(|known| known.name == *parent && moves(known));
            if !hangs {
                return Err(field(
                    "a parent is an earlier part that moves with the speed or the wing, for a part that does too",
                ));
            }
        }
        parts.push(part);
    }
    Ok(parts)
}

fn check_one(raw: Raw) -> Result<DamagePart, ContentError> {
    let name = raw.name.trim().to_owned();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS || name.chars().any(char::is_control) {
        return Err(field("a name is 1 to 32 characters with no control character"));
    }
    if !is_model(&raw.model) {
        return Err(ContentError::InvalidModel(raw.model));
    }
    let mut zones: Vec<Zone> = Vec::with_capacity(raw.zones.len());
    for text in &raw.zones {
        let zone = Zone::parse(text).ok_or_else(|| field(ZONE_HELP))?;
        if zones.contains(&zone) {
            return Err(field("a zone is listed twice"));
        }
        zones.push(zone);
    }
    if zones.is_empty() {
        return Err(field("a part needs at least one zone"));
    }
    if raw.pivot.iter().any(|c| !c.is_finite() || c.abs() > MAX_COORDINATE) {
        return Err(field("a pivot is three finite coordinates within 50 m"));
    }
    let length = raw.axis.iter().map(|c| c * c).sum::<f32>().sqrt();
    if raw.axis.iter().any(|c| !c.is_finite()) || length < 1e-3 || length > 1e6 {
        return Err(field("an axis is a direction: three finite numbers, not all 0"));
    }
    if !raw.angle.is_finite() || raw.angle.abs() > 180.0 {
        return Err(field("an angle is a number of degrees within 180 (0: the damage does not turn the part)"));
    }
    if raw.stages.len() > MAX_STAGES {
        return Err(field("a part has at most 8 stages"));
    }
    let out_of_range = raw.stages.iter().any(|s| !s.is_finite() || !(0.0..=1.0).contains(s));
    if out_of_range || raw.stages.windows(2).any(|w| w[0] > w[1]) {
        return Err(field("stages are damage fractions from 0 to 1, in ascending order"));
    }
    if raw.detach_at.is_some_and(|at| !at.is_finite() || at <= 0.0 || at > 1.0) {
        return Err(field("detach_at is a damage fraction above 0 and at most 1"));
    }
    let detach = raw.detach.unwrap_or(true);
    if !detach && raw.detach_at.is_some() {
        return Err(field("detach_at needs a part that can detach: detach is false"));
    }
    let mass_kg = raw.mass_kg.unwrap_or(DEFAULT_MASS_KG);
    if !mass_kg.is_finite() || mass_kg < MASS_RANGE.0 || mass_kg > MASS_RANGE.1 {
        return Err(field("mass_kg is from 0.5 to 5000"));
    }
    let active = raw.active.map(check_active).transpose()?;
    let tracks = raw.track.into_iter().map(check_track).collect::<Result<Vec<_>, _>>()?;
    let drives = raw.drive.into_iter().map(check_drive).collect::<Result<Vec<_>, _>>()?;
    if drives.len() > MAX_DRIVES || (!drives.is_empty() && (active.is_none() || !tracks.is_empty())) {
        return Err(field("a part has up to 4 drives, with an [damage_parts.active] movement and no tracks"));
    }
    if active.is_some_and(|a| a.delay.is_some() || a.gas) && !drives.is_empty() {
        return Err(field("a part with drives does not also wait or follow the gas"));
    }
    if active.is_some() && !tracks.is_empty() {
        return Err(field("a part moves with the speed (active) or with the wing (track), not both"));
    }
    if (0..WING_SLOTS).any(|slot| tracks.iter().filter(|track| track.slot == slot).count() > 1) {
        return Err(field("a part has one track per wing animation"));
    }
    let stages = if raw.stages.is_empty() { vec![0.0] } else { raw.stages };
    let angles_ok = raw.stage_angles.iter().all(|a| a.is_finite() && a.abs() <= 180.0);
    if !angles_ok || (!raw.stage_angles.is_empty() && raw.stage_angles.len() != stages.len()) {
        return Err(field("stage_angles is one angle of up to 180 degrees per stage"));
    }
    let axis = raw.axis.map(|c| c / length);
    Ok(DamagePart {
        name,
        model: raw.model,
        zones,
        pivot: raw.pivot,
        axis,
        angle: raw.angle,
        stages,
        stage_angles: raw.stage_angles,
        detach_at: raw.detach_at,
        detach,
        mass_kg,
        active,
        parent: raw.parent.map(|name| name.trim().to_owned()),
        tracks,
        drives,
    })
}

fn check_active(raw: RawActive) -> Result<Active, ContentError> {
    let below_mph = raw.below_mph.unwrap_or(raw.above_mph);
    let rotation = check_rotation(raw.rotation_axis, raw.rotation_deg)?;
    if raw.offset.iter().any(|c| !c.is_finite() || c.abs() > MAX_TRAVEL) {
        return Err(field("an active offset is a movement of up to 5 m"));
    }
    if raw.offset == [0.0; 3] && rotation.is_none() {
        return Err(field("an active part moves (offset) or turns (rotation_axis and rotation_deg)"));
    }
    let speeds = [raw.above_mph, below_mph];
    if speeds.iter().any(|v| !v.is_finite() || !(0.0..=MAX_MPH).contains(v)) || below_mph > raw.above_mph {
        return Err(field("active speeds are 0 to 500 mph and below_mph is not above above_mph"));
    }
    if !raw.seconds.is_finite() || !(SECONDS_RANGE.0..=SECONDS_RANGE.1).contains(&raw.seconds) {
        return Err(field("an active movement takes 0.05 to 10 seconds"));
    }
    let delay = match (raw.open_after_seconds, raw.close_after_seconds) {
        (None, None) => None,
        (Some(open), Some(close)) if [open, close].iter().all(|v| v.is_finite() && (0.0..=60.0).contains(v)) => {
            Some((open, close))
        }
        _ => return Err(field("open_after_seconds and close_after_seconds come together, 0 to 60 seconds")),
    };
    if delay.is_some() && raw.gas {
        return Err(field("a vent that waits does not follow the gas"));
    }
    Ok(Active {
        offset: raw.offset,
        rotation,
        above_mph: raw.above_mph,
        below_mph,
        seconds: raw.seconds,
        gas: raw.gas,
        delay,
    })
}

/// The turn of an active part: both of `rotation_axis` and `rotation_deg` or none of them.
fn check_rotation(axis: Option<[f32; 3]>, degrees: Option<f32>) -> Result<Option<([f32; 3], f32)>, ContentError> {
    let (axis, degrees) = match (axis, degrees) {
        (None, None) => return Ok(None),
        (Some(axis), Some(degrees)) => (axis, degrees),
        _ => return Err(field("rotation_axis and rotation_deg come together")),
    };
    let length = axis.iter().map(|c| c * c).sum::<f32>().sqrt();
    if axis.iter().any(|c| !c.is_finite()) || !(1e-3..=1e6).contains(&length) {
        return Err(field("a rotation_axis is a direction: three finite numbers, not all 0"));
    }
    if !degrees.is_finite() || degrees == 0.0 || degrees.abs() > 180.0 {
        return Err(field("rotation_deg is a turn of up to 180 degrees, not 0"));
    }
    Ok(Some((axis.map(|c| c / length), degrees)))
}
