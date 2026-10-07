//! Finding a car's textures in the texture packs.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use blackbox_tpk::Texture;
use game_install::GameDir;

use crate::read_unwrapped;

/// Packs searched for car textures, in priority order. `{car}` is the car folder.
const TEXTURE_FILES: &[&str] = &["CARS/{car}/TEXTURES.BIN", "CARS/TEXTURES.BIN", "GLOBAL/GLOBALB.LZC"];

/// The `wanted` textures (by name hash) that the packs have.
pub fn load(dir: &GameDir, car: &str, wanted: &HashSet<u32>) -> Result<HashMap<u32, Texture>> {
    let mut textures = HashMap::new();
    for pattern in TEXTURE_FILES {
        let rel = pattern.replace("{car}", car);
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
