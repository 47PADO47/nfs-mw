//! A wind profile: what a mod's wind variant sounds like (`docs/modding/effect-variants.md`, "Profile format of the wind").
//!
//! The file is `WIND/<value id>/profile.toml` in the mod's `files/` folder, where `<value id>` is the id of the value the mod
//! adds to the `wind_variant` option (`bb.settings`). A profile is a list of looping recordings that play all the time the
//! car is driven; each is as loud as a curve of the car's speed says and plays at a fixed rate. The game's own wind (the
//! `FX_WIND` module of the 2005 game) is silent while a profile plays.
//!
//! ```toml
//! format = 1
//! volume = 1.0                   # a level every layer is multiplied by, 0 to 8 (default 1)
//!
//! [[layer]]                      # one to sixteen
//! file = "WIND/MW2012/W182159.WAV"   # a 16-bit PCM WAV, mono or stereo, that loops over its whole length
//! volume = 0.22                  # this layer's level, 0 to 8 (default 1)
//! rate = 1.3                     # playback rate, 0.25 to 4 (default 1)
//! gain = [[0.0, 0.0], [45.0, 0.0], [65.0, 1.0], [90.0, 0.0]]   # miles an hour and level, 0 to 8
//! ```
//!
//! `gain` is piecewise linear and flat before its first and after its last point (two to 64 points, the speeds rising).

use serde::Deserialize;

use super::ContentError;
use super::engine_sound::is_sound_path;
use super::variant::{self, WIND};

/// The folder under a mod's `files/` that holds the variants' files.
pub const WIND_FOLDER: &str = WIND.folder;
/// The most layers a profile may have.
pub const MAX_LAYERS: usize = 16;
/// Fewest and most points of a layer's curve.
pub const POINTS: std::ops::RangeInclusive<usize> = 2..=64;
/// The largest level of a point, of a layer and of the profile.
pub const MAX_LEVEL: f32 = 8.0;
/// The range of a playback rate.
pub const RATES: std::ops::RangeInclusive<f32> = 0.25..=4.0;

/// The path of the profile of variant `value` in the install's layout.
pub fn wind_profile_path(value: &str) -> String {
    WIND.profile_path(value)
}

/// One looping recording of the wind.
#[derive(Debug, Clone, PartialEq)]
pub struct WindLayer {
    /// The WAV, a path of the install's layout.
    pub file: String,
    pub volume: f32,
    pub rate: f32,
    /// The level at a speed in mph: `(speed, level)`, speeds rising.
    pub gain: Vec<(f32, f32)>,
}

impl WindLayer {
    /// The layer's level at `mph` before the profile's own `volume`: its `volume` times the curve.
    pub fn level_at(&self, mph: f32) -> f32 {
        let (Some(first), Some(last)) = (self.gain.first(), self.gain.last()) else { return 0.0 };
        if mph <= first.0 {
            return self.volume * first.1;
        }
        if mph >= last.0 {
            return self.volume * last.1;
        }
        self.volume * self.interpolate(mph)
    }

    fn interpolate(&self, mph: f32) -> f32 {
        let Some(pair) = self.gain.windows(2).find(|pair| mph <= pair[1].0) else { return 0.0 };
        pair[0].1 + (pair[1].1 - pair[0].1) * (mph - pair[0].0) / (pair[1].0 - pair[0].0)
    }
}

/// A wind variant: the loops that play with the car's speed, in file order.
#[derive(Debug, Clone, PartialEq)]
pub struct WindProfile {
    pub volume: f32,
    pub layers: Vec<WindLayer>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: u32,
    #[serde(default)]
    volume: Option<f32>,
    #[serde(default)]
    layer: Vec<RawLayer>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLayer {
    file: String,
    #[serde(default)]
    volume: Option<f32>,
    #[serde(default)]
    rate: Option<f32>,
    gain: Vec<[f32; 2]>,
}

fn field(field: &'static str, reason: &'static str) -> ContentError {
    ContentError::Field { field, reason }
}

/// Parse and check a profile. A file that is not a `.WAV` path, a number out of range, a curve that does not rise, a key the
/// schema does not know, a profile with no layer or another format number is refused with the reason.
pub fn parse(text: &str) -> Result<WindProfile, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    variant::check_format(raw.format)?;
    let volume = raw.volume.unwrap_or(1.0);
    if !(0.0..=MAX_LEVEL).contains(&volume) {
        return Err(field("volume", "the profile's volume is a number from 0 to 8"));
    }
    if raw.layer.is_empty() || raw.layer.len() > MAX_LAYERS {
        return Err(field("layer", "a wind profile has 1 to 16 layers"));
    }
    let layers = raw.layer.into_iter().map(check_layer).collect::<Result<Vec<_>, _>>()?;
    Ok(WindProfile { volume, layers })
}

fn check_layer(raw: RawLayer) -> Result<WindLayer, ContentError> {
    if !is_sound_path(&raw.file) {
        return Err(ContentError::InvalidText(raw.file));
    }
    let volume = raw.volume.unwrap_or(1.0);
    if !(0.0..=MAX_LEVEL).contains(&volume) {
        return Err(field("layer.volume", "a layer's volume is a number from 0 to 8"));
    }
    let rate = raw.rate.unwrap_or(1.0);
    if !RATES.contains(&rate) {
        return Err(field("layer.rate", "a playback rate is a number from 0.25 to 4"));
    }
    if !POINTS.contains(&raw.gain.len()) {
        return Err(field("layer.gain", "a curve has 2 to 64 points"));
    }
    let finite = raw.gain.iter().all(|point| point[0].is_finite() && point[1].is_finite());
    if !finite || raw.gain.iter().any(|point| !(0.0..=MAX_LEVEL).contains(&point[1])) {
        return Err(field("layer.gain", "a curve point's level is a number from 0 to 8"));
    }
    if raw.gain.windows(2).any(|pair| pair[0][0] >= pair[1][0]) {
        return Err(field("layer.gain", "the speeds of a curve rise"));
    }
    Ok(WindLayer { file: raw.file, volume, rate, gain: raw.gain.iter().map(|point| (point[0], point[1])).collect() })
}

#[cfg(test)]
#[path = "wind_fx_tests.rs"]
mod tests;
