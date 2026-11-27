//! Shared upload cache for all authored driving levels.
use super::*;

/// Upload the placed solids of `model` with `shading`: the parts sorted by mesh (the renderer wants instances
/// of one mesh together), every mesh uploaded and the car's materials.
pub(super) fn upload_parts(
    renderer: &mut dyn RenderBackend,
    model: &CarModel,
    swaps: &TextureSwaps,
    shading: CarShading,
    to_sun: Vec3,
    background_actor: bool,
) -> (Vec<Part>, Vec<(u32, MeshHandle)>, CarMaterials) {
    let materials = {
        let _trace = blackbox_trace::scope("Car upload: materials and textures");
        CarMaterials::upload_with_background_policy(renderer, model, shading, to_sun, background_actor)
    };
    let _trace = blackbox_trace::scope("Car upload: solids");
    let (parts, meshes) = upload_solids(renderer, model, swaps, &materials, None, background_actor);
    (parts, meshes, materials)
}

/// Upload the placed solids of `model` drawn with `materials`: all of them, or only those in the damage variant. The parts
/// are sorted by mesh, as the renderer wants instances of one mesh together.
pub(super) fn upload_solids(
    renderer: &mut dyn RenderBackend,
    model: &CarModel,
    swaps: &TextureSwaps,
    materials: &CarMaterials,
    damage: Option<(&HashSet<u32>, &HashSet<u32>)>,
    background_actor: bool,
) -> (Vec<Part>, Vec<(u32, MeshHandle)>) {
    let mut cache: HashMap<(u32, bool, u8), Option<MeshHandle>> = HashMap::new();
    // Independent headlight, brake, reverse and cosmetic NOS bits. Equivalent material states share a mesh.
    let variants: [_; 16] = std::array::from_fn(|state| {
        let head = state & 1 != 0;
        let brake = state & 2 != 0;
        let current = swaps.clone().with_visual_lamp_state(head, brake, state & 8 != 0, &model.textures);
        let current = match !background_actor && materials.enhanced_lamps() {
            true => current.with_running_tail_surfaces(head, brake, &model.textures),
            false => current,
        };
        current.with_reverse_state(state & 4 != 0, &model.textures)
    });
    let mut parts = Vec::new();
    let levels = std::iter::once((0, model.placements.as_slice(), false))
        .chain(model.driving_lods.levels.iter().map(|l| (l.level, l.placements.as_slice(), false)))
        .chain(model.driving_lods.damage.levels.iter().map(|l| (l.level, l.placements.as_slice(), true)));
    for (lod, placements, damaged) in levels {
        for (index, p) in placements.iter().enumerate() {
            let key = match lod {
                0 if !damaged => index,
                _ => model
                    .placements
                    .iter()
                    .position(|canonical| canonical.slot == p.slot && canonical.corner == p.corner)
                    .unwrap_or(model.placements.len() + p.slot * 8 + p.corner.map_or(0, |c| c + 1)),
            };
            if damage.is_some_and(|(only, _)| !only.contains(&p.solid)) {
                continue;
            }
            let solid = &model.solids[&p.solid];
            let window_textures: Vec<_> = match damage {
                Some((_, cracked)) if cracked.contains(&p.solid) => solid
                    .groups
                    .iter()
                    .filter(|g| {
                        solid.light_material_hashes.get(usize::from(g.light_material))
                            != Some(&blackbox_hash::bstring_hash("WINDOWMASK"))
                    })
                    .filter_map(|g| g.diffuse_texture(solid))
                    .collect(),
                _ => Vec::new(),
            };
            let lamp_material = materials.enhanced_lamps_for(&solid.name);
            let relevant = solid.groups.iter().filter_map(|g| g.diffuse_texture(solid)).fold(0, |mask, hash| {
                let head = variants[0].resolve_for(hash, p.left_brake) != variants[1].resolve_for(hash, p.left_brake)
                    || (lamp_material && variants[0].lamp_is_on(hash) != variants[1].lamp_is_on(hash));
                let brake = variants[0].resolve_for(hash, p.left_brake) != variants[2].resolve_for(hash, p.left_brake)
                    || (lamp_material && variants[0].lamp_is_on(hash) != variants[2].lamp_is_on(hash));
                let reverse = variants[0].resolve_for(hash, p.left_brake)
                    != variants[4].resolve_for(hash, p.left_brake)
                    || (lamp_material && variants[0].lamp_is_on(hash) != variants[4].lamp_is_on(hash));
                let rear = variants[0].resolve_for(hash, p.left_brake) != variants[8].resolve_for(hash, p.left_brake)
                    || (lamp_material && variants[0].lamp_is_on(hash) != variants[8].lamp_is_on(hash));
                mask | u8::from(head) | (u8::from(brake) << 1) | (u8::from(reverse) << 2) | (u8::from(rear) << 3)
            });
            let lamps: [_; 16] = std::array::from_fn(|state| {
                let canonical = (0..state)
                    .find(|&other| {
                        solid.groups.iter().filter_map(|g| g.diffuse_texture(solid)).all(|hash| {
                            variants[state].resolve_for(hash, p.left_brake)
                                == variants[other].resolve_for(hash, p.left_brake)
                                && (!lamp_material
                                    || variants[state].lamp_is_on(hash) == variants[other].lamp_is_on(hash))
                        })
                    })
                    .unwrap_or(state);
                *cache.entry((p.solid, p.left_brake, canonical as u8 & relevant)).or_insert_with(|| {
                    let pane_swaps;
                    let swaps = match window_textures.is_empty() {
                        true => &variants[state],
                        false => {
                            pane_swaps = variants[state].clone().with_window_hashes(&window_textures);
                            &pane_swaps
                        }
                    };
                    let lookup = materials.for_solid(swaps, p.left_brake, &solid.name, p.slot);
                    upload_solid(renderer, solid, &lookup, Shading::Lit)
                })
            });
            let Some(mesh) = lamps[1] else { continue };
            parts.push(Part { mesh, lamps: lamps.map(|m| m.unwrap_or(mesh)), placement: p.clone(), key, lod, damaged });
        }
    }
    parts.sort_by_key(|p| p.mesh);
    let uploaded: Vec<_> = cache.iter().filter(|(_, m)| m.is_some()).map(|((solid, _, _), _)| *solid).collect();
    let vertices: usize = uploaded.iter().map(|s| model.solids[s].vertices.len()).sum();
    let distinct: std::collections::HashSet<_> = uploaded.iter().collect();
    blackbox_trace::global().note(
        "Car solids uploaded",
        &format!("{} meshes of {} solids, {vertices} vertices", uploaded.len(), distinct.len()),
    );
    (parts, cache.into_iter().filter_map(|((solid, _, _), mesh)| mesh.map(|m| (solid, m))).collect())
}
