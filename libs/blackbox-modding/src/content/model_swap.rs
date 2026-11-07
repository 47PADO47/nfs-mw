//! A model swap profile: which models of the game's world a mod's trees variant replaces, and with what
//! (`docs/modding/effect-variants.md`, "Profile format of the trees").
//!
//! The file is `TREES/<value id>/profile.toml` in the mod's `files/` folder, where `<value id>` is the id of the value the mod
//! adds to the `trees_variant` option (`bb.settings`). It names two pack files the mod ships in the same folder (a pack of
//! solids and a pack of textures, in the install's formats) and lists which solid of the game each model of the first
//! takes the place of. The game applies the list to every section of the world it loads while the variant is chosen: a
//! replaced solid keeps its name, its flags and its place in the section, and the model is fitted into the bounds of what
//! it replaces.
//!
//! ```toml
//! format = 1
//! geometry = "TREES/MW2012/GEOMETRY.BIN"   # the models: solids in the install's format, a path under the mod's files/
//! textures = "TREES/MW2012/TEXTURES.BIN"   # the textures the models use
//!
//! [[swap]]                                 # one per replaced solid, 1 to 4096
//! solid = "XT_CEDARM_1B_JG_00"             # the game's solid, by name as the game spells it (its hash is case-sensitive)
//! model = "MW12_CONIFER_TALL_L0"           # the solid of `geometry` that takes its place
//! fit = "height"                           # "height" (default) or "row"
//! ```
//!
//! A model's origin is its base: its height is how far it reaches above the origin. `fit = "height"` scales the model evenly to
//! the height of the solid it replaces and stands its origin on that solid's floor, at the solid's origin. `fit = "row"` is for
//! solids that are a stretch of trees: copies of the model, scaled to the replaced height but not past one and a half times
//! their own size, are set along the longer side of the replaced bounds.

use std::collections::HashSet;

use serde::Deserialize;

use super::ContentError;
use super::engine_sound::is_pack_path;
use super::variant::{self, TREES};

/// The folder under a mod's `files/` that holds the variants' files.
pub const TREES_FOLDER: &str = TREES.folder;
/// The most swaps a profile may list.
pub const MAX_SWAPS: usize = 4096;

/// The path of the profile of variant `value` in the install's layout.
pub fn model_swap_profile_path(value: &str) -> String {
    TREES.profile_path(value)
}

/// How a model is fitted into the bounds of the solid it replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    /// One copy, scaled evenly to the replaced solid's height.
    #[default]
    Height,
    /// Copies along the longer side of the replaced solid's bounds.
    Row,
}

/// One replaced solid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Swap {
    /// The name of the game's solid.
    pub solid: String,
    /// The name of the solid of the geometry pack that replaces it.
    pub model: String,
    pub fit: Fit,
}

/// A trees variant: the packs and the list of replaced solids, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSwapProfile {
    /// The pack of models, a path of the install's layout.
    pub geometry: String,
    /// The pack of textures the models use, a path of the install's layout.
    pub textures: String,
    pub swaps: Vec<Swap>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: u32,
    geometry: String,
    textures: String,
    #[serde(default)]
    swap: Vec<RawSwap>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSwap {
    solid: String,
    model: String,
    #[serde(default)]
    fit: Fit,
}

fn field(field: &'static str, reason: &'static str) -> ContentError {
    ContentError::Field { field, reason }
}

/// A solid name: letters, digits, `_`, `-`, 1 to 63 bytes.
fn is_solid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

/// Parse and check a profile. A pack that is not a `.BIN` path, a name that is not a solid name, a solid listed twice, a
/// key the schema does not know, no swap, more than 4096 swaps or another format number is refused with the reason.
pub fn parse(text: &str) -> Result<ModelSwapProfile, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    variant::check_format(raw.format)?;
    for path in [&raw.geometry, &raw.textures] {
        if !is_pack_path(path) {
            return Err(ContentError::InvalidText(path.clone()));
        }
    }
    if raw.swap.is_empty() || raw.swap.len() > MAX_SWAPS {
        return Err(field("swap", "a trees profile has 1 to 4096 swaps"));
    }
    let mut seen = HashSet::new();
    let mut swaps = Vec::with_capacity(raw.swap.len());
    for swap in raw.swap {
        if !is_solid_name(&swap.solid) || !is_solid_name(&swap.model) {
            return Err(ContentError::InvalidModel(format!("{} / {}", swap.solid, swap.model)));
        }
        if !seen.insert(swap.solid.clone()) {
            return Err(field("swap.solid", "a solid is replaced once"));
        }
        swaps.push(Swap { solid: swap.solid, model: swap.model, fit: swap.fit });
    }
    Ok(ModelSwapProfile { geometry: raw.geometry, textures: raw.textures, swaps })
}

#[cfg(test)]
#[path = "model_swap_tests.rs"]
mod tests;
