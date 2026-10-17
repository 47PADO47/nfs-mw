//! Finding a car's textures in the texture packs.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Context, Result};
use blackbox_tpk::Texture;
use game_install::GameDir;

use crate::{read_unwrapped, shared_unwrapped};

/// Packs searched for car textures, in priority order. `{car}` is the car folder.
const TEXTURE_FILES: &[&str] =
    &["CARS/{car}/TEXTURES.BIN", "CARS/TEXTURES.BIN", "GLOBAL/GLOBALB.LZC", "GLOBAL/INGAMEA.BUN"];

/// The packs that every car searches: they are kept for the session. A car's own pack and an accessory's pack are read
/// for one load only, so the session does not hold a copy of every car's textures.
const SESSION_PACKS: &[&str] = &["CARS/TEXTURES.BIN", "GLOBAL/GLOBALB.LZC", "GLOBAL/INGAMEA.BUN"];

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
        let file = pack_bytes(dir, &rel)?;
        // Only the wanted textures are copied or decompressed; the others are skipped before their pixels are read.
        let packs = blackbox_tpk::read_texture_packs_filtered(&file, |hash| wanted.contains(&hash))
            .with_context(|| format!("parsing textures in {rel}"))?;
        for pack in packs {
            for t in pack.textures {
                textures.entry(t.name_hash).or_insert(t);
            }
            for (hash, why) in pack.failed.iter().filter(|(h, _)| wanted.contains(h)) {
                log::debug!("{rel}: texture 0x{hash:08X}: {why}");
            }
        }
    }
    Ok(textures)
}

/// The bytes of one pack with its compression wrapper removed: shared for the session when it is one of the install's
/// packs in [`SESSION_PACKS`], read for this load otherwise.
fn pack_bytes(dir: &GameDir, rel: &str) -> Result<Arc<Vec<u8>>> {
    if SESSION_PACKS.contains(&rel) {
        return shared_unwrapped(dir, rel);
    }
    Ok(Arc::new(read_unwrapped(dir, rel)?))
}
