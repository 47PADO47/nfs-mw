//! A skid mark profile: what a mod's skid mark variant looks like (`docs/modding/effect-variants-marks-rain.md`).
//!
//! The file is `SKIDMARKS/<value id>/profile.toml` in the mod's `files/` folder, where `<value id>` is the id of the value the
//! mod adds to the `skid_marks_variant` option (`bb.settings`). The game's own marks are dark untextured strips behind each
//! tyre that skids; a profile gives them a picture. A profile is a list of sets: one for each kind of road (paved or
//! loose) and, optionally, for the wet road. A set holds two pictures, the mark of a tyre that slides sideways (`slip`) and
//! the mark of a tyre that spins or locks along the road (`spin`); the strip is drawn with the picture repeated along it.
//! Where the marks start, how wide the tyre is and when a bike leaves any is the game's own: the profile only changes the look.
//!
//! ```toml
//! format = 1
//!
//! [[set]]
//! surface = "paved"                       # "paved" or "loose" (grass, sand, dirt, mud, gravel, golf turf)
//! wet = false                             # true: the set of a wet road; a surface with no wet set uses its dry one
//! slip = "SKIDMARKS/MW2012/PAVED_SLIP.PNG"
//! spin = "SKIDMARKS/MW2012/PAVED_SPIN.PNG"
//! slip_tile = 4.0                         # metres of mark one repeat of the picture covers (0.25 to 64)
//! spin_tile = 2.0
//! width = 1.0                             # times the width of the tyre's tread (0.25 to 4)
//! opacity = 1.0                           # times the picture's alpha, 0 to 1
//! life = 45.0                             # seconds a mark stays (5 to 300); the last 5 of them it fades
//! ```
//!
//! The picture is an RGBA PNG: its columns run along the mark and its rows across the tyre, the alpha is the mark's strength
//! and the colour is the mark's colour.

use std::ops::RangeInclusive;

use serde::Deserialize;

use super::ContentError;
use super::engine_sound::is_png_path;
use super::variant::{self, SKID_MARKS};

/// The folder under a mod's `files/` that holds the variants' files.
pub const SKIDMARKS_FOLDER: &str = SKID_MARKS.folder;
/// The most sets a profile may have.
pub const MAX_SETS: usize = 8;
/// The range of a tile length, in metres.
pub const TILES: RangeInclusive<f32> = 0.25..=64.0;
/// The range of a width factor.
pub const WIDTHS: RangeInclusive<f32> = 0.25..=4.0;
/// The range of a mark's life, seconds.
pub const LIVES: RangeInclusive<f32> = 5.0..=300.0;

/// The path of the profile of variant `value` in the install's layout.
pub fn skid_marks_profile_path(value: &str) -> String {
    SKID_MARKS.profile_path(value)
}

/// The kind of road a set is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarkSurface {
    Paved,
    Loose,
}

/// One set of pictures for a kind of road.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkSet {
    pub surface: MarkSurface,
    pub wet: bool,
    /// The picture of a tyre sliding sideways, a path of the install's layout; `None` takes the `spin` one.
    pub slip: Option<String>,
    /// The picture of a tyre spinning or locking along the road; `None` takes the `slip` one.
    pub spin: Option<String>,
    pub slip_tile: f32,
    pub spin_tile: f32,
    pub width: f32,
    pub opacity: f32,
    pub life: f32,
}

/// A skid mark variant: the sets, in file order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SkidMarksProfile {
    pub sets: Vec<MarkSet>,
}

impl SkidMarksProfile {
    /// The set that draws the marks on `surface`: the wet one when `wet` and there is one, else the dry one, else `None`
    /// (the game's own mark).
    pub fn set_for(&self, surface: MarkSurface, wet: bool) -> Option<&MarkSet> {
        let of = |wet: bool| self.sets.iter().find(|set| set.surface == surface && set.wet == wet);
        wet.then(|| of(true)).flatten().or_else(|| of(false))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: u32,
    #[serde(default)]
    set: Vec<RawSet>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSet {
    surface: MarkSurface,
    #[serde(default)]
    wet: bool,
    slip: Option<String>,
    spin: Option<String>,
    slip_tile: Option<f32>,
    spin_tile: Option<f32>,
    width: Option<f32>,
    opacity: Option<f32>,
    life: Option<f32>,
}

fn field(field: &'static str, reason: &'static str) -> ContentError {
    ContentError::Field { field, reason }
}

fn within(
    value: Option<f32>,
    default: f32,
    range: &RangeInclusive<f32>,
    name: &'static str,
) -> Result<f32, ContentError> {
    let value = value.unwrap_or(default);
    if !value.is_finite() || !range.contains(&value) {
        return Err(field(name, "a number outside the range this field takes"));
    }
    Ok(value)
}

fn picture(path: Option<String>, name: &'static str) -> Result<Option<String>, ContentError> {
    let Some(path) = path else { return Ok(None) };
    if !is_png_path(&path) {
        return Err(field(name, "a relative .PNG path of letters, digits, _ - . /"));
    }
    Ok(Some(path))
}

/// Parse and check a profile. A number out of range, a path that is not a `.PNG` path, a set with no picture, a road that has
/// two sets alike, a key the schema does not know, a profile with no set or another format number is refused with the reason.
pub fn parse(text: &str) -> Result<SkidMarksProfile, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    variant::check_format(raw.format)?;
    if raw.set.is_empty() {
        return Err(field("set", "a skid mark profile needs at least one set"));
    }
    if raw.set.len() > MAX_SETS {
        return Err(field("set", "more sets than this effect takes"));
    }
    let mut sets: Vec<MarkSet> = Vec::new();
    for raw in raw.set {
        let (slip, spin) = (picture(raw.slip, "set.slip")?, picture(raw.spin, "set.spin")?);
        if slip.is_none() && spin.is_none() {
            return Err(field("set", "a set needs a slip or a spin picture"));
        }
        if sets.iter().any(|set| set.surface == raw.surface && set.wet == raw.wet) {
            return Err(field("set", "two sets for the same road"));
        }
        sets.push(MarkSet {
            surface: raw.surface,
            wet: raw.wet,
            slip,
            spin,
            slip_tile: within(raw.slip_tile, 4.0, &TILES, "set.slip_tile")?,
            spin_tile: within(raw.spin_tile, 2.0, &TILES, "set.spin_tile")?,
            width: within(raw.width, 1.0, &WIDTHS, "set.width")?,
            opacity: within(raw.opacity, 1.0, &(0.0..=1.0), "set.opacity")?,
            life: within(raw.life, 45.0, &LIVES, "set.life")?,
        });
    }
    Ok(SkidMarksProfile { sets })
}

#[cfg(test)]
#[path = "skid_marks_tests.rs"]
mod tests;
