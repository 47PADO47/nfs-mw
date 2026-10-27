//! The `[[lamp_flare]]` and `[[light_bar]]` tables of a car manifest: the flares and glows a car's lamps are drawn with.
//!
//! A car without these tables is lit by its lamp markers and the game's shared flare profiles (`docs/modding/car-lamps.md`).
//! A `[[lamp_flare]]` is one sprite at one place on the car that shows while its lamp is lit, sized in metres and fading with
//! distance and with the angle off the lamp's axis. A `[[light_bar]]` is the same sprite for a siren lamp: it flashes while the
//! car's emergency lights are on, by a pattern of steps. This module checks the schema only; whether the sprite exists in the
//! car's textures is for the game to decide.

use serde::Deserialize;

use super::ContentError;

/// Most flares and most siren lamps one car has.
pub const MAX_FLARES: usize = 96;
pub const MAX_BARS: usize = 32;
/// Longest flash pattern, in steps.
pub const MAX_PATTERN: usize = 4096;
/// Longest texture name, in bytes (the same limit as the other pictures of a car).
const MAX_ASSET_BYTES: usize = 56;
/// Furthest a lamp is from the car's origin, metres.
const MAX_REACH: f32 = 20.0;
/// Largest sprite radius, metres, and largest distance.
const MAX_RADIUS: f32 = 50.0;
const MAX_RANGE: f32 = 1000.0;
/// Largest brightness: the sprite's colour times this, so a value over 1 is a glow that saturates.
const MAX_ALPHA: f32 = 4.0;
const MAX_FALLOFF: f32 = 16.0;
const MAX_Z_BIAS: f32 = 2.0;
/// The shortest and longest step of a flash pattern, seconds.
const MIN_STEP: f32 = 0.005;
const MAX_STEP: f32 = 2.0;

/// When a lamp is lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LampLit {
    /// The headlights are on.
    Head,
    /// The rear lamps burn with the headlights.
    Tail,
    /// The brake is held.
    Brake,
    /// The car reverses.
    Reverse,
    /// Always.
    Always,
}

/// Which lamp of the car it is: a broken lamp (a damaged left head lamp, say) puts only its own sprites out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LampSide {
    Left,
    Right,
    #[default]
    Centre,
}

/// The colour a siren lamp is, which says which emergency light bit switches and breaks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SirenColour {
    Red,
    Blue,
    White,
}

/// The look shared by the flares and the siren lamps.
#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    /// A texture of the car's own `TEXTURES.BIN`, drawn additively.
    pub sprite: String,
    /// Where the lamp is, in the car's space: +x forward, +y left, +z up, metres.
    pub position: [f32; 3],
    /// The way the lamp shines: a unit vector.
    pub direction: [f32; 3],
    /// The sprite's tint, 0 to 1.
    pub colour: [f32; 3],
    /// The brightness (the sprite's colour times it) at the near and the far distance of `range`.
    pub alpha: [f32; 2],
    /// The sprite's radius, metres, at the near and the far distance of `range`.
    pub radius: [f32; 2],
    /// The two distances, metres, `alpha` and `radius` are given at; between them they change in a straight line, beyond
    /// them they stay.
    pub range: [f32; 2],
    /// The exponent of the cosine of the angle off the lamp's axis the brightness is multiplied by (0: seen from every side).
    pub falloff: f32,
    /// How far towards the eye the sprite is drawn, metres.
    pub z_bias: f32,
}

/// A checked `[[lamp_flare]]`.
#[derive(Debug, Clone, PartialEq)]
pub struct LampFlare {
    pub lit: LampLit,
    pub side: LampSide,
    pub look: Sprite,
}

/// A checked `[[light_bar]]`.
#[derive(Debug, Clone, PartialEq)]
pub struct LightBar {
    pub siren: SirenColour,
    pub look: Sprite,
    /// The brightness of each step, 0 to 1, played in a loop.
    pub pattern: Vec<f32>,
    /// How long one step lasts, seconds.
    pub step_seconds: f32,
    /// Where in the pattern the lamp starts, seconds.
    pub phase_seconds: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawFlare {
    state: LampLit,
    #[serde(default)]
    side: LampSide,
    sprite: String,
    position: [f32; 3],
    direction: [f32; 3],
    #[serde(default = "white")]
    colour: [f32; 3],
    #[serde(default = "ones")]
    alpha: [f32; 2],
    radius: [f32; 2],
    #[serde(default = "default_range")]
    range: [f32; 2],
    #[serde(default = "one")]
    falloff: f32,
    #[serde(default)]
    z_bias: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawBar {
    siren: SirenColour,
    /// One digit per step, `0` to `9`: the brightness in ninths.
    pattern: String,
    step_seconds: f32,
    #[serde(default)]
    phase_seconds: f32,
    sprite: String,
    position: [f32; 3],
    direction: [f32; 3],
    #[serde(default = "white")]
    colour: [f32; 3],
    #[serde(default = "ones")]
    alpha: [f32; 2],
    radius: [f32; 2],
    #[serde(default = "default_range")]
    range: [f32; 2],
    #[serde(default = "one")]
    falloff: f32,
    #[serde(default)]
    z_bias: f32,
}

/// The fields the two tables share.
struct RawLook {
    sprite: String,
    position: [f32; 3],
    direction: [f32; 3],
    colour: [f32; 3],
    alpha: [f32; 2],
    radius: [f32; 2],
    range: [f32; 2],
    falloff: f32,
    z_bias: f32,
}

impl From<&RawFlare> for RawLook {
    fn from(raw: &RawFlare) -> Self {
        Self {
            sprite: raw.sprite.clone(),
            position: raw.position,
            direction: raw.direction,
            colour: raw.colour,
            alpha: raw.alpha,
            radius: raw.radius,
            range: raw.range,
            falloff: raw.falloff,
            z_bias: raw.z_bias,
        }
    }
}

impl From<&RawBar> for RawLook {
    fn from(raw: &RawBar) -> Self {
        Self {
            sprite: raw.sprite.clone(),
            position: raw.position,
            direction: raw.direction,
            colour: raw.colour,
            alpha: raw.alpha,
            radius: raw.radius,
            range: raw.range,
            falloff: raw.falloff,
            z_bias: raw.z_bias,
        }
    }
}

fn white() -> [f32; 3] {
    [1.0; 3]
}

fn ones() -> [f32; 2] {
    [1.0; 2]
}

fn one() -> f32 {
    1.0
}

fn default_range() -> [f32; 2] {
    [2.0, 100.0]
}

fn field(field: &'static str, reason: &'static str) -> ContentError {
    ContentError::Field { field, reason }
}

fn within(values: &[f32], low: f32, high: f32) -> bool {
    values.iter().all(|v| v.is_finite() && (low..=high).contains(v))
}

fn look(raw: RawLook, table: &'static str) -> Result<Sprite, ContentError> {
    let valid = !raw.sprite.is_empty()
        && raw.sprite.len() <= MAX_ASSET_BYTES
        && raw.sprite.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
    if !valid {
        return Err(ContentError::InvalidModel(raw.sprite));
    }
    if !within(&raw.position, -MAX_REACH, MAX_REACH) {
        return Err(field(table, "position is within 20 m of the car's origin"));
    }
    let length = raw.direction.iter().map(|v| v * v).sum::<f32>().sqrt();
    if !within(&raw.direction, -1e6, 1e6) || length < 1e-3 {
        return Err(field(table, "direction is a vector that is not zero"));
    }
    if !within(&raw.colour, 0.0, 1.0) {
        return Err(field(table, "colour is three numbers from 0 to 1"));
    }
    if !within(&raw.alpha, 0.0, MAX_ALPHA) {
        return Err(field(table, "alpha is two numbers from 0 to 4"));
    }
    if !within(&raw.radius, 0.0, MAX_RADIUS) {
        return Err(field(table, "radius is two numbers from 0 to 50 metres"));
    }
    if !within(&raw.range, 0.0, MAX_RANGE) || raw.range[0] >= raw.range[1] {
        return Err(field(table, "range is a near and a farther distance, up to 1000 metres"));
    }
    if !within(&[raw.falloff], 0.0, MAX_FALLOFF) {
        return Err(field(table, "falloff is from 0 to 16"));
    }
    if !within(&[raw.z_bias], 0.0, MAX_Z_BIAS) {
        return Err(field(table, "z_bias is from 0 to 2 metres"));
    }
    Ok(Sprite {
        sprite: raw.sprite,
        position: raw.position,
        direction: raw.direction.map(|v| v / length),
        colour: raw.colour,
        alpha: raw.alpha,
        radius: raw.radius,
        range: raw.range,
        falloff: raw.falloff,
        z_bias: raw.z_bias,
    })
}

/// Check the `[[lamp_flare]]` tables.
pub(super) fn check_flares(raw: Option<Vec<RawFlare>>) -> Result<Vec<LampFlare>, ContentError> {
    let raw = raw.unwrap_or_default();
    if raw.len() > MAX_FLARES {
        return Err(field("lamp_flare", "a car has at most 96 flares"));
    }
    raw.into_iter()
        .map(|flare| {
            Ok(LampFlare { lit: flare.state, side: flare.side, look: look(RawLook::from(&flare), "lamp_flare")? })
        })
        .collect()
}

/// Check the `[[light_bar]]` tables.
pub(super) fn check_bars(raw: Option<Vec<RawBar>>) -> Result<Vec<LightBar>, ContentError> {
    let raw = raw.unwrap_or_default();
    if raw.len() > MAX_BARS {
        return Err(field("light_bar", "a car has at most 32 siren lamps"));
    }
    raw.into_iter().map(bar).collect()
}

fn bar(raw: RawBar) -> Result<LightBar, ContentError> {
    let steps: Option<Vec<f32>> = raw.pattern.chars().map(|c| c.to_digit(10).map(|d| d as f32 / 9.0)).collect();
    let Some(pattern) = steps.filter(|steps| !steps.is_empty() && steps.len() <= MAX_PATTERN) else {
        return Err(field("light_bar.pattern", "one digit from 0 to 9 for each step, 1 to 4096 steps"));
    };
    if !within(&[raw.step_seconds], MIN_STEP, MAX_STEP) {
        return Err(field("light_bar.step_seconds", "a step lasts 0.005 to 2 seconds"));
    }
    if !within(&[raw.phase_seconds], 0.0, 100_000.0) {
        return Err(field("light_bar.phase_seconds", "the phase is a number of seconds, not negative"));
    }
    Ok(LightBar {
        siren: raw.siren,
        look: look(RawLook::from(&raw), "light_bar")?,
        pattern,
        step_seconds: raw.step_seconds,
        phase_seconds: raw.phase_seconds,
    })
}

#[cfg(test)]
#[path = "lamp_flares_tests.rs"]
mod tests;
