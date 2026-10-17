//! PC shading-group effect routing; see `docs/specs/world-material-routing.md`.

use blackbox_gfx::{
    BlendMode, NormalMaterialDesc, NormalMaterialHandle, NormalModel, RenderBackend, Shading, TextureHandle,
};
use blackbox_scene::MaterialLookup;
use blackbox_solid::Solid;
use std::collections::HashMap;

pub(super) struct Lookup<F>(pub F, pub bool, pub Vec<Option<Shading>>);

impl<F: Fn(u32) -> Option<(TextureHandle, BlendMode)>> MaterialLookup for Lookup<F> {
    fn material(&self, hash: u32) -> Option<(TextureHandle, BlendMode)> {
        self.0(hash)
    }

    fn group_shading(&self, index: usize) -> Option<Shading> {
        self.2.get(index).copied().flatten()
    }

    fn effect_shading(&self, effect: u32) -> Option<Shading> {
        // Sky reconstruction owns these solids, including the legacy layers whose
        // groups use ordinary world effects. Do not light the entire sky as scenery.
        if self.1 {
            return None;
        }
        match effect {
            0 => Some(Shading::World),
            6 => Some(Shading::Tree),
            8 => Some(Shading::WorldNoFog),
            17 => Some(Shading::WorldPrelit),
            // Normal-mapped/reflected surfaces need their extra vertex and texture inputs.
            // Leave these and unknown effect IDs on the caller's documented fallback.
            _ => None,
        }
    }
}

#[cfg(test)]
mod gpu_install_tests;
#[cfg(test)]
mod installed_surface_tests;
#[cfg(test)]
mod tests;
mod windows;
pub(super) use windows::window_materials;

/// Resolve the verified family only when all required resident inputs exist.
/// Backend rejection leaves the ordinary per-group legacy fallback in place.
pub(super) fn normal_materials(
    renderer: &mut dyn RenderBackend,
    solid: &Solid,
    lookup: &impl Fn(u32) -> Option<(TextureHandle, BlendMode)>,
    cache: &mut HashMap<(NormalModel, [TextureHandle; 3]), NormalMaterialHandle>,
) -> Vec<Option<Shading>> {
    solid
        .groups
        .iter()
        .map(|group| {
            if !matches!(group.effect_id, 1 | 3) || solid.name.starts_with("SKY") {
                return None;
            }
            let buffer = solid.vertex_buffers.get(group.vertex_buffer as usize)?;
            if buffer.stride != 60 || solid.tangents.len() < solid.vertices.len() {
                return None;
            }
            let textures: Option<Vec<TextureHandle>> = [0, 1, 3]
                .into_iter()
                .map(|slot| {
                    let hash = solid.texture_hashes.get(usize::from(group.texture_slots[slot]))?;
                    lookup(*hash).map(|m| m.0)
                })
                .collect();
            let textures: [TextureHandle; 3] = textures?.try_into().ok()?;
            let model = match group.effect_id {
                1 => NormalModel::Reflective,
                _ => NormalModel::Bump,
            };
            if let Some(handle) = cache.get(&(model, textures)) {
                return Some(Shading::WorldNormal(*handle));
            }
            let handle = renderer.create_normal_material(&NormalMaterialDesc {
                model,
                diffuse: textures[0],
                normal: textures[1],
                specular: textures[2],
                reflection: None,
                uv_offset: [0.0; 2],
            })?;
            cache.insert((model, textures), handle);
            Some(Shading::WorldNormal(handle))
        })
        .collect()
}
