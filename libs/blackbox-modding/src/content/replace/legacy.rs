//! The old `TREES/<value>/profile.toml` as a replacement set: one mechanism for both forms.
//!
//! A mod built before `bb.content` 1.2 ships a model swap profile ([`ModelSwapProfile`]) and adds a value to the game's
//! `trees_variant` option itself. The game reads that profile as it always did and turns it into the set below, so the models
//! are fitted and swapped by the same code that serves a `replace/<id>.toml`.

use super::set::{ModelEntry, ReplaceSet, Target};
use crate::content::model_swap::ModelSwapProfile;

/// The kind the old trees profile belongs to.
pub const LEGACY_KIND: &str = "trees";

/// The set the old profile of variant `value` stands for (kind `trees`, priority 0, fit as each swap says).
pub fn from_model_swap(value: &str, profile: &ModelSwapProfile) -> ReplaceSet {
    let models = profile
        .swaps
        .iter()
        .map(|swap| ModelEntry {
            targets: vec![Target::Name(swap.solid.clone())],
            model: swap.model.clone(),
            fit: swap.fit,
        })
        .collect();
    ReplaceSet {
        id: value.to_owned(),
        kind: LEGACY_KIND.to_owned(),
        kind_label: None,
        value: value.to_owned(),
        value_label: value.to_owned(),
        always: false,
        priority: 0,
        limits: profile.limits,
        geometry: Some(profile.geometry.clone()),
        textures: Some(profile.textures.clone()),
        models,
        texture_entries: Vec::new(),
        adds: Vec::new(),
        source_game: None,
    }
}
