//! Car textures and shading materials on the GPU, looked up through the car's texture swaps
//! and its paint. With simple car shading no glossy material or light rig is created at all and the
//! solids keep the renderer's single-light shading.

use std::collections::{HashMap, HashSet};

use blackbox_gfx::{
    BlendMode, GlossyMaterial, GlossyMaterialHandle, GlossyModel, RenderBackend, Shading, TextureHandle,
};
use blackbox_scene::{MaterialLookup, blend_mode, upload_texture};
use glam::Vec3;
use nfsmw_data::car::{CARSKIN, CarModel, TextureSwaps};

use super::lighting;
use super::shading::{glossy_material, original_material};
use crate::settings::CarShading;

const WINDSHIELD: u32 = blackbox_hash::bstring_hash("WINDSHIELD");

pub struct CarMaterials {
    uploaded: HashMap<u32, (TextureHandle, BlendMode)>,
    /// The glossy material of each light material the car uses, by name hash.
    glossy: HashMap<u32, GlossyMaterialHandle>,
    /// For light materials the car tables do not know; `None` with simple shading.
    fallback: Option<GlossyMaterialHandle>,
    /// The light material that stands in for `CARSKIN`.
    paint: Option<u32>,
    window_tint: Option<u32>,
}

impl CarMaterials {
    /// Upload the car's textures and, for glossy shading, its light materials, lighting the rig by a sun
    /// that is in direction `to_sun` from the car.
    pub fn upload(renderer: &mut dyn RenderBackend, model: &CarModel, shading: CarShading, to_sun: Vec3) -> Self {
        let uploaded = model
            .textures
            .iter()
            .filter_map(|(&hash, t)| upload_texture(renderer, t).map(|handle| (hash, (handle, blend_mode(Some(t))))))
            .collect();
        let paint = model.paint.as_ref().and_then(|p| p.light_material);
        set_lighting(renderer, shading, to_sun);
        if shading == CarShading::Simple {
            return Self { uploaded, glossy: HashMap::new(), fallback: None, paint, window_tint: None };
        }
        let rig = match shading.is_original() {
            true => super::original_lighting::gameplay(to_sun, 1.0),
            false => lighting::rig(to_sun),
        };
        renderer.set_lighting_rig(&rig);
        let model_kind = match shading {
            CarShading::Original => GlossyModel::Shadowed,
            CarShading::OriginalUnshadowed => GlossyModel::VertexLit,
            _ => GlossyModel::Custom,
        };
        let window_tint = model.window_tint_material.filter(|_| shading.is_original());
        let used: HashSet<u32> = model
            .solids
            .values()
            .flat_map(|s| s.light_material_hashes.iter().copied())
            .chain(paint)
            .chain(window_tint)
            .collect();
        let glossy = used
            .into_iter()
            .filter_map(|hash| {
                let material = model.light_materials.get(&hash)?;
                let material = match shading.is_original() {
                    true => original_material(material, model_kind),
                    false => glossy_material(material),
                };
                Some((hash, renderer.create_glossy_material(&material)))
            })
            .collect();
        let fallback =
            Some(renderer.create_glossy_material(&GlossyMaterial { model: model_kind, ..Default::default() }));
        Self { uploaded, glossy, fallback, paint, window_tint }
    }

    /// The uploaded texture with this name hash, if the car draws it.
    pub fn texture(&self, hash: u32) -> Option<TextureHandle> {
        self.uploaded.get(&hash).map(|(handle, _)| *handle)
    }

    /// Free the textures and materials.
    pub fn destroy(self, renderer: &mut dyn RenderBackend) {
        for (handle, _) in self.uploaded.into_values() {
            renderer.destroy_texture(handle);
        }
        for handle in self.glossy.into_values().chain(self.fallback) {
            renderer.destroy_glossy_material(handle);
        }
    }

    /// The lookup for one placement (left brakes show the mirrored caliper).
    pub fn for_placement<'a>(&'a self, swaps: &'a TextureSwaps, left_brake: bool) -> Lookup<'a> {
        Lookup { materials: self, swaps, left_brake }
    }
}

#[cfg(test)]
mod tests;
/// Light the car's glossy surfaces by a sun in direction `to_sun` from the car. Simple shading keeps the
/// renderer's own single light, so nothing is set.
pub fn set_lighting(renderer: &mut dyn RenderBackend, shading: CarShading, to_sun: Vec3) {
    if shading == CarShading::Simple {
        return;
    }
    renderer.set_lighting_rig(&lighting::rig(to_sun));
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
        let fallback = materials.fallback?;
        let hash = match light_material_hash {
            CARSKIN => materials.paint.unwrap_or(light_material_hash),
            WINDSHIELD => materials.window_tint.unwrap_or(light_material_hash),
            _ => light_material_hash,
        };
        Some(Shading::Glossy(materials.glossy.get(&hash).copied().unwrap_or(fallback)))
    }
}
