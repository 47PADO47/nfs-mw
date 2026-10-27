//! Placing a section's scenery in two halves. The first needs no renderer: which instances are in, their transforms,
//! bounds, collision shapes and lamps. A loader thread can do it ([`worker_hook`]) while the frame thread draws.
//! The second joins that to the meshes the frame thread has uploaded ([`resolve_placed`]).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use blackbox_gfx::InstanceKey;
use blackbox_scene::Aabb;
use glam::Mat4;
use nfsmw_data::world::{Prepare, PropCatalog, PropShape, SectionData};

use super::reflections::{PreparedReflection, prepare_reflections};
use super::{Placed, SKIPPED_MODELS, SectionResources, hidden_group, shadow_policy};
use crate::scenes::world::ai::signal_props::{SignalHead, signal_head};
use crate::scenes::world::props::scenery::ModelParts;

/// One scenery instance that is in the world, without its meshes.
pub struct PreparedInstance {
    /// The solid of each LOD, 0 for none.
    keys: Vec<u32>,
    /// The model is an authored shadow overlay by its name.
    shd_name: bool,
    hierarchy: Option<Arc<blackbox_scenery::ModelHierarchy>>,
    transform: Mat4,
    bounds: Aabb,
    radius: f32,
    flags: u32,
    group: u16,
    prop: Option<Arc<PropShape>>,
    spawner: Option<blackbox_scenery::ObjectSpawner>,
    signal: Option<SignalHead>,
    casts_sun_shadow: bool,
}

/// What a section's placement needs of the catalog and the groups, computed without a renderer. It is what a
/// loader thread leaves in `SectionData::prepared`.
pub struct Prepared {
    pub(in crate::scenes::world) placed: Vec<PreparedInstance>,
    pub(in crate::scenes::world) reflections: Vec<PreparedReflection>,
}

/// The renderer-free half of placing `data`'s scenery: the instances the player view draws, and those a scenery group
/// keeps while hidden (`groups` maps an override index to its group, see `WorldIndex::override_groups`).
pub fn prepare(data: &SectionData, props: &PropCatalog, groups: &HashMap<u16, u16>) -> Prepared {
    Prepared { placed: prepare_placed(data, props, groups), reflections: prepare_reflections(data) }
}

/// The hook a stream loader runs on each section it parses: it leaves [`prepare`]'s result in the section. The loaders
/// share `catalog`, which caches shapes, behind a lock.
pub fn worker_hook(catalog: PropCatalog, groups: HashMap<u16, u16>) -> Box<Prepare> {
    let catalog = Mutex::new(catalog);
    Box::new(move |data: &mut SectionData| {
        let catalog = catalog.lock().unwrap_or_else(PoisonError::into_inner);
        let prepared = prepare(data, &catalog, &groups);
        data.prepared = Some(Box::new(prepared));
    })
}

/// The preparation a loader thread left in `data`, or made here when it left none (the loader had no catalog yet).
pub fn take_prepared(data: &mut SectionData, props: &PropCatalog, groups: &HashMap<u16, u16>) -> Prepared {
    let left = data.prepared.take().and_then(|boxed| boxed.downcast::<Prepared>().ok());
    match left {
        Some(prepared) => *prepared,
        None => prepare(data, props, groups),
    }
}

fn prepare_placed(data: &SectionData, props: &PropCatalog, groups: &HashMap<u16, u16>) -> Vec<PreparedInstance> {
    let instances: usize = data.scenery.iter().map(|s| s.instances.len()).sum();
    let mut prepared = Vec::with_capacity(instances);
    let mut spawners = HashMap::with_capacity(data.spawners.len());
    spawners.extend(data.spawners.iter().map(|s| (s.override_index, s)));
    for section in &data.scenery {
        let mut hooks = HashMap::with_capacity(section.override_hooks.len());
        hooks.extend(
            section.override_hooks.iter().map(|h| (usize::from(h.instance_index), u32::from(h.override_index))),
        );
        for (instance, inst) in section.instances.iter().enumerate() {
            let Some(info) = section.info_of(inst) else { continue };
            let rules = &blackbox_scenery::layout::MOST_WANTED.visibility;
            let hidden = !inst.visible_in(rules.player_view, rules);
            let group = hidden_group(hidden, hooks.get(&instance).copied(), groups);
            if (hidden && group == 0) || SKIPPED_MODELS.contains(&info.name.as_str()) {
                continue;
            }
            let transform = Mat4::from_cols_array_2d(&inst.matrix_columns());
            let spawner = hooks.get(&instance).and_then(|id| spawners.get(id)).copied();
            // Static solidity comes from authored world collision, not a model-name bounds guess.
            let prop = spawner.and_then(|spawn| props.for_spawner(&info.name, spawn));
            if spawner.is_some() && prop.is_none() {
                log::debug!("no physics bounds/attributes for placed {}", info.name);
            }
            let bounds = Aabb::new(inst.bbox_min, inst.bbox_max);
            prepared.push(PreparedInstance {
                keys: info.solid_keys.clone(),
                shd_name: info.name.starts_with("SHD_"),
                hierarchy: props.hierarchy(info.hierarchy_hash),
                transform,
                bounds,
                radius: info.radius,
                flags: inst.exclude_flags,
                group,
                prop,
                spawner: spawner.copied(),
                signal: signal_head(&info.name, transform),
                casts_sun_shadow: shadow_policy::casts_sun(&info.name),
            });
        }
    }
    prepared
}

/// Joins prepared instances to the meshes in `own` and `shared`: the placed objects, and how many instances had no
/// mesh at all (they are dropped).
pub fn resolve_placed(
    prepared: &[PreparedInstance],
    own: &SectionResources,
    shared: &SectionResources,
) -> (Vec<Placed>, usize) {
    let mut placed = Vec::with_capacity(prepared.len());
    let mut unresolved = 0;
    for item in prepared {
        let resolved: Vec<_> = item.keys.iter().map(|&k| (k != 0).then(|| own.mesh(k, shared)).flatten()).collect();
        if resolved.iter().all(Option::is_none) {
            unresolved += 1;
            continue;
        }
        let traits =
            |i: usize| own.meshes.get(&item.keys[i]).or_else(|| shared.meshes.get(&item.keys[i])).map(|model| model.2);
        let from_overlay = |i: usize| traits(i).is_some_and(|t| t.shadow_overlay);
        let sway_rules = &blackbox_scenery::layout::MOST_WANTED.sway;
        let sway_lods = match sway_rules.instance_sways(item.flags) {
            true => {
                (0..resolved.len().min(4)).filter(|&i| traits(i).is_some_and(|t| t.sways)).fold(0, |m, i| m | 1 << i)
            }
            false => 0,
        };
        placed.push(Placed {
            casts_sun_shadow: item.casts_sun_shadow,
            original_shadow: item.shd_name
                || resolved.iter().enumerate().filter(|(_, mesh)| mesh.is_some()).all(|(i, _)| from_overlay(i)),
            lods: std::array::from_fn(|i| resolved.get(i).copied().flatten().map(|(m, _)| m)),
            detailed: resolved.first().copied().flatten().map(|(_, lod)| lod),
            transform: item.transform,
            bounds: item.bounds,
            radius: item.radius,
            flags: item.flags,
            group: item.group,
            prop: item.prop.clone(),
            prop_id: 0,
            key: InstanceKey::TRANSIENT,
            spawner: item.spawner,
            signal: item.signal,
            sway_lods,
            parts: item.hierarchy.clone().map(|hierarchy| {
                let meshes: std::collections::BTreeMap<_, _> = hierarchy
                    .nodes
                    .iter()
                    .enumerate()
                    .filter_map(|(i, n)| Some((i, own.mesh(n.model_hash, shared)?.0)))
                    .collect();
                Arc::new(ModelParts { available: meshes.keys().copied().collect(), hierarchy, meshes })
            }),
        });
    }
    (placed, unresolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_section_prepares_and_resolves_to_nothing() {
        let data = SectionData {
            index: 0,
            solids: Vec::new(),
            textures: Vec::new(),
            decoded: Vec::new(),
            anims: Vec::new(),
            scrolls: Vec::new(),
            scenery: Vec::new(),
            spawners: Vec::new(),
            emitters: Vec::new(),
            collision: Vec::new(),
            prepared: None,
        };
        let catalog = PropCatalog::new(Vec::new(), &blackbox_attrib::Database::new());
        let prepared = prepare(&data, &catalog, &HashMap::new());
        let own = SectionResources::default();
        let (placed, unresolved) = resolve_placed(&prepared.placed, &own, &SectionResources::default());
        assert!(placed.is_empty());
        assert_eq!(unresolved, 0);
    }

    #[test]
    fn a_preparation_left_by_a_loader_is_the_one_used() {
        let mut data = SectionData {
            index: 0,
            solids: Vec::new(),
            textures: Vec::new(),
            decoded: Vec::new(),
            anims: Vec::new(),
            scrolls: Vec::new(),
            scenery: Vec::new(),
            spawners: Vec::new(),
            emitters: Vec::new(),
            collision: Vec::new(),
            prepared: None,
        };
        let catalog = PropCatalog::new(Vec::new(), &blackbox_attrib::Database::new());
        let hook = worker_hook(PropCatalog::new(Vec::new(), &blackbox_attrib::Database::new()), HashMap::new());
        hook(&mut data);
        assert!(data.prepared.is_some(), "the hook leaves its result in the section");
        let prepared = take_prepared(&mut data, &catalog, &HashMap::new());
        assert!(prepared.placed.is_empty());
        assert!(data.prepared.is_none(), "taking it empties the slot");
        // Without one the frame thread makes it.
        let again = take_prepared(&mut data, &catalog, &HashMap::new());
        assert!(again.placed.is_empty());
    }
}
