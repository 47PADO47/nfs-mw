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
    let materials = CarMaterials::upload_with_background_policy(renderer, model, shading, to_sun, background_actor);
    let (parts, meshes) = upload_solids(renderer, model, swaps, &materials, None);
    (parts, meshes, materials)
}

/// Upload the placed solids of `model` drawn with `materials`: all of them, or only those in `only`. The parts
/// are sorted by mesh, as the renderer wants instances of one mesh together.
pub(super) fn upload_solids(
    renderer: &mut dyn RenderBackend,
    model: &CarModel,
    swaps: &TextureSwaps,
    materials: &CarMaterials,
    only: Option<&HashSet<u32>>,
) -> (Vec<Part>, Vec<(u32, MeshHandle)>) {
    let mut cache: HashMap<(u32, bool, u8), Option<MeshHandle>> = HashMap::new();
    // A state is the headlights (bit 0), the brake (bit 1) and the reverse gear (bit 2), which only a mod car draws.
    let variants: [_; 8] = std::array::from_fn(|state| {
        swaps
            .clone()
            .with_lamp_state(state & 1 != 0, state & 2 != 0, &model.textures)
            .with_reverse_state(state & 4 != 0, &model.textures)
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
            if only.is_some_and(|only| !only.contains(&p.solid)) {
                continue;
            }
            let solid = &model.solids[&p.solid];
            let lamp_material = materials.enhanced_lamps_for(&solid.name);
            let relevant = solid.groups.iter().filter_map(|g| g.diffuse_texture(solid)).fold(0, |mask, hash| {
                let head = variants[0].resolve_for(hash, p.left_brake) != variants[1].resolve_for(hash, p.left_brake)
                    || (lamp_material && variants[0].lamp_is_on(hash) != variants[1].lamp_is_on(hash));
                let brake = variants[0].resolve_for(hash, p.left_brake) != variants[2].resolve_for(hash, p.left_brake)
                    || (lamp_material && variants[0].lamp_is_on(hash) != variants[2].lamp_is_on(hash));
                let reverse = variants[0].resolve_for(hash, p.left_brake)
                    != variants[4].resolve_for(hash, p.left_brake)
                    || (lamp_material && variants[0].lamp_is_on(hash) != variants[4].lamp_is_on(hash));
                mask | u8::from(head) | (u8::from(brake) << 1) | (u8::from(reverse) << 2)
            });
            let lamps: [_; 8] = std::array::from_fn(|state| {
                *cache.entry((p.solid, p.left_brake, state as u8 & relevant)).or_insert_with(|| {
                    let lookup = materials.for_solid(&variants[state], p.left_brake, &solid.name);
                    upload_solid(renderer, solid, &lookup, Shading::Lit)
                })
            });
            let Some(mesh) = lamps[1] else { continue };
            parts.push(Part { mesh, lamps: lamps.map(|m| m.unwrap_or(mesh)), placement: p.clone(), key, lod, damaged });
        }
    }
    parts.sort_by_key(|p| p.mesh);
    (parts, cache.into_iter().filter_map(|((solid, _, _), mesh)| mesh.map(|m| (solid, m))).collect())
}
