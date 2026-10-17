//! Textures, meshes and materials: turning the queue's resource ops into Bevy assets.

use bevy_asset::{Assets, Handle};
use bevy_ecs::system::ResMut;
use bevy_ecs::system::SystemParam;
use bevy_image::Image;
use bevy_mesh::Mesh;
use bevy_render::storage::ShaderBuffer;

use super::redirects;
use super::state::{MaterialKey, MeshEntry, RangeEntry, WorldState};
use crate::material::{BlackboxMaterial, BlendKind, Params, SCENE_BUFFER, SceneUniform, ShadingKind, scene};
use crate::ops::Op;

/// The asset stores the apply system writes.
#[derive(SystemParam)]
pub struct Stores<'w> {
    pub images: ResMut<'w, Assets<Image>>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub materials: ResMut<'w, Assets<BlackboxMaterial>>,
    pub buffers: ResMut<'w, Assets<ShaderBuffer>>,
}

/// Apply one resource op.
pub fn apply_op(state: &mut WorldState, stores: &mut Stores, op: Op) {
    match op {
        Op::AddTexture { handle, image } => {
            state.textures.insert(handle.raw(), stores.images.add(*image));
        }
        Op::RemoveTexture(handle) => remove_texture(state, stores, handle.raw()),
        Op::SetRedirect { from, to } => redirects::set(state, stores, from.raw(), to.map(|h| h.raw())),
        Op::AddMesh { handle, ranges } => {
            let ranges = ranges
                .into_iter()
                .map(|range| RangeEntry {
                    aabb: range.aabb,
                    mesh: stores.meshes.add(range.mesh),
                    texture: range.texture.map(|t| t.raw()),
                    blend: range.blend.into(),
                    shading: ShadingKind::of(range.shading),
                    glossy: match range.shading {
                        blackbox_gfx::Shading::Glossy(handle) => Some(handle.raw()),
                        _ => None,
                    },
                    colour: range.colour,
                })
                .collect();
            state.meshes.insert(handle.raw(), MeshEntry { ranges });
        }
        Op::RemoveMesh(handle) => {
            state.meshes.remove(&handle.raw());
        }
        Op::AddGlossyMaterial { handle, params } => {
            state.glossy.insert(handle.raw(), params);
        }
        Op::RemoveGlossyMaterial(handle) => {
            state.glossy.remove(&handle.raw());
        }
        Op::SetLightingRig(rig) => state.rig = rig,
        Op::SetEnvironment(image) => set_environment(state, stores, *image),
    }
}

/// Make `image` the environment of every glossy material, now and for the ones made later. The only per-material
/// data a scene still changes, and only when the caller sets a new environment.
fn set_environment(state: &mut WorldState, stores: &mut Stores, image: Image) {
    let environment = stores.images.add(image);
    state.environment = Some(environment.clone());
    for (key, handle) in &state.materials.by_key {
        if key.shading != ShadingKind::Glossy {
            continue;
        }
        if let Some(mut material) = stores.materials.get_mut(handle) {
            material.environment = Some(environment.clone());
        }
    }
}

/// Drop a texture and the materials that hold it. Entities that still use one keep the asset alive until they go.
fn remove_texture(state: &mut WorldState, stores: &mut Stores, texture: usize) {
    redirects::purge(state, stores, texture);
    state.textures.remove(&texture);
    let Some(keys) = state.materials.by_texture.remove(&texture) else { return };
    for key in keys {
        state.materials.by_key.remove(&key);
    }
}

/// The material for a draw: shared by everything with the same texture, blend, shading and (for
/// glossy draws) glossy material number.
pub fn material_for(
    state: &mut WorldState,
    stores: &mut Stores,
    texture: Option<usize>,
    blend: BlendKind,
    shading: ShadingKind,
    glossy: Option<usize>,
) -> Handle<BlackboxMaterial> {
    let key = MaterialKey { texture, blend, shading, glossy };
    if let Some(handle) = state.materials.by_key.get(&key) {
        return handle.clone();
    }
    let image = texture.and_then(|t| state.textures.get(&redirects::resolve(state, t)).cloned());
    let glossy_params = glossy.and_then(|h| state.glossy.get(&h).copied()).unwrap_or_default();
    let handle = stores.materials.add(BlackboxMaterial {
        scene: SCENE_BUFFER,
        texture: image,
        glossy: glossy_params,
        environment: state.environment.clone(),
        shading,
        blend,
    });
    state.materials.by_key.insert(key, handle.clone());
    if let Some(texture) = texture {
        state.materials.by_texture.entry(texture).or_default().push(key);
    }
    handle
}

/// Write the frame's fog, light and the lighting rig into the buffer every material reads, when they differ from
/// what it holds. The materials themselves are not touched.
pub fn sync_scene(state: &mut WorldState, stores: &mut Stores, wanted: Params) {
    let scene = SceneUniform { params: wanted, rig: state.rig };
    if state.scene == Some(scene) {
        return;
    }
    state.scene = Some(scene);
    state.params = Some(wanted);
    scene::write(&mut stores.buffers, &scene);
}

#[cfg(test)]
mod tests;
