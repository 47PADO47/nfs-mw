//! A rain splash profile: the splashes a mod's variant makes where the rain meets the road (`docs/modding/effect-variants-marks-rain.md`).
//!
//! The file is `RAINSPLASH/<value id>/profile.toml` in the mod's `files/` folder, where `<value id>` is the id of the value the
//! mod adds to the `rain_splash_variant` option (`bb.settings`). The game's own rain draws falling streaks, the drops on the
//! lens and the spray of the tyres, and makes no splash on the road: a variant adds them. While it rains and the player is not
//! under cover, the game makes splashes at random places of the road around the player, as many a second as the layers say
//! times how hard it rains, and draws each with the layer's picture for its short life.
//!
//! ```toml
//! format = 1
//! radius = 10.0                    # metres around the player in which splashes are made, 2 to 40 (default 10)
//! density = 1.0                    # a level every layer's rate is multiplied by, 0 to 4 (default 1)
//!
//! [[layer]]                        # one to four
//! texture = "RAINSPLASH/MW2012/RING.PNG"   # an RGBA PNG; a sheet when `grid` is above 1
//! kind = "ring"                    # "ring": lies on the road and grows; "burst": stands up from the road facing the camera
//! blend = "alpha"                  # or "additive"
//! grid = 1                         # cells per side of the sheet, 1 to 8: a splash takes a random cell
//! rate = 120.0                     # splashes a second at full rain in the whole area, 0 to 2000
//! life = 0.5                       # seconds, 0.05 to 5
//! life_variance = 0.3              # how much of the life a splash may fall short of, 0 to 1
//! size = [0.1, 0.6]                # full width in metres at birth and at the end of the life, 0.01 to 20
//! opacity = 0.6                    # the greatest alpha, 0 to 1: it rises over the first fifth of the life and falls to 0
//! color = [215, 225, 240]          # red, green and blue the picture is multiplied by
//! ```

use std::ops::RangeInclusive;

use serde::Deserialize;

use super::ContentError;
use super::engine_sound::is_png_path;
use super::variant::{self, RAIN_SPLASH};

/// The folder under a mod's `files/` that holds the variants' files.
pub const RAINSPLASH_FOLDER: &str = RAIN_SPLASH.folder;
/// The most layers a profile may have.
pub const MAX_LAYERS: usize = 4;
/// The range of the radius of the area, metres.
pub const RADII: RangeInclusive<f32> = 2.0..=40.0;

/// The path of the profile of variant `value` in the install's layout.
pub fn rain_splash_profile_path(value: &str) -> String {
    RAIN_SPLASH.profile_path(value)
}

/// How a splash lies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplashKind {
    /// Flat on the road, growing.
    Ring,
    /// Standing up from the road and facing the camera.
    Burst,
}

/// One layer of splashes.
#[derive(Debug, Clone, PartialEq)]
pub struct SplashLayer {
    pub texture: String,
    pub kind: SplashKind,
    pub additive: bool,
    pub grid: u8,
    pub rate: f32,
    pub life: f32,
    pub life_variance: f32,
    /// Full width at birth and at the end of the life, metres.
    pub size: [f32; 2],
    pub opacity: f32,
    pub color: [u8; 3],
}

/// A rain splash variant.
#[derive(Debug, Clone, PartialEq)]
pub struct RainSplashProfile {
    pub radius: f32,
    pub density: f32,
    pub layers: Vec<SplashLayer>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: u32,
    radius: Option<f32>,
    density: Option<f32>,
    #[serde(default)]
    layer: Vec<RawLayer>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLayer {
    texture: String,
    kind: Option<SplashKind>,
    blend: Option<String>,
    grid: Option<u8>,
    rate: Option<f32>,
    life: Option<f32>,
    life_variance: Option<f32>,
    size: Option<[f32; 2]>,
    opacity: Option<f32>,
    color: Option<[u8; 3]>,
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

fn layer(raw: RawLayer) -> Result<SplashLayer, ContentError> {
    if !is_png_path(&raw.texture) {
        return Err(field("layer.texture", "a relative .PNG path of letters, digits, _ - . /"));
    }
    let additive = match raw.blend.as_deref() {
        None | Some("alpha") => false,
        Some("additive") => true,
        Some(_) => return Err(field("layer.blend", "\"alpha\" or \"additive\"")),
    };
    let grid = raw.grid.unwrap_or(1);
    if !(1..=8).contains(&grid) {
        return Err(field("layer.grid", "1 to 8 cells a side"));
    }
    let size = raw.size.unwrap_or([0.1, 0.6]);
    if size.iter().any(|width| !width.is_finite() || !(0.01..=20.0).contains(width)) {
        return Err(field("layer.size", "widths of 0.01 to 20 metres"));
    }
    Ok(SplashLayer {
        texture: raw.texture,
        kind: raw.kind.unwrap_or(SplashKind::Ring),
        additive,
        grid,
        rate: within(raw.rate, 60.0, &(0.0..=2000.0), "layer.rate")?,
        life: within(raw.life, 0.5, &(0.05..=5.0), "layer.life")?,
        life_variance: within(raw.life_variance, 0.3, &(0.0..=1.0), "layer.life_variance")?,
        size,
        opacity: within(raw.opacity, 0.6, &(0.0..=1.0), "layer.opacity")?,
        color: raw.color.unwrap_or([255, 255, 255]),
    })
}

/// Parse and check a profile. A number out of range, a path that is not a `.PNG` path, a key the schema does not know, a profile
/// with no layer or another format number is refused with the reason.
pub fn parse(text: &str) -> Result<RainSplashProfile, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    variant::check_format(raw.format)?;
    if raw.layer.is_empty() {
        return Err(field("layer", "a rain splash profile needs at least one layer"));
    }
    if raw.layer.len() > MAX_LAYERS {
        return Err(field("layer", "more layers than this effect takes"));
    }
    let layers = raw.layer.into_iter().map(layer).collect::<Result<Vec<_>, _>>()?;
    Ok(RainSplashProfile {
        radius: within(raw.radius, 10.0, &RADII, "radius")?,
        density: within(raw.density, 1.0, &(0.0..=4.0), "density")?,
        layers,
    })
}

#[cfg(test)]
#[path = "rain_splash_tests.rs"]
mod tests;
