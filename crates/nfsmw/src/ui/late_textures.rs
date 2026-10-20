//! Textures added to the UI assets after they were loaded: the per-event minimap tiles (`docs/specs/race-minimap.md`),
//! read on demand when a race starts and dropped when it ends. The stock textures are never touched.

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

use blackbox_tpk::Texture;

/// A texture found by [`super::UiAssets::texture`]: one of the stock textures (borrowed) or a late one (shared, so it
/// outlives the lock that found it).
pub enum TextureRef<'a> {
    Stock(&'a Texture),
    Late(Arc<Texture>),
}

impl Deref for TextureRef<'_> {
    type Target = Texture;

    fn deref(&self) -> &Texture {
        match self {
            TextureRef::Stock(texture) => texture,
            TextureRef::Late(texture) => texture,
        }
    }
}

/// The late textures by name hash. The lock is held only for a lookup, an insert or a removal.
#[derive(Default)]
pub struct LateTextures {
    textures: RwLock<HashMap<u32, Arc<Texture>>>,
}

impl LateTextures {
    pub fn get(&self, hash: u32) -> Option<Arc<Texture>> {
        self.textures.read().unwrap_or_else(|e| e.into_inner()).get(&hash).cloned()
    }

    pub fn contains(&self, hash: u32) -> bool {
        self.textures.read().unwrap_or_else(|e| e.into_inner()).contains_key(&hash)
    }

    /// Adds textures; one with a name already present replaces it.
    pub fn add(&self, textures: impl IntoIterator<Item = Texture>) {
        let mut map = self.textures.write().unwrap_or_else(|e| e.into_inner());
        for texture in textures {
            map.insert(texture.name_hash, Arc::new(texture));
        }
    }

    pub fn remove(&self, hashes: &[u32]) {
        let mut map = self.textures.write().unwrap_or_else(|e| e.into_inner());
        for hash in hashes {
            map.remove(hash);
        }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.textures.read().unwrap_or_else(|e| e.into_inner()).len()
    }
}
