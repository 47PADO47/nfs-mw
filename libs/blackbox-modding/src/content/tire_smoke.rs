//! A tire smoke profile: what a mod's tire smoke variant looks like (`docs/modding/effect-variants.md`).
//!
//! The file is `TIRESMOKE/<value id>/profile.toml` in the mod's `files/` folder, where `<value id>` is the id of the value the
//! mod adds to the `tire_smoke_variant` option (`bb.settings`). A profile is a list of particle layers: each is one emitter
//! at every wheel that smokes, in the frame of the road under it (the local `z` axis is the road's normal, up), drawn with the
//! mod's own picture. A layer has the keys of a nitrous `[[effect]]` (`nitrous_effects`) except `on`, and its rate is the
//! number of particles a second **at full smoke** (the game's own wheel smoke intensity, 0 to 1, multiplies it).
//!
//! ```toml
//! format = 1
//!
//! [[layer]]
//! texture = "TIRESMOKE/MW2012/PUFF.PNG"
//! blend = "alpha"               # or "additive"; the default is additive, so tire smoke says it
//! grid = 2                      # cells per side of the picture (a random cell for each particle with fps = 0)
//! random_start = true
//! fps = 0
//! rate = 18.0                   # particles a second at full smoke, and for each m/s the car goes
//! rate_per_speed = 2.0
//! life = 1.0
//! size = [0.5, 1.5, 2.5]        # full width in metres at each key
//! keys = [0.0, 0.15, 1.0]
//! colors = [[200, 205, 220, 0], [200, 205, 220, 80], [150, 155, 170, 0]]
//! ```

use serde::Deserialize;

use super::ContentError;
use super::nitrous_effects::{self as effects, EffectProfile, RawEffect};
use super::variant::{self, TIRE_SMOKE};

/// The folder under a mod's `files/` that holds the variants' files.
pub const TIRESMOKE_FOLDER: &str = TIRE_SMOKE.folder;
/// The most layers a profile may have.
pub const MAX_LAYERS: usize = 8;

/// The path of the profile of variant `value` in the install's layout.
pub fn tire_smoke_profile_path(value: &str) -> String {
    TIRE_SMOKE.profile_path(value)
}

/// A tire smoke variant: the layers drawn at each smoking wheel, in file order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TireSmokeProfile {
    pub layers: Vec<EffectProfile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: u32,
    #[serde(default)]
    layer: Vec<RawEffect>,
}

/// Parse and check a profile. A number out of range, a path that is not a `.PNG` path, a key the schema does not know, a
/// profile with no layer or another format number is refused with the reason.
pub fn parse(text: &str) -> Result<TireSmokeProfile, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    variant::check_format(raw.format)?;
    let layers = effects::check_layers(raw.layer, MAX_LAYERS)?;
    if layers.is_empty() {
        return Err(ContentError::Field { field: "layer", reason: "a tire smoke profile needs at least one layer" });
    }
    Ok(TireSmokeProfile { layers })
}

#[cfg(test)]
#[path = "tire_smoke_tests.rs"]
mod tests;
