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
//! geometry = "TREES/MYMOD/GEOMETRY.BIN"   # the models: solids in the install's format, a path under the mod's files/
//! textures = "TREES/MYMOD/TEXTURES.BIN"   # the textures the models use
//!
//! [[swap]]                                 # one per replaced solid, 1 to 4096
//! solid = "TALL_TREE_1B"             # the game's solid, by name as the game spells it (its hash is case-sensitive)
//! model = "TALL_TREE_L0"           # the solid of `geometry` that takes its place
//! fit = "height"                           # "height" (default) or "row"
//! ```
//!
//! A model's origin is its base: its height is how far it reaches above the origin. `fit = "height"` scales the model to the
//! height of the solid it replaces and stands its origin on that solid's floor, at the solid's origin. `fit = "row"` is for
//! solids that are a stretch of trees: copies of the model, scaled to the replaced height but not past one and a half times
//! their own size, are set where the replaced solid's own geometry is, one for each stretch of it, at the floor of that stretch.
//! An optional `[fit]` table sets the limits ([`FitLimits`]): `spread`, `squeeze`, `min_height`, `clearance` and `settle`.

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
    /// One copy scaled evenly to fit **inside** the replaced solid's box (its footprint and its height, whichever binds first),
    /// centred on the box with its lowest point on the box's floor: for lamps, signs, barriers and buildings, which stand on
    /// the ground and have a size of their own. In a world-space copy it is cut into stretches like `height`.
    Stand,
    /// The model as authored, in the frame of the solid it replaces (a mod that models to the old solid's origin). It is used
    /// only when its box stays inside the old box grown by the spread; otherwise the solid keeps its own model.
    Exact,
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

/// The limits a replacement keeps against the solid it replaces (the `[fit]` table of the profile, every key optional).
///
/// A replacement is never grown past the footprint of the solid it takes the place of: its width is held to `spread` times
/// the old one, squeezing it sideways down to `squeeze` of its own proportions if it has to, and a tree that would still be too
/// wide is made smaller; one that ends up shorter than `min_height` of the replaced height is not used (the solid keeps the
/// game's own model).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FitLimits {
    /// How much wider than the replaced solid a replacement may be, as a factor (1 to 4).
    pub spread: f32,
    /// The least a replacement's width is, as a share of its height scale, when it is squeezed to fit (0.1 to 1).
    pub squeeze: f32,
    /// The least share of the replaced height a replacement may have after being made smaller to fit (0 to 1).
    pub min_height: f32,
    /// Whether the copies of a replacement keep clear of the solid objects next to them (walls, guard rails, barriers, buildings): a
    /// copy that would reach into one is made smaller, down to the footprint it replaces, or left out. For low growth (bushes, grass,
    /// hedges); a tree's crown may overhang a rail, so it is off by default.
    pub clearance: bool,
    /// Whether the copies of a stretch of trees (a world-space copy of the game's solid) are **settled** on the real ground and off the road:
    /// each foot is put on the terrain under it (a crown that hovered over its stretch's lowest point comes down, one that sank comes up), a
    /// copy on the carriageway is moved off it or left out, and a crown that hangs low over a road is narrowed or left out. For trees; off by
    /// default (`docs/modding/api-replace.md`, "Settling").
    pub settle: bool,
}

impl Default for FitLimits {
    fn default() -> Self {
        Self { spread: 1.25, squeeze: 0.5, min_height: 0.7, clearance: false, settle: false }
    }
}

/// A trees variant: the packs and the list of replaced solids, in file order.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelSwapProfile {
    /// The pack of models, a path of the install's layout.
    pub geometry: String,
    /// The pack of textures the models use, a path of the install's layout.
    pub textures: String,
    /// The limits the fit keeps ([`FitLimits`]); the defaults when the profile has no `[fit]` table.
    pub limits: FitLimits,
    pub swaps: Vec<Swap>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: u32,
    geometry: String,
    textures: String,
    #[serde(default)]
    fit: RawLimits,
    #[serde(default)]
    swap: Vec<RawSwap>,
}

/// The `[fit]` table as serde reads it (shared with the replacement sets, `replace/`).
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(super) struct RawLimits {
    spread: Option<f32>,
    squeeze: Option<f32>,
    min_height: Option<f32>,
    clearance: Option<bool>,
    settle: Option<bool>,
}

impl RawLimits {
    /// The limits with the defaults filled in, or the refused key.
    pub(super) fn check(&self) -> Result<FitLimits, ContentError> {
        let defaults = FitLimits::default();
        let spread = self.spread.unwrap_or(defaults.spread);
        let squeeze = self.squeeze.unwrap_or(defaults.squeeze);
        let min_height = self.min_height.unwrap_or(defaults.min_height);
        if !(1.0..=4.0).contains(&spread) {
            return Err(field("fit.spread", "1 to 4"));
        }
        if !(0.1..=1.0).contains(&squeeze) {
            return Err(field("fit.squeeze", "0.1 to 1"));
        }
        if !(0.0..=1.0).contains(&min_height) {
            return Err(field("fit.min_height", "0 to 1"));
        }
        Ok(FitLimits {
            spread,
            squeeze,
            min_height,
            clearance: self.clearance.unwrap_or(defaults.clearance),
            settle: self.settle.unwrap_or(defaults.settle),
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSwap {
    solid: String,
    model: String,
    #[serde(default)]
    fit: Fit,
}

pub(super) fn field(field: &'static str, reason: &'static str) -> ContentError {
    ContentError::Field { field, reason }
}

/// A solid name: letters, digits, `_`, `-`, 1 to 63 bytes.
pub(super) fn is_solid_name(name: &str) -> bool {
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
    let limits = raw.fit.check()?;
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
    Ok(ModelSwapProfile { geometry: raw.geometry, textures: raw.textures, limits, swaps })
}

#[cfg(test)]
#[path = "model_swap_tests.rs"]
mod tests;
