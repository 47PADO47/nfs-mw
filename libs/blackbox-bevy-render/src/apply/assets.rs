//! Textures, meshes and materials: turning the queue's resource ops into Bevy assets.

use bevy_asset::{Assets, Handle};
use bevy_ecs::system::ResMut;
use bevy_ecs::system::SystemParam;
use bevy_image::Image;
use bevy_mesh::Mesh;

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
        Op::AddMesh { handle, ranges } => {
            let ranges = ranges
                .into_iter()
                .map(|range| RangeEntry {
                    mesh: stores.meshes.add(range.mesh),
                    texture: range.texture.map(|t| t.raw()),
                    blend: range.blend.into(),
                    shading: ShadingKind::of(range.shading),
                })
                .collect();
            state.meshes.insert(handle.raw(), MeshEntry { ranges });
        }
        Op::RemoveMesh(handle) => {
            state.meshes.remove(&handle.raw());
        }
    }
}

/// Drop a texture and the materials that hold it. Entities that still use one keep the asset alive until they go.
fn remove_texture(state: &mut WorldState, _stores: &mut Stores, texture: usize) {
    state.textures.remove(&texture);
    let Some(keys) = state.materials.by_texture.remove(&texture) else { return };
    for key in keys {
        state.materials.by_key.remove(&key);
    }
}

/// The material for a draw: shared by everything with the same texture, blend and shading.
pub fn material_for(
    state: &mut WorldState,
    stores: &mut Stores,
    texture: Option<usize>,
    blend: BlendKind,
    shading: ShadingKind,
) -> Handle<BlackboxMaterial> {
    let key = MaterialKey { texture, blend, shading };
    if let Some(handle) = state.materials.by_key.get(&key) {
        return handle.clone();
    }
    let image = texture.and_then(|t| state.textures.get(&t).cloned());
    let handle = stores.materials.add(BlackboxMaterial {
        params: state.params.unwrap_or_default(),
        texture: image,
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
