//! Texture redirects: draw `to` wherever `from` is used, like native's `gpu/textures.rs`.
//!
//! Draws key their material by the texture number they were given (`from`), never by what it currently
//! resolves to, so a redirect that changes mid-game repoints the materials already drawing `from` instead
//! of needing every caller to notice and redraw.

use super::assets::Stores;
use super::state::WorldState;

/// `texture` resolved through the current redirects (itself, if there is none).
pub fn resolve(state: &WorldState, texture: usize) -> usize {
    state.redirects.get(&texture).copied().unwrap_or(texture)
}

/// Set or clear a redirect, and repoint every material already keyed on `from`.
pub fn set(state: &mut WorldState, stores: &mut Stores, from: usize, to: Option<usize>) {
    match to {
        Some(to) if to != from => {
            state.redirects.insert(from, to);
        }
        _ => {
            state.redirects.remove(&from);
        }
    }
    retarget(state, stores, from);
}

/// `texture` was destroyed: drop a redirect that starts there, and un-redirect (and repoint) anything
/// that was pointed at it.
pub fn purge(state: &mut WorldState, stores: &mut Stores, texture: usize) {
    state.redirects.remove(&texture);
    let affected: Vec<usize> =
        state.redirects.iter().filter(|&(_, &to)| to == texture).map(|(&from, _)| from).collect();
    for from in affected {
        state.redirects.remove(&from);
        retarget(state, stores, from);
    }
}

/// Point every material keyed on `from` at the image it now resolves to.
fn retarget(state: &mut WorldState, stores: &mut Stores, from: usize) {
    let Some(keys) = state.materials.by_texture.get(&from) else { return };
    let image = state.textures.get(&resolve(state, from)).cloned();
    for key in keys.clone() {
        let Some(handle) = state.materials.by_key.get(&key) else { continue };
        let Some(mut material) = stores.materials.get_mut(handle) else { continue };
        material.texture = image.clone();
    }
}

#[cfg(test)]
mod tests {
    use bevy_asset::{Assets, Handle};
    use bevy_ecs::system::SystemState;
    use bevy_ecs::world::World;
    use bevy_image::Image;
    use bevy_mesh::Mesh;

    use super::*;
    use crate::apply::assets::material_for;
    use crate::apply::state::MaterialKey;
    use crate::material::{BlackboxMaterial, BlendKind, ShadingKind};

    fn world() -> World {
        let mut world = World::new();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<BlackboxMaterial>>();
        world
    }

    fn with_stores<R>(world: &mut World, f: impl FnOnce(&mut Stores) -> R) -> R {
        let mut system_state = SystemState::<Stores>::new(world);
        let mut stores = system_state.get_mut(world).unwrap();
        f(&mut stores)
    }

    fn solid(world: &mut World) -> Handle<Image> {
        with_stores(world, |stores| stores.images.add(Image::default()))
    }

    #[test]
    fn a_redirected_draw_gets_the_target_s_image() {
        let mut world = world();
        let mut state = WorldState::default();
        let a = solid(&mut world);
        let b = solid(&mut world);
        state.textures.insert(1, a);
        state.textures.insert(2, b.clone());
        with_stores(&mut world, |stores| {
            let handle = material_for(&mut state, stores, Some(1), BlendKind::Opaque, ShadingKind::Lit);
            set(&mut state, stores, 1, Some(2));
            assert_eq!(stores.materials.get(&handle).unwrap().texture, Some(b));
        });
    }

    #[test]
    fn clearing_a_redirect_restores_the_original_image() {
        let mut world = world();
        let mut state = WorldState::default();
        let a = solid(&mut world);
        let b = solid(&mut world);
        state.textures.insert(1, a.clone());
        state.textures.insert(2, b.clone());
        with_stores(&mut world, |stores| {
            let handle = material_for(&mut state, stores, Some(1), BlendKind::Opaque, ShadingKind::Lit);
            set(&mut state, stores, 1, Some(2));
            assert_eq!(stores.materials.get(&handle).unwrap().texture, Some(b));
            set(&mut state, stores, 1, None);
            assert_eq!(stores.materials.get(&handle).unwrap().texture, Some(a));
        });
    }

    #[test]
    fn destroying_the_target_un_redirects_and_repoints_back() {
        let mut world = world();
        let mut state = WorldState::default();
        let a = solid(&mut world);
        let b = solid(&mut world);
        state.textures.insert(1, a.clone());
        state.textures.insert(2, b);
        with_stores(&mut world, |stores| {
            let handle = material_for(&mut state, stores, Some(1), BlendKind::Opaque, ShadingKind::Lit);
            set(&mut state, stores, 1, Some(2));
            purge(&mut state, stores, 2);
            assert!(state.redirects.is_empty());
            assert_eq!(stores.materials.get(&handle).unwrap().texture, Some(a));
        });
    }

    #[test]
    fn an_unrelated_material_is_untouched() {
        let mut world = world();
        let mut state = WorldState::default();
        let a = solid(&mut world);
        let b = solid(&mut world);
        let c = solid(&mut world);
        state.textures.insert(1, a);
        state.textures.insert(2, b);
        state.textures.insert(3, c.clone());
        with_stores(&mut world, |stores| {
            let other = material_for(&mut state, stores, Some(3), BlendKind::Opaque, ShadingKind::Lit);
            set(&mut state, stores, 1, Some(2));
            assert_eq!(stores.materials.get(&other).unwrap().texture, Some(c));
            assert!(!state.materials.by_key.contains_key(&MaterialKey {
                texture: Some(1),
                blend: BlendKind::Opaque,
                shading: ShadingKind::Lit
            }));
        });
    }
}
