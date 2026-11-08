//! Reading a replacement set from its files: the geometry, the texture pack, and the slots of the textures it replaces.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use blackbox_hash::bstring_hash;
use blackbox_modding::content::{ReplaceSet, TextureEntry, TextureSource};
use game_install::GameDir;

use super::active::{LoadedSet, target_hash};
use super::textures::TextureSlot;

/// The set `set` of mod `owner` (`order`: its place in the load order), read through the install's layers (the mod's `files/` is
/// one). Fails when the geometry file or the pack is missing or does not parse. A texture entry whose picture cannot be had is left
/// out with a warning (a file that is there is read later, when a section needs it).
pub fn load_set(dir: &GameDir, set: Arc<ReplaceSet>, owner: &str, order: usize) -> Result<LoadedSet> {
    let models = match &set.geometry {
        Some(path) => {
            let bytes = dir.read(path).with_context(|| format!("reading {path}"))?;
            blackbox_solid::read_solids(&bytes).with_context(|| format!("{path} does not parse"))?
        }
        None => Vec::new(),
    };
    let mut pack = Vec::new();
    if let Some(path) = &set.textures {
        let bytes = dir.read(path).with_context(|| format!("reading {path}"))?;
        for texture_pack in
            blackbox_tpk::read_texture_packs(&bytes).with_context(|| format!("{path} does not parse"))?
        {
            pack.extend(texture_pack.textures);
        }
    }
    let mut by_hash: HashMap<u32, usize> =
        pack.iter().enumerate().map(|(at, texture)| (texture.name_hash, at)).collect();
    let mut replaced = Vec::new();
    let mut consumed = Vec::new();
    let mut added_from_files = Vec::new();
    for entry in &set.texture_entries {
        let source = SlotSource::of(entry, &pack, &mut by_hash, dir, owner);
        let slot = match source {
            Ok(slot) => slot,
            Err(error) => {
                log::warn!("mod {owner}: set {}: the texture {} is left out: {error:#}", set.id, entry.name);
                continue;
            }
        };
        consumed.push(slot.pack_index);
        match &entry.replaces {
            Some(target) => replaced.push((target_hash(target), Arc::new(slot.slot))),
            None => {
                // An added texture is made now: it goes to the GPU when the set takes effect.
                match slot.slot.added(&entry.name) {
                    Some(texture) => added_from_files.push(texture),
                    None => log::warn!("mod {owner}: set {}: the texture {} is not added", set.id, entry.name),
                }
            }
        }
    }
    let consumed: Vec<usize> = consumed.into_iter().flatten().collect();
    let mut added: Vec<_> =
        pack.into_iter().enumerate().filter(|(at, _)| !consumed.contains(at)).map(|(_, texture)| texture).collect();
    added.extend(added_from_files);
    Ok(LoadedSet { owner: owner.to_owned(), order, set, models, added, replaced })
}

/// A slot and, when its picture is a texture of the pack, which one (a texture a `[[texture]]` entry takes is not added again).
struct SlotSource {
    slot: TextureSlot,
    pack_index: Option<usize>,
}

impl SlotSource {
    fn of(
        entry: &TextureEntry,
        pack: &[blackbox_tpk::Texture],
        by_hash: &mut HashMap<u32, usize>,
        dir: &GameDir,
        owner: &str,
    ) -> Result<Self> {
        match &entry.source {
            TextureSource::Pack(name) => {
                let at = *by_hash
                    .get(&bstring_hash(name))
                    .with_context(|| format!("the texture {name} is not in the set's pack"))?;
                let slot = TextureSlot::ready(owner, &entry.name, pack[at].clone(), entry.state);
                Ok(Self { slot, pack_index: Some(at) })
            }
            TextureSource::File(path) => {
                anyhow::ensure!(dir.exists(path), "{path} is not in any layer");
                let slot = TextureSlot::file(
                    owner,
                    &entry.name,
                    path,
                    dir.clone(),
                    entry.state,
                    entry.convert,
                    entry.max_size,
                );
                Ok(Self { slot, pack_index: None })
            }
        }
    }
}
