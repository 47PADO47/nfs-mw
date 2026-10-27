//! Car textures and shading materials on the GPU, looked up through the car's texture swaps
//! and its paint. With simple car shading no glossy material or light rig is created at all and the
//! solids keep the renderer's single-light shading.

use std::collections::{HashMap, HashSet};

use blackbox_gfx::{
    BlendMode, GlossyMaterial, GlossyMaterialHandle, GlossyModel, GlossySurface, RenderBackend, Shading, SurfaceKind,
    TextureHandle,
};
use blackbox_scene::{MaterialLookup, Sidedness, blend_mode, upload_texture};
use glam::Vec3;
use nfsmw_data::car::{CARSKIN, CarModel, TextureSwaps};

use super::lighting;
use super::shading::{glossy_material, original_material};
use crate::settings::CarShading;

const WINDSHIELD: u32 = blackbox_hash::bstring_hash("WINDSHIELD");

fn lamp_surface_allowed(name: &str) -> bool {
    !name.contains("HEADLIGHT_GLASS") && !name.contains("BRAKELIGHT_GLASS")
}

#[cfg(test)]
#[path = "materials/traffic_tests.rs"]
mod traffic_tests;

pub struct CarMaterials {
    uploaded: HashMap<u32, (TextureHandle, BlendMode)>,
    /// Which faces each texture's surfaces draw, by name hash.
    sides: HashMap<u32, Sidedness>,
    /// The glossy material of each light material the car uses, by name hash.
    glossy: HashMap<u32, GlossyMaterialHandle>,
    lamp_glossy: HashMap<u32, GlossyMaterialHandle>,
    lamp_fallback: Option<GlossyMaterialHandle>,
    brake_glossy: HashMap<u32, GlossyMaterialHandle>,
    brake_fallback: Option<GlossyMaterialHandle>,
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
        Self::upload_with_background_policy(renderer, model, shading, to_sun, false)
    }

    pub fn upload_with_background_policy(
        renderer: &mut dyn RenderBackend,
        model: &CarModel,
        shading: CarShading,
        to_sun: Vec3,
        background_actor: bool,
    ) -> Self {
        let uploaded = model
            .textures
            .iter()
            .filter(|(_, t)| !t.name.starts_with("__GARAGE_"))
            .filter_map(|(&hash, t)| upload_texture(renderer, t).map(|handle| (hash, (handle, blend_mode(Some(t))))))
            .collect();
        let sides = model.textures.iter().map(|(&hash, t)| (hash, Sidedness::of(Some(t)))).collect();
        let paint = model.paint.as_ref().and_then(|p| p.light_material);
        set_lighting(renderer, shading, to_sun);
        if shading == CarShading::Simple {
            return Self {
                uploaded,
                sides,
                glossy: HashMap::new(),
                lamp_glossy: HashMap::new(),
                lamp_fallback: None,
                brake_glossy: HashMap::new(),
                brake_fallback: None,
                fallback: None,
                paint,
                window_tint: None,
            };
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
        let window_tint = model.window_tint_material;
        let used: HashSet<u32> = model
            .solids
            .values()
            .flat_map(|s| s.light_material_hashes.iter().copied())
            .chain(paint)
            .chain(window_tint)
            .collect();
        let glossy = used
            .iter()
            .copied()
            .filter_map(|hash| {
                let material = model.light_materials.get(&hash)?;
                let mut material = match shading.is_original() {
                    true => original_material(material, model_kind),
                    false => glossy_material(material),
                };
                if matches!(shading, CarShading::Glossy | CarShading::GlossyLamps)
                    && Some(hash) == paint
                    && model.paintable
                {
                    material.surface = model.paint.as_ref().and_then(super::paint_surface::response);
                }
                if matches!(shading, CarShading::Glossy | CarShading::GlossyLamps)
                    && Some(hash) != paint
                {
                    material.surface = if background_actor && shading == CarShading::Glossy {
                        super::material_response::scene_response(&model.light_materials[&hash])
                    } else {
                        super::material_response::response(&model.light_materials[&hash])
                    };
                }
                if background_actor
                    && shading == CarShading::Glossy
                    && model.light_materials[&hash].name == "DULLPLASTIC"
                {
                    // Cargo atlases already contain their colour; avoid the authored 1.2x diffuse boost.
                    for (minimum, range) in material.diffuse_min[..3]
                        .iter_mut()
                        .zip(&mut material.diffuse_range[..3])
                    {
                        let facing = (*minimum + *range).clamp(0.0, 1.0);
                        *minimum = minimum.clamp(0.0, 1.0);
                        *range = facing - *minimum;
                    }
                }
                material.background_actor = background_actor && shading == CarShading::Glossy;
                Some((hash, renderer.create_glossy_material(&material)))
            })
            .collect();
        let fallback = Some(renderer.create_glossy_material(&GlossyMaterial {
            model: model_kind,
            background_actor: background_actor && shading == CarShading::Glossy,
            ..Default::default()
        }));
        let enabled = shading == CarShading::GlossyLamps;
        let mut lamp_glossy = HashMap::new();
        let mut lamp_fallback = None;
        let mut brake_glossy = HashMap::new();
        let mut brake_fallback = None;
        if enabled {
            let surface = Some(GlossySurface { kind: SurfaceKind::Emissive, roughness: 0.2, coat: 0.0 });
            let red = Some(GlossySurface { kind: SurfaceKind::EmissiveRed, roughness: 0.2, coat: 0.0 });
            for hash in used {
                if let Some(authored) = model.light_materials.get(&hash) {
                    let mut desc = glossy_material(authored);
                    desc.surface = surface;
                    lamp_glossy.insert(hash, renderer.create_glossy_material(&desc));
                    desc.surface = red;
                    brake_glossy.insert(hash, renderer.create_glossy_material(&desc));
                }
            }
            lamp_fallback = Some(renderer.create_glossy_material(&GlossyMaterial { surface, ..Default::default() }));
            brake_fallback =
                Some(renderer.create_glossy_material(&GlossyMaterial { surface: red, ..Default::default() }));
        }
        Self {
            uploaded,
            sides,
            glossy,
            lamp_glossy,
            lamp_fallback,
            brake_glossy,
            brake_fallback,
            fallback,
            paint,
            window_tint,
        }
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
        for handle in self
            .glossy
            .into_values()
            .chain(self.fallback)
            .chain(self.lamp_glossy.into_values())
            .chain(self.lamp_fallback)
            .chain(self.brake_glossy.into_values())
            .chain(self.brake_fallback)
        {
            renderer.destroy_glossy_material(handle);
        }
    }

    pub fn enhanced_lamps(&self) -> bool {
        self.lamp_fallback.is_some()
    }

    pub fn enhanced_lamps_for(&self, name: &str) -> bool {
        self.enhanced_lamps() && lamp_surface_allowed(name)
    }

    /// The lookup for one placement (left brakes show the mirrored caliper).
    pub fn for_placement<'a>(&'a self, swaps: &'a TextureSwaps, left_brake: bool) -> Lookup<'a> {
        Lookup { materials: self, swaps, left_brake, lamp_surface: true }
    }

    /// Lamp glass sometimes reuses a lamp body texture slot; keep the cover non-emissive.
    pub fn for_solid<'a>(&'a self, swaps: &'a TextureSwaps, left_brake: bool, name: &str) -> Lookup<'a> {
        let mut lookup = self.for_placement(swaps, left_brake);
        lookup.lamp_surface = lamp_surface_allowed(name);
        lookup
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
    lamp_surface: bool,
}

impl MaterialLookup for Lookup<'_> {
    fn texture_shading(&self, texture_hash: u32, light_material_hash: Option<u32>) -> Option<Shading> {
        if !self.lamp_surface || !self.swaps.lamp_is_on(texture_hash) {
            return None;
        }
        let (materials, fallback) = match TextureSwaps::is_brake_lamp(texture_hash) {
            true => (&self.materials.brake_glossy, self.materials.brake_fallback?),
            false => (&self.materials.lamp_glossy, self.materials.lamp_fallback?),
        };
        let Some(light_material_hash) = light_material_hash else {
            return Some(Shading::Glossy(fallback));
        };
        let hash = match light_material_hash {
            CARSKIN => self.materials.paint.unwrap_or(light_material_hash),
            WINDSHIELD => self.materials.window_tint.unwrap_or(light_material_hash),
            _ => light_material_hash,
        };
        Some(Shading::Glossy(materials.get(&hash).copied().unwrap_or(fallback)))
    }
    fn material(&self, texture_hash: u32) -> Option<(TextureHandle, BlendMode)> {
        // A replacement the packs don't have falls back to the placeholder itself.
        let swapped = self.swaps.resolve_for(texture_hash, self.left_brake);
        let uploaded = &self.materials.uploaded;
        uploaded.get(&swapped).or_else(|| uploaded.get(&texture_hash)).copied()
    }

    fn draws(&self, texture_hash: u32) -> bool {
        !self.swaps.hidden.contains(&texture_hash)
    }

    /// The faces of the texture drawn in its place, as `material` resolves it.
    fn two_sided(&self, texture_hash: u32, effect_id: u32) -> bool {
        let swapped = self.swaps.resolve_for(texture_hash, self.left_brake);
        let sides = &self.materials.sides;
        let side = sides.get(&swapped).or_else(|| sides.get(&texture_hash));
        side.is_none_or(|s| s.draws_both(crate::scenes::culling::culls_blended(effect_id)))
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
