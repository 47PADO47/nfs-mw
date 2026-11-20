//! The game's own textures that a replacement took the place of, kept so the world can show them again.
//!
//! A section that loads has the textures the sets replace swapped for the sets' pictures before it is uploaded, and the shared sets
//! (where the road strips and most props' textures live) load once, when the world opens. When the player chooses another value of a
//! kind with the world open, the replacement is shown over the texture that is on the GPU (a texture redirect, `residency/shared_textures.rs`),
//! and going back to the game's own needs its pixels again: the first time a replacement is made for a texture, the original is kept
//! here (a few megabytes: only the textures a set replaced, once each).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use blackbox_tpk::Texture;

static ORIGINALS: RwLock<Option<HashMap<u32, Arc<Texture>>>> = RwLock::new(None);

/// Keep `texture` (the game's, about to be replaced) unless it is kept already.
pub fn note(texture: &Texture) {
    let kept =
        ORIGINALS.read().ok().is_some_and(|map| map.as_ref().is_some_and(|map| map.contains_key(&texture.name_hash)));
    if kept {
        return;
    }
    let Ok(mut map) = ORIGINALS.write() else { return };
    let map = map.get_or_insert_with(HashMap::new);
    if map.insert(texture.name_hash, Arc::new(texture.clone())).is_none() {
        log::debug!(
            "replacements: the texture {} ({:08X}) is replaced as a section loads",
            texture.name,
            texture.name_hash
        );
    }
}

/// The game's own texture of this name hash, when a set has replaced it since the process started.
pub fn original(hash: u32) -> Option<Arc<Texture>> {
    ORIGINALS.read().ok()?.as_ref()?.get(&hash).cloned()
}

/// The name hashes of the textures kept.
pub fn original_hashes() -> Vec<u32> {
    ORIGINALS.read().ok().and_then(|map| map.as_ref().map(|map| map.keys().copied().collect())).unwrap_or_default()
}
