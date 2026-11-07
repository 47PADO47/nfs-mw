//! The models of a trees variant, ready to take the place of the world's solids.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use blackbox_hash::bstring_hash;
use blackbox_modding::content::{ModelSwapProfile, SwapFit};
use blackbox_solid::Solid;
use blackbox_tpk::Texture;
use game_install::GameDir;
use nfsmw_data::world::SectionData;

use super::fit::fit;

/// A replacement: the model, and how it is fitted into the solid it replaces.
struct Entry {
    model: Solid,
    fit: SwapFit,
}

/// What a fitted replacement is cached by: the replaced solid's name hash and the bits of its bounds. A solid of the game that
/// appears in many sections with the same bounds is fitted once.
type Key = (u32, [u32; 6]);

/// The replacement models of one variant, by the name hash of the solid each replaces, and the textures they use.
pub struct Pack {
    entries: HashMap<u32, Entry>,
    textures: Vec<Texture>,
    fitted: Mutex<HashMap<Key, Arc<Solid>>>,
}

impl Pack {
    /// The pack the profile describes, read through the install's layers (the mod's `files/` is one). Fails when a pack
    /// file is missing or does not parse; a swap whose model or textures are missing is left out (a warning says which).
    pub fn load(dir: &GameDir, profile: &ModelSwapProfile) -> Result<Self> {
        let geometry = dir.read(&profile.geometry).with_context(|| format!("reading {}", profile.geometry))?;
        let textures = dir.read(&profile.textures).with_context(|| format!("reading {}", profile.textures))?;
        let models =
            blackbox_solid::read_solids(&geometry).with_context(|| format!("{} does not parse", profile.geometry))?;
        let mut pictures = Vec::new();
        for pack in blackbox_tpk::read_texture_packs(&textures)
            .with_context(|| format!("{} does not parse", profile.textures))?
        {
            pictures.extend(pack.textures);
        }
        Ok(Self::build(profile, models, pictures))
    }

    /// The pack of `models` and `textures` under `profile`. A swap whose model is not among the models, or whose model uses a
    /// texture that is not among the textures, is left out with a warning.
    pub fn build(profile: &ModelSwapProfile, models: Vec<Solid>, textures: Vec<Texture>) -> Self {
        let by_name: HashMap<u32, &Solid> = models.iter().map(|model| (bstring_hash(&model.name), model)).collect();
        let mut entries = HashMap::new();
        for swap in &profile.swaps {
            let Some(model) = by_name.get(&bstring_hash(&swap.model)) else {
                log::warn!(
                    "trees: the model {} of the swap of {} is not in {}",
                    swap.model,
                    swap.solid,
                    profile.geometry
                );
                continue;
            };
            let missing = model.texture_hashes.iter().any(|hash| !textures.iter().any(|t| t.name_hash == *hash));
            if missing {
                log::warn!("trees: the model {} uses a texture that is not in {}", swap.model, profile.textures);
                continue;
            }
            entries.insert(bstring_hash(&swap.solid), Entry { model: Solid::clone(model), fit: swap.fit });
        }
        Self { entries, textures, fitted: Mutex::new(HashMap::new()) }
    }

    /// How many solids of the game this pack replaces.
    pub fn swaps(&self) -> usize {
        self.entries.len()
    }

    /// Replace the solids of a loaded section that the pack has a model for, and add the textures those models use that the
    /// section does not have. A solid whose model cannot be fitted keeps its own look (a warning says why).
    pub fn apply(&self, data: &mut SectionData) {
        let mut used: Vec<u32> = Vec::new();
        for solid in &mut data.solids {
            let Some(entry) = self.entries.get(&solid.name_hash) else { continue };
            let Some(replacement) = self.replacement(entry, solid) else { continue };
            for hash in &replacement.texture_hashes {
                if !used.contains(hash) {
                    used.push(*hash);
                }
            }
            *solid = replacement;
        }
        if used.is_empty() {
            return;
        }
        for texture in &self.textures {
            let wanted = used.contains(&texture.name_hash);
            if wanted && !data.textures.iter().any(|have| have.name_hash == texture.name_hash) {
                data.textures.push(texture.clone());
            }
        }
        data.decoded.resize(data.textures.len(), None);
    }

    /// The fitted replacement of `stock`, from the cache when its bounds were seen before.
    fn replacement(&self, entry: &Entry, stock: &Solid) -> Option<Solid> {
        let bits = |v: [f32; 3]| v.map(f32::to_bits);
        let [a, b, c] = bits(stock.bounds_min);
        let [d, e, f] = bits(stock.bounds_max);
        let key = (stock.name_hash, [a, b, c, d, e, f]);
        if let Some(found) = self.fitted.lock().ok().and_then(|cache| cache.get(&key).cloned()) {
            return Some((*found).clone());
        }
        let solid = match fit(&entry.model, stock, entry.fit) {
            Ok(solid) => solid,
            Err(error) => {
                log::warn!("trees: {} keeps its own model: {error:#}", stock.name);
                return None;
            }
        };
        if let Ok(mut cache) = self.fitted.lock() {
            cache.insert(key, Arc::new(solid.clone()));
        }
        Some(solid)
    }
}
