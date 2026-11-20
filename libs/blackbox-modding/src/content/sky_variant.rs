//! A sky profile: the cloud panoramas, the moon picture and the days a mod's sky variant draws (`docs/modding/sky-variants.md`).
//!
//! The file is `SKY/<value id>/profile.toml` in the mod's `files/` folder, where `<value id>` is the id of the value the mod's
//! replacement set of kind `sky` adds to the `sky_variant` option. A profile names **domes**: equirectangular pictures of the
//! upper hemisphere (the full width is 360 degrees, the top row the zenith, the bottom row the horizon), colour and cloud cover
//! in the alpha. A **weather** is a day: the hour on which the game switches from one dome to the next, with the colour each is
//! lit with. The game picks the weather by the cloud cover now, blends the two keys around the hour, and draws the result as the
//! clouds of its procedural sky.
//!
//! ```toml
//! format = 1
//! exposure = 0.55                  # a multiplier of the clouds' light, 0.05 to 4 (default 1)
//! wind = 0.01                      # turns of the sky a unit of the game's cloud scroll moves the domes (0 to 1; default 0.01)
//!
//! [moon]                           # optional
//! file = "SKY/MW2012/MOON.DDS"     # a DDS (BC1, BC3 or uncompressed) or PNG picture; the disc fills 3/4 of its width
//! size = 0.0173                    # half the picture's width as an angle in radians, 0.002 to 0.1
//! brightness = 1.0                 # 0 to 4
//!
//! [[dome]]
//! name = "day"                     # a-z 0-9 _ -
//! file = "SKY/MW2012/DAY.DDS"
//! glow = 0.5                       # optional: where along the picture (0 to 1) the sun is painted; the dome is turned so
//!                                  # that point is under the game's sun
//!
//! [[weather]]                      # one to eight, by the cloud cover they take up to
//! max_cover = 0.35                 # 0 to 1: used while the cover is at most this; the last one takes the rest
//! gain = 1.0                       # a multiplier of the domes' cloud cover (alpha), 0.1 to 4 (default 1)
//!
//! [[weather.key]]                  # one to 48, hours rising, 0 to 24 (the day wraps: the last blends into the first)
//! hour = 5.0
//! dome = "day"
//! tint = [1.2, 1.2, 1.2]           # what the dome's pixels are multiplied by (linear), 0 to 8 each
//! ```

use serde::Deserialize;

use super::ContentError;
use super::variant::{self, VariantKind};

/// The sky variants: the option is the one the replacement sets of kind `sky` create.
pub const SKY: VariantKind = VariantKind { option: "sky_variant", folder: "SKY" };
/// The folder under a mod's `files/` that holds the variants' files.
pub const SKY_FOLDER: &str = SKY.folder;
/// Most domes, weathers and keys of a weather a profile may have.
pub const MAX_DOMES: usize = 16;
pub const MAX_WEATHERS: usize = 8;
pub const MAX_KEYS: usize = 48;

/// The path of the profile of variant `value` in the install's layout.
pub fn sky_profile_path(value: &str) -> String {
    SKY.profile_path(value)
}

/// A picture of the sky.
#[derive(Debug, Clone, PartialEq)]
pub struct Dome {
    pub name: String,
    pub file: String,
    /// Where along the picture the sun is painted (0 to 1), when the picture has one.
    pub glow: Option<f32>,
}

/// The moon's picture.
#[derive(Debug, Clone, PartialEq)]
pub struct Moon {
    pub file: String,
    pub size: f32,
    pub brightness: f32,
}

/// One key of a day: from `hour` the dome `dome` (an index of the profile's domes), lit by `tint`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    pub hour: f32,
    pub dome: usize,
    pub tint: [f32; 3],
}

/// A day for a range of cloud cover.
#[derive(Debug, Clone, PartialEq)]
pub struct Weather {
    pub max_cover: f32,
    pub gain: f32,
    pub keys: Vec<Key>,
}

/// A sky variant.
#[derive(Debug, Clone, PartialEq)]
pub struct SkyProfile {
    pub exposure: f32,
    pub wind: f32,
    pub moon: Option<Moon>,
    pub domes: Vec<Dome>,
    pub weathers: Vec<Weather>,
}

/// What the profile says the sky is at one hour and cloud cover: two domes, how far the first has passed into the second, and
/// what each is lit with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub dome: [usize; 2],
    pub blend: f32,
    pub tint: [[f32; 3]; 2],
    pub gain: f32,
}

impl SkyProfile {
    /// The weather for a cloud cover: the first, in the order of the file, whose `max_cover` is at least the cover; the last
    /// when none is.
    pub fn weather(&self, cover: f32) -> Option<&Weather> {
        let cover = match cover.is_finite() {
            true => cover.clamp(0.0, 1.0),
            false => 0.0,
        };
        self.weathers.iter().find(|weather| weather.max_cover >= cover).or(self.weathers.last())
    }

    /// The sky at `hour` (any number, taken modulo 24) with the cloud `cover` (0 to 1).
    pub fn pose(&self, hour: f32, cover: f32) -> Option<Pose> {
        let weather = self.weather(cover)?;
        let hour = match hour.is_finite() {
            true => hour.rem_euclid(24.0),
            false => 12.0,
        };
        let keys = &weather.keys;
        let before = keys.iter().rev().find(|key| key.hour <= hour).or(keys.last())?;
        let after = keys.iter().find(|key| key.hour > hour).or(keys.first())?;
        let span = (after.hour - before.hour).rem_euclid(24.0);
        let blend = match span > f32::EPSILON {
            true => ((hour - before.hour).rem_euclid(24.0) / span).clamp(0.0, 1.0),
            false => 0.0,
        };
        Some(Pose { dome: [before.dome, after.dome], blend, tint: [before.tint, after.tint], gain: weather.gain })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: u32,
    #[serde(default)]
    exposure: Option<f32>,
    #[serde(default)]
    wind: Option<f32>,
    #[serde(default)]
    moon: Option<RawMoon>,
    #[serde(default)]
    dome: Vec<RawDome>,
    #[serde(default)]
    weather: Vec<RawWeather>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMoon {
    file: String,
    #[serde(default)]
    size: Option<f32>,
    #[serde(default)]
    brightness: Option<f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDome {
    name: String,
    file: String,
    #[serde(default)]
    glow: Option<f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWeather {
    max_cover: f32,
    #[serde(default)]
    gain: Option<f32>,
    #[serde(default)]
    key: Vec<RawKey>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawKey {
    hour: f32,
    dome: String,
    tint: [f32; 3],
}

fn field(field: &'static str, reason: &'static str) -> ContentError {
    ContentError::Field { field, reason }
}

/// A relative path of the install's layout that names a `.DDS` or `.PNG` picture: letters, digits, `_`, `-`, `.` and `/`, no `..`.
fn is_picture_path(path: &str) -> bool {
    let safe = path.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/'));
    let parts_ok = path.split('/').all(|part| !part.is_empty() && part != "." && part != "..");
    let lower = path.to_ascii_lowercase();
    safe && parts_ok && path.len() <= 96 && (lower.ends_with(".dds") || lower.ends_with(".png"))
}

fn is_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && name.bytes().all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-'))
}

/// Parse and check a profile: the format number, the ranges of every number, the pictures' paths, that every key names a dome of
/// the profile and that hours rise.
pub fn parse(text: &str) -> Result<SkyProfile, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    variant::check_format(raw.format)?;
    let exposure = raw.exposure.unwrap_or(1.0);
    if !(0.05..=4.0).contains(&exposure) {
        return Err(field("exposure", "the exposure is a number from 0.05 to 4"));
    }
    let wind = raw.wind.unwrap_or(0.01);
    if !(0.0..=1.0).contains(&wind) {
        return Err(field("wind", "the wind is a number from 0 to 1"));
    }
    let moon = raw.moon.map(check_moon).transpose()?;
    if raw.dome.is_empty() || raw.dome.len() > MAX_DOMES {
        return Err(field("dome", "a sky profile has 1 to 16 domes"));
    }
    let domes = raw.dome.into_iter().map(check_dome).collect::<Result<Vec<_>, _>>()?;
    if (1..domes.len()).any(|at| domes[..at].iter().any(|earlier| earlier.name == domes[at].name)) {
        return Err(field("dome.name", "a dome's name is used once"));
    }
    if raw.weather.is_empty() || raw.weather.len() > MAX_WEATHERS {
        return Err(field("weather", "a sky profile has 1 to 8 weathers"));
    }
    let weathers =
        raw.weather.into_iter().map(|weather| check_weather(weather, &domes)).collect::<Result<Vec<_>, _>>()?;
    Ok(SkyProfile { exposure, wind, moon, domes, weathers })
}

fn check_moon(raw: RawMoon) -> Result<Moon, ContentError> {
    if !is_picture_path(&raw.file) {
        return Err(ContentError::InvalidText(raw.file));
    }
    let size = raw.size.unwrap_or(0.0173);
    if !(0.002..=0.1).contains(&size) {
        return Err(field("moon.size", "the moon's size is an angle from 0.002 to 0.1 radians"));
    }
    let brightness = raw.brightness.unwrap_or(1.0);
    if !(0.0..=4.0).contains(&brightness) {
        return Err(field("moon.brightness", "the moon's brightness is a number from 0 to 4"));
    }
    Ok(Moon { file: raw.file, size, brightness })
}

fn check_dome(raw: RawDome) -> Result<Dome, ContentError> {
    if !is_name(&raw.name) {
        return Err(ContentError::InvalidText(raw.name));
    }
    if !is_picture_path(&raw.file) {
        return Err(ContentError::InvalidText(raw.file));
    }
    if raw.glow.is_some_and(|glow| !(0.0..=1.0).contains(&glow)) {
        return Err(field("dome.glow", "a glow is a position along the picture from 0 to 1"));
    }
    Ok(Dome { name: raw.name, file: raw.file, glow: raw.glow })
}

fn check_weather(raw: RawWeather, domes: &[Dome]) -> Result<Weather, ContentError> {
    if !(0.0..=1.0).contains(&raw.max_cover) {
        return Err(field("weather.max_cover", "a cloud cover is a number from 0 to 1"));
    }
    let gain = raw.gain.unwrap_or(1.0);
    if !(0.1..=4.0).contains(&gain) {
        return Err(field("weather.gain", "a gain is a number from 0.1 to 4"));
    }
    if raw.key.is_empty() || raw.key.len() > MAX_KEYS {
        return Err(field("weather.key", "a weather has 1 to 48 keys"));
    }
    let mut keys = Vec::with_capacity(raw.key.len());
    for key in raw.key {
        if !(0.0..24.0).contains(&key.hour) {
            return Err(field("weather.key.hour", "an hour is a number from 0 up to 24"));
        }
        let Some(dome) = domes.iter().position(|dome| dome.name == key.dome) else {
            return Err(field("weather.key.dome", "a key names a dome of the profile"));
        };
        if key.tint.iter().any(|value| !(0.0..=8.0).contains(value)) {
            return Err(field("weather.key.tint", "a tint is three numbers from 0 to 8"));
        }
        keys.push(Key { hour: key.hour, dome, tint: key.tint });
    }
    if keys.windows(2).any(|pair| pair[0].hour >= pair[1].hour) {
        return Err(field("weather.key.hour", "the hours of a weather rise"));
    }
    Ok(Weather { max_cover: raw.max_cover, gain, keys })
}

#[cfg(test)]
#[path = "sky_variant_tests.rs"]
mod tests;
