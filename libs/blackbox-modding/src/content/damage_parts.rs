//! The `[[damage_parts]]` tables of a car manifest: the parts of the car that swing on a hinge, or fall off, when the
//! zones around them are damaged.
//!
//! A part is a solid of the car's own `GEOMETRY.BIN` (named by `model`, without the level-of-detail letter) that the
//! body does not contain. While the car is intact it is drawn where it was modelled; once the damage of one of its
//! `zones` reaches the first of its `stages` it turns about the line through `pivot` along `axis`, further with every
//! stage passed, up to `angle` degrees at the last. A part with `detach_at` is not drawn once the damage reaches it.
//! This module checks the schema only; whether the model exists is for the game to decide, because only the game opens
//! the car's files. Spec: `docs/modding/car-damage-parts.md`.

use serde::Deserialize;

use super::ContentError;

/// Most hinged parts a car lists.
pub const MAX_DAMAGE_PARTS: usize = 16;
/// Most stages of one part.
pub const MAX_STAGES: usize = 8;
/// Longest name, in characters, and longest model name, in bytes (the game appends `_A` to `_E`).
const MAX_NAME_CHARS: usize = 32;
const MAX_MODEL_BYTES: usize = 56;
/// The biggest coordinate of a pivot, in metres: far outside any car.
const MAX_COORDINATE: f32 = 50.0;

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
    /// The zone damage (above 0, at most 1) from which the part is gone. `None`: it never falls off.
    pub detach_at: Option<f32>,
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
    detach_at: Option<f32>,
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
        return Err(field("a car lists at most 16 parts"));
    }
    let mut parts: Vec<DamagePart> = Vec::with_capacity(raw.len());
    for entry in raw {
        let part = check_one(entry)?;
        if parts.iter().any(|known| known.name == part.name) {
            return Err(field("two parts have the same name"));
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
    if !raw.angle.is_finite() || raw.angle == 0.0 || raw.angle.abs() > 180.0 {
        return Err(field("an angle is a number of degrees, not 0, within 180"));
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
    let stages = if raw.stages.is_empty() { vec![0.0] } else { raw.stages };
    let axis = raw.axis.map(|c| c / length);
    Ok(DamagePart {
        name,
        model: raw.model,
        zones,
        pivot: raw.pivot,
        axis,
        angle: raw.angle,
        stages,
        detach_at: raw.detach_at,
    })
}
