//! Car textures and shading materials on the GPU, looked up through the car's texture swaps
//! and its paint.

use std::collections::{HashMap, HashSet};

use blackbox_render::{BlendMode, GlossyMaterial, GlossyMaterialHandle, Renderer, Shading, TextureHandle};
use blackbox_scene::{MaterialLookup, blend_mode, upload_texture};
use nfsmw_data::car::{CARSKIN, CarModel, TextureSwaps};

use super::shading::glossy_material;

pub struct CarMaterials {
    uploaded: HashMap<u32, (TextureHandle, BlendMode)>,
    /// The glossy material of each light material the car uses, by name hash.
    glossy: HashMap<u32, GlossyMaterialHandle>,
    /// For light materials the car tables do not know.
    fallback: GlossyMaterialHandle,
    /// The light material that stands in for `CARSKIN`.
    paint: Option<u32>,
}

impl CarMaterials {
    pub fn upload(renderer: &mut Renderer, model: &CarModel) -> Self {
        let uploaded = model
            .textures
            .iter()
            .filter_map(|(&hash, t)| upload_texture(renderer, t).map(|handle| (hash, (handle, blend_mode(Some(t))))))
            .collect();
        let paint = model.paint.as_ref().and_then(|p| p.light_material);
        let used: HashSet<u32> =
            model.solids.values().flat_map(|s| s.light_material_hashes.iter().copied()).chain(paint).collect();
        let glossy = used
            .into_iter()
            .filter_map(|hash| {
                let material = model.light_materials.get(&hash)?;
                Some((hash, renderer.create_glossy_material(&glossy_material(material))))
            })
            .collect();
        let fallback = renderer.create_glossy_material(&GlossyMaterial::default());
        Self { uploaded, glossy, fallback, paint }
    }

    /// Free the textures and materials.
    pub fn destroy(self, renderer: &mut Renderer) {
        for (handle, _) in self.uploaded.into_values() {
            renderer.destroy_texture(handle);
        }
        for handle in self.glossy.into_values().chain([self.fallback]) {
            renderer.destroy_glossy_material(handle);
        }
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

    /// Body groups name `CARSKIN`, which stands for the paint's light material.
    fn shading(&self, light_material_hash: u32) -> Option<Shading> {
        let materials = self.materials;
        let hash = match (light_material_hash == CARSKIN, materials.paint) {
            (true, Some(paint)) => paint,
            _ => light_material_hash,
        };
        Some(Shading::Glossy(materials.glossy.get(&hash).copied().unwrap_or(materials.fallback)))
    }
}
