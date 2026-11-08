//! A grading profile: what a mod's colour grading variant looks like (`docs/modding/effect-variants.md`, "Profile format of
//! the grading"; the renderer side is `docs/specs/colour-grading.md`).
//!
//! The file is `GRADING/<value id>/profile.toml` in the mod's `files/` folder, where `<value id>` is the id of the value the
//! mod adds to the `grading_variant` option (`bb.settings`). A profile names the colour cubes it ships and says which cube is
//! in force at which hour of the day, for each weather the mod has a look for. The game blends the cube of the hour before with
//! the one after by the time between them, and the sets of two weathers by how overcast it is.
//!
//! ```toml
//! format = 1
//!
//! [[cube]]                          # 1 to 32
//! name = "noon"                     # a-z 0-9 _ -, what the keys call it
//! file = "GRADING/MW2012/C04E220.LUT"   # a .lut (binary) or .cube (Adobe text) of 2 to 64 entries a side
//!
//! [[set]]                           # one or two: "clear" (the fallback for every weather) and "overcast" (cloud and rain)
//! weather = "clear"
//! [[set.key]]                       # 1 to 64, the hours rising (0 up to 24); the day wraps from the last to the first
//! hour = 12.0
//! cube = "noon"
//! ```

use serde::Deserialize;

use super::ContentError;
use super::engine_sound::is_path;
use super::variant::{self, GRADING};

/// The folder under a mod's `files/` that holds the variants' files.
pub const GRADING_FOLDER: &str = GRADING.folder;
/// The most cubes a profile names, sets it has and keys a set has.
pub const MAX_CUBES: usize = 32;
pub const MAX_SETS: usize = 2;
pub const MAX_KEYS: usize = 64;
/// The longest a name is.
const MAX_NAME: usize = 32;

/// The path of the profile of variant `value` in the install's layout.
pub fn grading_profile_path(value: &str) -> String {
    GRADING.profile_path(value)
}

/// The weather a set is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GradingWeather {
    /// The fallback: used in every weather a profile has no set for.
    Clear,
    /// Cloud and rain.
    Overcast,
}

/// A cube file a profile names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradingCube {
    pub name: String,
    /// The file, a path of the install's layout.
    pub file: String,
}

/// The cube in force from an hour on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradingKey {
    pub hour: f32,
    /// An index into the profile's cubes.
    pub cube: usize,
}

/// The cubes of an hour: the key before it and the key after, and how far between them the hour is (0 to 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradingPair {
    pub from: usize,
    pub to: usize,
    pub mix: f32,
}

/// The keys of one weather.
#[derive(Debug, Clone, PartialEq)]
pub struct GradingSet {
    pub weather: GradingWeather,
    pub keys: Vec<GradingKey>,
}

impl GradingSet {
    /// The cubes at `hour` (wrapped into the day): the last key at or before it and the next one, the day wrapping from the
    /// last key to the first. A set of one key is that cube all day.
    pub fn at(&self, hour: f32) -> Option<GradingPair> {
        let first = self.keys.first()?;
        let last = self.keys.last()?;
        let hour = match hour.is_finite() {
            true => hour.rem_euclid(24.0),
            false => 0.0,
        };
        let after = self.keys.iter().position(|key| key.hour > hour);
        // Before the first key the previous one is the last key of the day before.
        let (prev, next, start, end) = match after {
            Some(0) => (last, first, last.hour - 24.0, first.hour),
            Some(at) => (&self.keys[at - 1], &self.keys[at], self.keys[at - 1].hour, self.keys[at].hour),
            None => (last, first, last.hour, first.hour + 24.0),
        };
        let span = end - start;
        // Two keys with the same cube blend to the same cube: no blend.
        let mix = match span > f32::EPSILON && prev.cube != next.cube {
            true => ((hour - start) / span).clamp(0.0, 1.0),
            false => 0.0,
        };
        Some(GradingPair { from: prev.cube, to: next.cube, mix })
    }
}

/// A grading variant: the cubes it ships and the key lists of its weathers.
#[derive(Debug, Clone, PartialEq)]
pub struct GradingProfile {
    pub cubes: Vec<GradingCube>,
    pub sets: Vec<GradingSet>,
}

impl GradingProfile {
    /// The set of `weather`, if the profile has one.
    pub fn set(&self, weather: GradingWeather) -> Option<&GradingSet> {
        self.sets.iter().find(|set| set.weather == weather)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: u32,
    #[serde(default)]
    cube: Vec<RawCube>,
    #[serde(default)]
    set: Vec<RawSet>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCube {
    name: String,
    file: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSet {
    weather: GradingWeather,
    #[serde(default)]
    key: Vec<RawKey>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawKey {
    hour: f32,
    cube: String,
}

fn field(field: &'static str, reason: &'static str) -> ContentError {
    ContentError::Field { field, reason }
}

fn is_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME
        && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-'))
}

/// Whether `path` is a path of the install's layout that names a cube file (`.lut` or `.cube`).
pub fn is_cube_path(path: &str) -> bool {
    is_path(path, ".lut") || is_path(path, ".cube")
}

/// Parse and check a profile. A name that is not plain or is used twice, a file that is not a cube path, a key that names no
/// cube, hours that do not rise inside 0 to 24, a weather given twice, a profile with no cube or no set, a key the schema does
/// not know or another format number is refused with the reason.
pub fn parse(text: &str) -> Result<GradingProfile, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    variant::check_format(raw.format)?;
    if raw.cube.is_empty() || raw.cube.len() > MAX_CUBES {
        return Err(field("cube", "a grading profile names 1 to 32 cubes"));
    }
    let mut cubes: Vec<GradingCube> = Vec::new();
    for cube in raw.cube {
        if !is_name(&cube.name) {
            return Err(field("cube.name", "the name of a cube is a-z, 0-9, _ and -, 1 to 32 bytes"));
        }
        if cubes.iter().any(|known| known.name == cube.name) {
            return Err(field("cube.name", "the name of a cube is used once"));
        }
        if !is_cube_path(&cube.file) {
            return Err(ContentError::InvalidText(cube.file));
        }
        cubes.push(GradingCube { name: cube.name, file: cube.file });
    }
    if raw.set.is_empty() || raw.set.len() > MAX_SETS {
        return Err(field("set", "a grading profile has 1 or 2 sets"));
    }
    let mut sets: Vec<GradingSet> = Vec::new();
    for set in raw.set {
        if sets.iter().any(|known| known.weather == set.weather) {
            return Err(field("set.weather", "a weather has one set"));
        }
        sets.push(check_set(set, &cubes)?);
    }
    Ok(GradingProfile { cubes, sets })
}

fn check_set(raw: RawSet, cubes: &[GradingCube]) -> Result<GradingSet, ContentError> {
    if raw.key.is_empty() || raw.key.len() > MAX_KEYS {
        return Err(field("set.key", "a set has 1 to 64 keys"));
    }
    let mut keys: Vec<GradingKey> = Vec::new();
    for key in raw.key {
        if !key.hour.is_finite() || !(0.0..24.0).contains(&key.hour) {
            return Err(field("set.key.hour", "an hour is a number from 0 up to 24"));
        }
        if keys.last().is_some_and(|before| before.hour >= key.hour) {
            return Err(field("set.key.hour", "the hours of a set rise"));
        }
        let Some(cube) = cubes.iter().position(|cube| cube.name == key.cube) else {
            return Err(field("set.key.cube", "a key names a cube the profile has"));
        };
        keys.push(GradingKey { hour: key.hour, cube });
    }
    Ok(GradingSet { weather: raw.weather, keys })
}

#[cfg(test)]
#[path = "grading_tests.rs"]
mod tests;
