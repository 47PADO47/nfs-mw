//! The instance pool: one set of entities per `InstanceKey`, kept from frame to frame.
//!
//! Each frame the caller lists every placed object. Objects seen before keep their entities and only get a
//! `GlobalTransform` write when their matrix changed (so Bevy's change detection and previous-frame transforms
//! stay meaningful). New objects spawn one entity per draw range of their mesh. Objects that are no longer
//! listed are hidden at once and despawned after [`GRACE_FRAMES`], so tiles that flicker at the edge of view
//! do not churn entities.

use bevy_camera::visibility::{NoFrustumCulling, Visibility};
use bevy_ecs::entity::Entity;
use bevy_ecs::system::{Commands, Query};
use bevy_math::Affine3A;
use bevy_mesh::Mesh3d;
use bevy_pbr::MeshMaterial3d;
use bevy_transform::components::GlobalTransform;
use blackbox_gfx::Instance;

use super::assets::{Stores, material_for};
use super::state::{GRACE_FRAMES, Placed, PoolKey, WorldState};

/// The components the pool changes on entities that already exist.
pub type PlacedQuery<'w, 's> = Query<'w, 's, (&'static mut GlobalTransform, &'static mut Visibility)>;

fn global_transform(m: glam::Mat4) -> GlobalTransform {
    let bevy = bevy_math::Mat4::from_cols_array(&crate::axes::model(m).to_cols_array());
    GlobalTransform::from(Affine3A::from_mat4(bevy))
}

/// Make the pool match `instances`.
pub fn sync(
    state: &mut WorldState,
    stores: &mut Stores,
    commands: &mut Commands,
    placed: &mut PlacedQuery,
    instances: &[Instance],
) {
    state.epoch += 1;
    let epoch = state.epoch;
    let mut slot = 0u32;
    for instance in instances {
        let key = match instance.key.is_transient() {
            true => {
                slot += 1;
                PoolKey::Slot(slot - 1)
            }
            false => PoolKey::Keyed(instance.key.0),
        };
        let mesh = instance.mesh.raw();
        if !state.meshes.contains_key(&mesh) {
            continue;
        }
        match state.pool.get(&key) {
            Some(entry) if entry.mesh == mesh => update(state, placed, key, instance, epoch),
            Some(_) => {
                despawn(state, commands, key);
                spawn(state, stores, commands, key, instance, epoch);
            }
            None => spawn(state, stores, commands, key, instance, epoch),
        }
    }
    sweep(state, commands, placed, epoch);
}

/// An object listed again: move it if it moved, show it if it was hidden.
fn update(state: &mut WorldState, placed: &mut PlacedQuery, key: PoolKey, instance: &Instance, epoch: u64) {
    let Some(entry) = state.pool.get_mut(&key) else { return };
    entry.seen = epoch;
    let moved = entry.transform != instance.transform;
    let was_hidden = entry.hidden_since.take().is_some();
    if !moved && !was_hidden {
        return;
    }
    entry.transform = instance.transform;
    let transform = global_transform(instance.transform);
    for &entity in &entry.entities {
        let Ok((mut global, mut visibility)) = placed.get_mut(entity) else { continue };
        if moved {
            *global = transform;
        }
        if was_hidden {
            *visibility = Visibility::Visible;
        }
    }
}

fn spawn(
    state: &mut WorldState,
    stores: &mut Stores,
    commands: &mut Commands,
    key: PoolKey,
    instance: &Instance,
    epoch: u64,
) {
    let mesh = instance.mesh.raw();
    let transform = global_transform(instance.transform);
    let ranges: Vec<_> =
        state.meshes[&mesh].ranges.iter().map(|r| (r.mesh.clone(), r.texture, r.blend, r.shading)).collect();
    let mut entities: Vec<Entity> = Vec::with_capacity(ranges.len());
    for (bevy_mesh, texture, blend, shading) in ranges {
        let material = material_for(state, stores, texture, blend, shading);
        let entity = commands
            .spawn((Mesh3d(bevy_mesh), MeshMaterial3d(material), transform, Visibility::Visible, NoFrustumCulling))
            .id();
        entities.push(entity);
    }
    state.pool.insert(key, Placed { mesh, entities, transform: instance.transform, seen: epoch, hidden_since: None });
}

fn despawn(state: &mut WorldState, commands: &mut Commands, key: PoolKey) {
    let Some(entry) = state.pool.remove(&key) else { return };
    for entity in entry.entities {
        commands.entity(entity).despawn();
    }
}

/// Hide what was not listed this frame, and despawn what has been hidden for the grace period.
fn sweep(state: &mut WorldState, commands: &mut Commands, placed: &mut PlacedQuery, epoch: u64) {
    let mut gone = Vec::new();
    for (&key, entry) in &mut state.pool {
        if entry.seen == epoch {
            continue;
        }
        let since = *entry.hidden_since.get_or_insert(epoch);
        if since == epoch {
            for &entity in &entry.entities {
                if let Ok((_, mut visibility)) = placed.get_mut(entity) {
                    *visibility = Visibility::Hidden;
                }
            }
            continue;
        }
        if epoch - since > GRACE_FRAMES {
            gone.push(key);
        }
    }
    for key in gone {
        despawn(state, commands, key);
    }
}

/// How many objects have entities, and how many entities that is (for stats and tests).
pub fn counts(state: &WorldState) -> (usize, usize) {
    (state.pool.len(), state.pool.values().map(|p| p.entities.len()).sum())
}
