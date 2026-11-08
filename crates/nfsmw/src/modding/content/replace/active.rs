//! The replacements in effect: what the chosen sets, together, take the place of (`docs/specs/mod-content-replacement.md`).
//!
//! [`Active::build`] merges the loaded sets in the order the conflict rule gives (lowest priority first, then load order; the last
//! to name a target wins and the clash is logged and kept for the report), makes every model ready to be fitted once, and keeps the
//! textures. The loader threads then call [`Active::apply`] on each section they parse.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use blackbox_hash::bstring_hash;
use blackbox_modding::content::{FitLimits, Place, ReplaceSet, ReplaceTarget, SwapFit};
use blackbox_scenery::ScenerySection;
use blackbox_solid::Solid;
use blackbox_tpk::Texture;
use nfsmw_data::world::SectionData;

use super::fit::{Frame, Prepared, fit_prepared};
use super::textures::TextureSlot;
use crate::scenes::culling::is_mirrored;

/// What the name of a world-space copy of a solid has between the solid's name and the number of the copy
/// (`<NAME>_DEINST<n>_<tile>`).
const WORLD_COPY: &str = "_DEINST";

/// A set read from its files, ready to be merged.
pub struct LoadedSet {
    /// The mod that provides it, and where that mod is in the load order.
    pub owner: String,
    pub order: usize,
    pub set: Arc<ReplaceSet>,
    /// The solids of the set's geometry file.
    pub models: Vec<Solid>,
    /// The textures added to the world by this set (its pack's, and the `name = ...` entries), by the names the models use.
    pub added: Vec<Texture>,
    /// The textures this set replaces, by the hash of the game's texture.
    pub replaced: Vec<(u32, Arc<TextureSlot>)>,
}

/// Objects a set places in the world (`[[add]]`): they become map packs when a world opens (`adds.rs`).
pub struct Added {
    pub owner: String,
    pub set: String,
    /// The set's geometry file, a path of the install's layout.
    pub geometry: String,
    /// The names of the solids in it, in upper case.
    pub models: HashSet<String>,
    pub places: Vec<Place>,
}

/// A replacement model: made ready once, and how it is fitted.
struct Entry {
    model: Arc<Prepared>,
    fit: SwapFit,
    limits: FitLimits,
}

/// What a fitted replacement is cached by: the replaced solid's name hash and the bits of its bounds. A solid of the game that
/// appears in many sections with the same bounds is fitted once.
type Key = (u32, [u32; 6]);

/// The replacements in effect.
pub struct Active {
    models: HashMap<u32, Entry>,
    pub(super) fitted: Mutex<HashMap<Key, Arc<Solid>>>,
    textures: HashMap<u32, Arc<TextureSlot>>,
    added: Vec<Texture>,
    objects: Vec<Added>,
    /// The sets in effect, as `mod:set`, in the order they were merged.
    sets: Vec<String>,
    /// One line for every target two sets named, saying which won.
    conflicts: Vec<String>,
}

/// The name of the solid a world-space copy was made of: `XT_CEDARM_1B_JG_00` for `XT_CEDARM_1B_JG_00_DEINST12_T5`. `None` for a
/// name that is not a copy's.
pub(super) fn world_copy_base(name: &str) -> Option<&str> {
    let at = name.find(WORLD_COPY)?;
    let number = name[at + WORLD_COPY.len()..].chars().next()?;
    number.is_ascii_digit().then(|| &name[..at])
}

/// The name hashes of the solids that a mirrored instance of the sections draws (their instances carry the mirror flag).
fn mirrored_solids(sections: &[ScenerySection]) -> HashSet<u32> {
    let mut hashes = HashSet::new();
    for section in sections {
        for instance in section.instances.iter().filter(|instance| is_mirrored(instance.exclude_flags)) {
            let Some(info) = section.info_of(instance) else { continue };
            hashes.extend(info.solid_keys.iter().copied());
        }
    }
    hashes
}

/// The hash the game keys a target by.
pub fn target_hash(target: &ReplaceTarget) -> u32 {
    match target {
        ReplaceTarget::Name(name) => bstring_hash(name),
        ReplaceTarget::Hash(hash) => *hash,
    }
}

impl Active {
    /// Nothing replaced.
    pub fn empty() -> Self {
        Self {
            models: HashMap::new(),
            fitted: Mutex::new(HashMap::new()),
            textures: HashMap::new(),
            added: Vec::new(),
            objects: Vec::new(),
            sets: Vec::new(),
            conflicts: Vec::new(),
        }
    }

    /// The replacements of `loaded`, merged. A model that is not in its set's geometry file, uses a texture the set's pack does
    /// not have, or cannot be made ready is left out with a warning; the rest of the set stands.
    pub fn build(mut loaded: Vec<LoadedSet>) -> Self {
        loaded.sort_by_key(|set| (set.set.priority, set.order, set.set.id.clone()));
        let mut active = Self::empty();
        let mut model_owner: HashMap<u32, String> = HashMap::new();
        let mut texture_owner: HashMap<u32, String> = HashMap::new();
        for input in loaded {
            let label = format!("{}:{}", input.owner, input.set.id);
            active.add_models(&input, &label, &mut model_owner);
            for (hash, slot) in input.replaced {
                if let Some(earlier) = texture_owner.insert(hash, label.clone()) {
                    active.conflict("texture", hash, &label, &earlier);
                }
                active.textures.insert(hash, slot);
            }
            // Two sets of one mod may share a pack (the 2012 mod's trees and bushes): a texture of the same name is added once.
            for texture in input.added {
                if active.added.iter().all(|known| known.name_hash != texture.name_hash) {
                    active.added.push(texture);
                }
            }
            if let (false, Some(geometry)) = (input.set.adds.is_empty(), &input.set.geometry) {
                active.objects.push(Added {
                    owner: input.owner.clone(),
                    set: input.set.id.clone(),
                    geometry: geometry.clone(),
                    models: input.models.iter().map(|model| model.name.to_ascii_uppercase()).collect(),
                    places: input.set.adds.clone(),
                });
            }
            active.sets.push(label);
        }
        active
    }

    fn conflict(&mut self, what: &str, hash: u32, winner: &str, loser: &str) {
        let line = format!("{what} {hash:08X}: {winner} replaces what {loser} replaced, and wins");
        log::warn!("replacements: {line}");
        self.conflicts.push(line);
    }

    fn add_models(&mut self, input: &LoadedSet, label: &str, owner: &mut HashMap<u32, String>) {
        let set = &input.set;
        let by_name: HashMap<u32, &Solid> =
            input.models.iter().map(|model| (bstring_hash(&model.name), model)).collect();
        let provided: HashSet<u32> = input.added.iter().map(|texture| texture.name_hash).collect();
        let mut prepared: HashMap<u32, Option<Arc<Prepared>>> = HashMap::new();
        for entry in &set.models {
            let model_hash = bstring_hash(&entry.model);
            let Some(model) = by_name.get(&model_hash) else {
                log::warn!(
                    "replacements: {label}: the model {} is not in {}",
                    entry.model,
                    set.geometry.as_deref().unwrap_or("the set")
                );
                continue;
            };
            let missing = model.texture_hashes.iter().any(|hash| !provided.contains(hash));
            if missing && set.textures.is_some() {
                log::warn!(
                    "replacements: {label}: the model {} uses a texture that is not in {}",
                    entry.model,
                    set.textures.as_deref().unwrap_or("")
                );
                continue;
            }
            let ready = prepared.entry(model_hash).or_insert_with(|| match Prepared::new(model) {
                Ok(ready) => Some(Arc::new(ready)),
                Err(error) => {
                    log::warn!("replacements: {label}: {error:#}");
                    None
                }
            });
            let Some(ready) = ready.clone() else { continue };
            for target in &entry.targets {
                let hash = target_hash(target);
                if let Some(earlier) = owner.insert(hash, label.to_owned()) {
                    self.conflict("model", hash, label, &earlier);
                }
                self.models.insert(hash, Entry { model: ready.clone(), fit: entry.fit, limits: set.limits });
            }
        }
    }

    /// How many solids of the game the sets replace.
    pub fn swaps(&self) -> usize {
        self.models.len()
    }

    /// How many textures of the game the sets replace.
    pub fn replaced_textures(&self) -> usize {
        self.textures.len()
    }

    /// The textures the sets add to the world. They are not copied into the sections the sets rewrite: the world puts them in the
    /// shared sets once, and every section's solids find them there.
    pub fn textures(&self) -> &[Texture] {
        &self.added
    }

    /// The objects the sets place in the world.
    pub fn objects(&self) -> &[Added] {
        &self.objects
    }

    /// The sets in effect, as `mod:set`.
    pub fn sets(&self) -> &[String] {
        &self.sets
    }

    /// One line for every target that two sets named.
    pub fn conflicts(&self) -> &[String] {
        &self.conflicts
    }

    /// What a loader thread runs on a section it has parsed: the models and the textures the sets replace.
    pub fn apply(&self, data: &mut SectionData) {
        if !self.models.is_empty() {
            self.apply_models(data);
        }
        self.replace_textures(&mut data.textures);
    }

    /// Replace the textures of `textures` that a set replaces, by name hash.
    pub fn replace_textures(&self, textures: &mut [Texture]) {
        if self.textures.is_empty() {
            return;
        }
        for texture in textures {
            let Some(slot) = self.textures.get(&texture.name_hash) else { continue };
            if let Some(new) = slot.replacement(texture) {
                *texture = new;
            }
        }
    }

    /// Replace the solids of a loaded section that a set has a model for: the game's own solids by name, and the world-space
    /// copies of them (`_DEINST` names) by the name of the solid they are a copy of. A solid whose model cannot be fitted keeps
    /// its own look (a warning says why).
    fn apply_models(&self, data: &mut SectionData) {
        let mut mirrored: Option<HashSet<u32>> = None;
        for solid in &mut data.solids {
            let Some((entry, world_copy)) = self.entry_for(solid) else { continue };
            let frame = match world_copy {
                false => Frame::Local,
                true => {
                    let set = mirrored.get_or_insert_with(|| mirrored_solids(&data.scenery));
                    Frame::World { mirrored: set.contains(&solid.name_hash) }
                }
            };
            let Some(replacement) = self.replacement(entry, solid, frame) else { continue };
            *solid = replacement;
        }
    }

    /// The entry that replaces `solid`, and whether the solid is a world-space copy of the one the entry is listed under.
    fn entry_for(&self, solid: &Solid) -> Option<(&Entry, bool)> {
        if let Some(entry) = self.models.get(&solid.name_hash) {
            return Some((entry, false));
        }
        let base = world_copy_base(&solid.name)?;
        let entry = self.models.get(&bstring_hash(base))?;
        Some((entry, true))
    }

    /// The fitted replacement of `stock`. A local solid is cached by its bounds (the game uses one solid in many places with the
    /// same bounds); a world copy is its own, so it is fitted every time and nothing of it is kept.
    fn replacement(&self, entry: &Entry, stock: &Solid, frame: Frame) -> Option<Solid> {
        if frame != Frame::Local {
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

    /// The replacement of `stock`, or `None` when the solid keeps the game's own model: the model cannot be fitted into its
    /// footprint within the limits (that is the rule, not an error), or the result cannot be made (a warning says why).
    fn fit_now(&self, entry: &Entry, stock: &Solid, frame: Frame) -> Option<Solid> {
        match fit_prepared(&entry.model, stock, entry.fit, frame, &entry.limits) {
            Ok(solid) => solid,
            Err(error) => {
                log::warn!("replacements: {} keeps its own model: {error:#}", stock.name);
                None
            }
        }
    }
}
