//! Finding a car's textures in the texture packs.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use blackbox_tpk::Texture;
use game_install::GameDir;

use crate::read_unwrapped;

/// Packs searched for car textures, in priority order. `{car}` is the car folder.
const TEXTURE_FILES: &[&str] =
    &["CARS/{car}/TEXTURES.BIN", "CARS/TEXTURES.BIN", "GLOBAL/GLOBALB.LZC", "GLOBAL/INGAMEA.BUN"];

/// The `wanted` textures (by name hash) that the packs have.
pub fn load(dir: &GameDir, car: &str, wanted: &HashSet<u32>) -> Result<HashMap<u32, Texture>> {
    load_with_shared(dir, car, wanted, &HashSet::new())
}

/// Include only accessory packs that supplied placed geometry (e.g. aftermarket rims).
pub(super) fn load_with_shared(
    dir: &GameDir,
    car: &str,
    wanted: &HashSet<u32>,
    shared: &HashSet<String>,
) -> Result<HashMap<u32, Texture>> {
    let mut textures = HashMap::new();
    let mut shared: Vec<_> = shared.iter().collect();
    shared.sort();
    let files = std::iter::once(TEXTURE_FILES[0].replace("{car}", car))
        .chain(shared.into_iter().map(|folder| format!("CARS/{folder}/TEXTURES.BIN")))
        .chain(TEXTURE_FILES[1..].iter().map(|s| (*s).to_owned()));
    for rel in files {
        if !dir.exists(&rel) {
            continue;
        }
        let file = read_unwrapped(dir, &rel)?;
        for pack in blackbox_tpk::read_texture_packs(&file).with_context(|| format!("parsing textures in {rel}"))? {
            for t in pack.textures.into_iter().filter(|t| wanted.contains(&t.name_hash)) {
                textures.entry(t.name_hash).or_insert(t);
            }
            for (hash, why) in pack.failed.iter().filter(|(h, _)| wanted.contains(h)) {
                log::debug!("{rel}: texture 0x{hash:08X}: {why}");
            }
        }
    }
    Ok(textures)
}
