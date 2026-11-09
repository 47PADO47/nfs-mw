//! The sky of the Chameleon engine (NFS Most Wanted 2012): the time-of-day **timelines** of `HAWAII/ENVIRONMENT.BNDL`, the cloud
//! dome each keyframe names, the colours it lights them with, and the dome and moon pictures. Format and evidence:
//! `docs/formats/chameleon-environment.md`.
//!
//! Everything takes a parsed [`Bundle`] (the caller decides how its bytes are read, whole or by ranges) and finds fields by
//! offset: the field names of Genesys types are hashed, so what a number means is a reading, marked in the docs as such. The
//! structures inside an object are found by the **name of their type** (`Genesys.Gen.EnvironmentKeyframe.Clouds`), which the
//! bundle's `GenesysType` resources spell out, never by a hash.

mod read;
#[cfg(test)]
mod tests;

use chameleon_bundle::{Bundle, ResourceId};

pub use read::{Error, Result};

/// Where the bundle is under the install.
pub const BUNDLE_PATH: &str = "HAWAII/ENVIRONMENT.BNDL";

/// The colours and the picture a keyframe lights its clouds with (`Genesys.Gen.EnvironmentKeyframe.Clouds`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clouds {
    /// The key of the dome texture (`+0x30`, a `CgsCore.UniqueId`), a resource of the bundle (class 1).
    pub texture: u32,
    /// The first colour (`+0x10`, linear RGB and 1): 0 at night, 1.74 grey at sunrise, bluish above 1.6 at noon. Read as the colour
    /// of the sunlit clouds **[unconfirmed]**.
    pub lit: [f32; 4],
    /// The second colour (`+0x20`): 0.02 at night, 0.77 at sunrise, 0.47 in the evening. Read as the colour of the clouds in shade
    /// **[unconfirmed]**.
    pub ambient: [f32; 4],
    /// The seven numbers from `+0x38` (0.5 or 1.0, 0.186 or 0.21, 0.569 or 0.193, 5.0 or 3.5, ...): meaning not decoded.
    pub params: [f32; 7],
}

impl Clouds {
    /// The colour the dome's pixels are multiplied by: the mean of the two colours **[a choice, not the engine's rule]**.
    pub fn tint(&self) -> [f32; 3] {
        std::array::from_fn(|i| 0.5 * (self.lit[i] + self.ambient[i]))
    }
}

/// One keyframe of a timeline: the hour it holds from and its clouds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    /// The hour, 0 to 23.
    pub hour: f32,
    /// The key of the keyframe object (`Genesys.Gen.EnvironmentKeyframe`, class 1).
    pub keyframe: u32,
    pub clouds: Clouds,
}

/// A day: the keyframes the game blends between by the hour (`Genesys.Gen.EnvironmentTimeline`).
#[derive(Debug, Clone, PartialEq)]
pub struct Timeline {
    /// The key of the timeline object.
    pub id: u32,
    pub keys: Vec<Key>,
}

/// The id of a numbered resource of the bundle (class 1) from its key.
pub const fn numbered(key: u32) -> u64 {
    ResourceId::from_parts(key, 0, 0, 1).0
}

/// Every timeline of the bundle with its keyframes' clouds, in the order of the bundle's table.
pub fn timelines(bundle: &Bundle<'_>) -> Result<Vec<Timeline>> {
    read::timelines(bundle)
}

/// The dome, moon or noise texture with key `key` of the bundle.
pub fn texture(bundle: &Bundle<'_>, key: u32) -> Result<chameleon_gfx::Texture> {
    read::texture(bundle, numbered(key))
}

/// The key of the texture of the moon: the one the moon material (`Material`, resource type 0x02) imports.
pub fn moon_texture(bundle: &Bundle<'_>) -> Result<u32> {
    read::moon_texture(bundle)
}

impl Timeline {
    /// The keyframes around `hour` and how far it is from the first to the second (0 to 1): the last key at or before the hour
    /// and the next one, wrapping past midnight. A timeline with one key holds it.
    pub fn around(&self, hour: f32) -> Option<(&Key, &Key, f32)> {
        let first = self.keys.first()?;
        let hour = hour.rem_euclid(24.0);
        let before =
            self.keys.iter().rev().find(|key| key.hour <= hour).unwrap_or_else(|| self.keys.last().unwrap_or(first));
        let after_index = self.keys.iter().position(|key| key.hour > hour).unwrap_or(0);
        let after = &self.keys[after_index];
        let span = (after.hour - before.hour).rem_euclid(24.0);
        if span <= f32::EPSILON {
            return Some((before, before, 0.0));
        }
        let into = (hour - before.hour).rem_euclid(24.0);
        Some((before, after, (into / span).clamp(0.0, 1.0)))
    }
}
