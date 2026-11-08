//! Replacement sets: the one way a mod replaces or adds models and textures of a game's world (`bb.content` 1.2,
//! `docs/modding/api-replace.md`, `docs/specs/mod-content-replacement.md`).
//!
//! A *set* is one file, `replace/<id>.toml` in the mod's folder, with the models and textures it takes the place of (or adds)
//! and the files that hold them. It belongs to a *kind* (`trees`, `bushes`, `lamps`: any name the mod picks), and the kinds are
//! the option groups of the game's settings: each kind has one list option, `<kind>_variant`, whose values are the sets the
//! mods offer and the game's own `original`. A set marked `always` has no option and applies whenever its mod is loaded.
//!
//! This module checks the schema and the values: names, numbers, paths. Whether a target exists in the game, whether the model
//! is in the geometry file, and how a model is fitted are for the game; the lib has no game names in it.
//!
//! ```toml
//! format = 1
//!
//! [set]
//! kind = "trees"                     # the option group; option id `trees_variant`
//! value = "mw2012"                   # this set's value of the option (default: the file name)
//! value_label = "NFS Most Wanted 2012"
//! priority = 0                       # -1000 to 1000: who wins when two active sets replace the same thing
//! fit = "height"                     # the default fit of the models: height, row, stand or exact
//!
//! [files]
//! geometry = "TREES/MW2012/GEOMETRY.BIN"   # the models, in the install's format
//! textures = "TREES/MW2012/TEXTURES.BIN"   # the textures the models use (added to the world once)
//!
//! [fit]                              # optional limits (spread, squeeze, min_height)
//!
//! [[model]]                          # replace the game's models
//! targets = ["XT_CEDARM_1B_JG_00", "XT_CEDARM_1A_JG_00"]
//! model = "MW12_CONIFER_TALL_L0"
//!
//! [[texture]]                        # replace a texture of the game (by name or hash), or add one (`name`)
//! target = "ROAD_ASPHALT_01"
//! file = "TEXTURES/ROAD.png"
//!
//! [[add]]                            # place a model of the geometry file in the world
//! model = "MW12_BENCH"
//! pos = [120.0, -40.0, 3.0]
//! ```

mod legacy;
mod set;
#[cfg(test)]
mod tests;
mod texture;

pub use legacy::{LEGACY_KIND, from_model_swap};
pub use set::{MAX_ENTRIES, MAX_PRIORITY, ModelEntry, REPLACE_FOLDER, ReplaceSet, Target, parse};
pub use texture::{Convert, StateRule, TextureEntry, TextureSource};

/// The longest label of an option or a value, in bytes.
pub(super) const MAX_LABEL_BYTES: usize = 64;
