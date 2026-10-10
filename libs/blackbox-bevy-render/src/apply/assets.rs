//! Textures, meshes and materials: turning the queue's resource ops into Bevy assets.

use bevy_asset::{Assets, Handle};
use bevy_ecs::system::ResMut;
use bevy_ecs::system::SystemParam;
use bevy_image::Image;
use bevy_mesh::Mesh;

use super::redirects;
use super::state::{MaterialKey, MeshEntry, RangeEntry, WorldState};
use crate::material::{BlackboxMaterial, BlendKind, Params, ShadingKind};
use crate::ops::Op;

/// The asset stores the apply system writes.
#[derive(SystemParam)]
pub struct Stores<'w> {
    pub images: ResMut<'w, Assets<Image>>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub materials: ResMut<'w, Assets<BlackboxMaterial>>,
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
                    mesh: stores.meshes.add(range.mesh),
                    texture: range.texture.map(|t| t.raw()),
                    blend: range.blend.into(),
                    shading: ShadingKind::of(range.shading),
                    glossy: match range.shading {
                        blackbox_gfx::Shading::Glossy(handle) => Some(handle.raw()),
                        _ => None,
                    },
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
        Op::SetEnvironment(image) => {
            state.environment = Some(stores.images.add(*image));
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
        params: state.params.unwrap_or_default(),
        texture: image,
        glossy: glossy_params,
        rig: state.rig,
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

/// Give every material the new fog and light, once, when the frame's differ from what they hold.
pub fn sync_params(state: &mut WorldState, stores: &mut Stores, wanted: Params) {
    if state.params == Some(wanted) {
        return;
    }
    state.params = Some(wanted);
    for handle in state.materials.by_key.values() {
        if let Some(mut material) = stores.materials.get_mut(handle) {
            material.params = wanted;
        }
    }
}

/// Give every glossy material the current lighting rig and environment, once, when either changed.
pub fn sync_rig_environment(state: &mut WorldState, stores: &mut Stores) {
    let wanted = (state.rig, state.environment.clone());
    if state.rig_environment_applied == Some(wanted.clone()) {
        return;
    }
    state.rig_environment_applied = Some(wanted.clone());
    for (key, handle) in &state.materials.by_key {
        if key.shading != ShadingKind::Glossy {
            continue;
        }
        if let Some(mut material) = stores.materials.get_mut(handle) {
            material.rig = wanted.0;
            material.environment = wanted.1.clone();
        }
    }
}
