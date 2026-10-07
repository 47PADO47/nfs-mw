//! Car textures on the GPU, looked up through the car's texture swaps.

use std::collections::HashMap;

use blackbox_render::{BlendMode, Renderer, TextureHandle};
use blackbox_scene::{MaterialLookup, blend_mode, upload_texture};
use blackbox_tpk::Texture;
use nfsmw_data::car::TextureSwaps;

pub struct CarMaterials {
    uploaded: HashMap<u32, (TextureHandle, BlendMode)>,
}

impl CarMaterials {
    pub fn upload(renderer: &mut Renderer, textures: &HashMap<u32, Texture>) -> Self {
        let uploaded = textures
            .iter()
            .filter_map(|(&hash, t)| upload_texture(renderer, t).map(|handle| (hash, (handle, blend_mode(Some(t))))))
            .collect();
        Self { uploaded }
    }

    /// The lookup for one placement (left brakes show the mirrored caliper).
    pub fn for_placement<'a>(&'a self, swaps: &'a TextureSwaps, left_brake: bool) -> Lookup<'a> {
        Lookup { materials: self, swaps, left_brake }
    }
}

pub struct Lookup<'a> {
    materials: &'a CarMaterials,
    swaps: &'a TextureSwaps,
    left_brake: bool,
}

impl MaterialLookup for Lookup<'_> {
    fn material(&self, texture_hash: u32) -> Option<(TextureHandle, BlendMode)> {
        // A replacement the packs don't have falls back to the placeholder itself.
        let swapped = self.swaps.resolve_for(texture_hash, self.left_brake);
        let uploaded = &self.materials.uploaded;
        uploaded.get(&swapped).or_else(|| uploaded.get(&texture_hash)).copied()
    }

    fn draws(&self, texture_hash: u32) -> bool {
        !self.swaps.hidden.contains(&texture_hash)
    }
}
