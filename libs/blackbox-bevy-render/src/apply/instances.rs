//! The instance pool: one set of entities per `InstanceKey`, kept from frame to frame.
//!
//! Each frame the caller lists every placed object. Objects seen before keep their entities and only get a
//! `GlobalTransform` write when their matrix changed (so Bevy's change detection and previous-frame transforms
//! stay meaningful). New objects spawn one entity per draw range of their mesh. Objects that are no longer
//! listed are hidden at once and despawned after [`GRACE_FRAMES`], so tiles that flicker at the edge of view
//! do not churn entities.

use bevy_asset::{Assets, Handle};
use bevy_camera::visibility::Visibility;
use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Commands, Query, ResMut, SystemParam};
use bevy_math::Affine3A;
use bevy_mesh::Mesh3d;
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_transform::components::GlobalTransform;
use blackbox_gfx::Instance;
use std::collections::HashMap;

use crate::material::{BlackboxMaterial, BlendKind, ShadingKind};
use crate::ops::Op;

use super::assets::{Stores, material_for};
#[cfg(feature = "ray-tracing")]
use super::redirects;
use super::state::{GRACE_FRAMES, Placed, PoolKey, WorldState};

/// The components the pool changes on entities that already exist.
pub type PlacedQuery<'w, 's> = Query<'w, 's, (&'static mut GlobalTransform, &'static mut Visibility)>;

/// A stand-in's key: its texture number (`None` for the white texture) and its quantised colour (see `solari.rs`).
pub type StandInKey = (Option<usize>, [u8; 3]);

/// The Solari stand-ins by [`StandInKey`] (see `solari.rs`). The resource exists only when Solari was added at
/// startup, so its presence is what turns the stand-in path on.
#[derive(Resource, Default)]
pub struct StandInCache(pub HashMap<StandInKey, Handle<StandardMaterial>>);

/// What the stand-in path needs, borrowed for one tick: the material store and the cache.
pub struct StandIns<'a> {
    pub standard: &'a mut Assets<StandardMaterial>,
    pub cache: &'a mut StandInCache,
}

/// What the apply system writes to the pool: the placed entities, and, when Solari is on, the stand-in stores.
#[derive(SystemParam)]
pub struct Surfaces<'w, 's> {
    pub placed: PlacedQuery<'w, 's>,
    pub standard: ResMut<'w, Assets<StandardMaterial>>,
    pub cache: Option<ResMut<'w, StandInCache>>,
}

/// Solari needs its meshes prepared before they are added; without it the op passes through unchanged.
#[cfg(feature = "ray-tracing")]
pub fn prepare_op(op: Op, solari: bool) -> Op {
    match solari {
        true => crate::solari::prepare_op(op),
        false => op,
    }
}

/// Without Solari there is nothing to prepare.
#[cfg(not(feature = "ray-tracing"))]
pub fn prepare_op(op: Op, _solari: bool) -> Op {
    op
}

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
    mut stand_ins: Option<&mut StandIns>,
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
                spawn(state, stores, commands, key, instance, epoch, stand_ins.as_deref_mut());
            }
            None => spawn(state, stores, commands, key, instance, epoch, stand_ins.as_deref_mut()),
        }
    }
    sweep(state, commands, placed, epoch);
    log_batch_stats(state, epoch);
}

/// Log the per-frame batch split (every 120 frames) so we know whether batches are material- or mesh-driven.
fn log_batch_stats(state: &WorldState, epoch: u64) {
    if !log::log_enabled!(log::Level::Debug) || !epoch.is_multiple_of(120) {
        return;
    }
    use std::collections::{HashMap, HashSet};
    type MeshId = bevy_asset::AssetId<bevy_mesh::Mesh>;
    type MaterialId = bevy_asset::AssetId<BlackboxMaterial>;
    let mut per_batch: HashMap<(MeshId, MaterialId), usize> = HashMap::new();
    let mut meshes: HashSet<MeshId> = HashSet::new();
    let mut materials: HashSet<MaterialId> = HashSet::new();
    let mut draws = 0usize;
    for entry in state.pool.values() {
        if entry.seen != epoch {
            continue;
        }
        for &(mesh_id, material_id) in &entry.batches {
            draws += 1;
            *per_batch.entry((mesh_id, material_id)).or_insert(0) += 1;
            meshes.insert(mesh_id);
            materials.insert(material_id);
        }
    }
    let mut sizes: Vec<usize> = per_batch.values().copied().collect();
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    let top: Vec<usize> = sizes.into_iter().take(3).collect();
    log::debug!(
        "batch-stats: draws={draws} batches={} meshes={} materials={} top_batch_sizes={top:?}",
        per_batch.len(),
        meshes.len(),
        materials.len(),
    );
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

/// What draws one draw range: the Blackbox material, or a Solari stand-in while ray tracing is on.
enum Surface {
    Blackbox(Handle<BlackboxMaterial>),
    #[cfg(feature = "ray-tracing")]
    Solari(Handle<StandardMaterial>),
}

/// What one draw range is drawn with, from its registered range.
struct Draw {
    texture: Option<usize>,
    blend: BlendKind,
    shading: ShadingKind,
    glossy: Option<usize>,
    /// Read by the Solari stand-in only.
    #[cfg_attr(not(feature = "ray-tracing"), allow(dead_code))]
    colour: [u8; 3],
}

/// The surface of a draw. An opaque, non-sky draw becomes a stand-in when Solari is on (`solari.rs`).
#[cfg(feature = "ray-tracing")]
fn surface_for(state: &mut WorldState, stores: &mut Stores, stand_ins: Option<&mut StandIns>, draw: &Draw) -> Surface {
    if let Some(stand_ins) = stand_ins {
        let key = draw.texture.map(|t| redirects::resolve(state, t));
        let image = key.and_then(|k| state.textures.get(&k).cloned());
        if let Some(material) = crate::solari::stand_in(stand_ins, image, key, draw.colour, draw.blend, draw.shading) {
            return Surface::Solari(material);
        }
    }
    Surface::Blackbox(material_for(state, stores, draw.texture, draw.blend, draw.shading, draw.glossy))
}

/// Without Solari every draw uses the Blackbox material.
#[cfg(not(feature = "ray-tracing"))]
fn surface_for(state: &mut WorldState, stores: &mut Stores, _stand_ins: Option<&mut StandIns>, draw: &Draw) -> Surface {
    Surface::Blackbox(material_for(state, stores, draw.texture, draw.blend, draw.shading, draw.glossy))
}

fn spawn(
    state: &mut WorldState,
    stores: &mut Stores,
    commands: &mut Commands,
    key: PoolKey,
    instance: &Instance,
    epoch: u64,
    mut stand_ins: Option<&mut StandIns>,
) {
    let mesh = instance.mesh.raw();
    let transform = global_transform(instance.transform);
    let ranges: Vec<_> = state.meshes[&mesh]
        .ranges
        .iter()
        .map(|r| {
            let draw =
                Draw { texture: r.texture, blend: r.blend, shading: r.shading, glossy: r.glossy, colour: r.colour };
            (r.mesh.clone(), r.aabb, draw)
        })
        .collect();
    let mut entities: Vec<Entity> = Vec::with_capacity(ranges.len());
    let mut batches = Vec::with_capacity(ranges.len());
    for (bevy_mesh, aabb, draw) in ranges {
        let surface = surface_for(state, stores, stand_ins.as_deref_mut(), &draw);
        let shading = draw.shading;
        let blend = draw.blend;
        let mesh_id = bevy_mesh.id();
        let (entity, batch) = match surface {
            Surface::Blackbox(material) => {
                let material_id = material.id();
                let entity = commands
                    .spawn((Mesh3d(bevy_mesh), MeshMaterial3d(material), aabb, transform, Visibility::Visible))
                    .id();
                (entity, Some((mesh_id, material_id)))
            }
            #[cfg(feature = "ray-tracing")]
            Surface::Solari(material) => {
                (crate::solari::spawn_stand_in(commands, bevy_mesh, material, aabb, transform), None)
            }
        };
        if !super::shadows::casts(shading, blend) || !instance.shadow.cast {
            commands.entity(entity).insert(bevy_light::NotShadowCaster);
        }
        if !instance.shadow.receive {
            commands.entity(entity).insert(bevy_light::NotShadowReceiver);
        }
        entities.push(entity);
        batches.extend(batch);
    }
    state.pool.insert(
        key,
        Placed { mesh, entities, batches, transform: instance.transform, seen: epoch, hidden_since: None },
    );
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
