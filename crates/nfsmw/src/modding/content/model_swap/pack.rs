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

use super::fit::{Frame, Prepared, fit_prepared};

/// What the name of a world-space copy of a solid has between the solid's name and the number of the copy
/// (`<NAME>_DEINST<n>_<tile>`).
const WORLD_COPY: &str = "_DEINST";

/// A replacement: the model, made ready once for every solid it takes the place of, and how it is fitted.
struct Entry {
    model: Arc<Prepared>,
    fit: SwapFit,
}

/// What a fitted replacement is cached by: the replaced solid's name hash and the bits of its bounds. A solid of the game that
/// appears in many sections with the same bounds is fitted once.
type Key = (u32, [u32; 6]);

/// The replacement models of one variant, by the name hash of the solid each replaces, and the textures they use.
pub struct Pack {
    entries: HashMap<u32, Entry>,
    textures: Vec<Texture>,
    pub(super) fitted: Mutex<HashMap<Key, Arc<Solid>>>,
}

/// The name of the solid a world-space copy was made of: `XT_CEDARM_1B_JG_00` for `XT_CEDARM_1B_JG_00_DEINST12_T5`. `None` for a
/// name that is not a copy's.
pub(super) fn world_copy_base(name: &str) -> Option<&str> {
    let at = name.find(WORLD_COPY)?;
    let number = name[at + WORLD_COPY.len()..].chars().next()?;
    number.is_ascii_digit().then(|| &name[..at])
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

    /// The pack of `models` and `textures` under `profile`. A swap whose model is not among the models or cannot be fitted at
    /// all, or whose model uses a texture that is not among the textures, is left out with a warning.
    pub fn build(profile: &ModelSwapProfile, models: Vec<Solid>, textures: Vec<Texture>) -> Self {
        let by_name: HashMap<u32, &Solid> = models.iter().map(|model| (bstring_hash(&model.name), model)).collect();
        let mut prepared: HashMap<u32, Option<Arc<Prepared>>> = HashMap::new();
        let mut entries = HashMap::new();
        for swap in &profile.swaps {
            let model_hash = bstring_hash(&swap.model);
            let Some(model) = by_name.get(&model_hash) else {
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
            let ready = prepared.entry(model_hash).or_insert_with(|| match Prepared::new(model) {
                Ok(ready) => Some(Arc::new(ready)),
                Err(error) => {
                    log::warn!("trees: {error:#}");
                    None
                }
            });
            let Some(ready) = ready.clone() else { continue };
            entries.insert(bstring_hash(&swap.solid), Entry { model: ready, fit: swap.fit });
        }
        Self { entries, textures, fitted: Mutex::new(HashMap::new()) }
    }

    /// How many solids of the game this pack replaces.
    pub fn swaps(&self) -> usize {
        self.entries.len()
    }

    /// The textures of the pack's models. They are not copied into the sections the pack rewrites: the world puts them in the
    /// shared sets once, and every section's solids find them there.
    pub fn textures(&self) -> &[Texture] {
        &self.textures
    }

    /// Replace the solids of a loaded section that the pack has a model for: the game's own solids by name, and the world-space
    /// copies of them (`_DEINST` names) by the name of the solid they are a copy of. A solid whose model cannot be fitted keeps
    /// its own look (a warning says why). The textures the models use are not added to the section ([`Pack::textures`]).
    pub fn apply(&self, data: &mut SectionData) {
        for solid in &mut data.solids {
            let Some((entry, frame)) = self.entry_for(solid) else { continue };
            let Some(replacement) = self.replacement(entry, solid, frame) else { continue };
            *solid = replacement;
        }
    }

    /// The entry that replaces `solid`, and the space the solid is in.
    fn entry_for(&self, solid: &Solid) -> Option<(&Entry, Frame)> {
        if let Some(entry) = self.entries.get(&solid.name_hash) {
            return Some((entry, Frame::Local));
        }
        let base = world_copy_base(&solid.name)?;
        let entry = self.entries.get(&bstring_hash(base))?;
        Some((entry, Frame::World))
    }

    /// The fitted replacement of `stock`. A local solid is cached by its bounds (the game uses one solid in many places with the
    /// same bounds); a world copy is its own, so it is fitted every time and nothing of it is kept.
    fn replacement(&self, entry: &Entry, stock: &Solid, frame: Frame) -> Option<Solid> {
        if frame == Frame::World {
            return self.fit_now(entry, stock, frame);
        }
        let bits = |v: [f32; 3]| v.map(f32::to_bits);
        let [a, b, c] = bits(stock.bounds_min);
        let [d, e, f] = bits(stock.bounds_max);
        let key = (stock.name_hash, [a, b, c, d, e, f]);
        if let Some(found) = self.fitted.lock().ok().and_then(|cache| cache.get(&key).cloned()) {
            return Some((*found).clone());
        }
        let solid = self.fit_now(entry, stock, frame)?;
        if let Ok(mut cache) = self.fitted.lock() {
            cache.insert(key, Arc::new(solid.clone()));
        }
        Some(solid)
    }

    fn fit_now(&self, entry: &Entry, stock: &Solid, frame: Frame) -> Option<Solid> {
        match fit_prepared(&entry.model, stock, entry.fit, frame) {
            Ok(solid) => Some(solid),
            Err(error) => {
                log::warn!("trees: {} keeps its own model: {error:#}", stock.name);
                None
            }
        }
    }
}
