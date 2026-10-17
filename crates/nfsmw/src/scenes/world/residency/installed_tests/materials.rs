//! Real local streaming must resolve all three textures and retain decoded tangents.

use crate::app::test_backend::NullBackend;
use crate::viewer::Scene;
use blackbox_gfx::Shading;
use std::time::{Duration, Instant};

#[test]
#[ignore = "needs the original installed track selected by NFSMW_GAME_DIR"]
fn installed_original_normal_materials_resolve_resident_bindings_and_release() {
    let (dir, mut scene) = super::open();
    let world_index = nfsmw_data::world::WorldIndex::open(&dir, &scene.track).unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native())
        .with_mesh_tracking()
        .with_normal_tracking()
        .with_sky_tracking();
    scene.init(&mut renderer).unwrap();
    let start = Instant::now();
    loop {
        // The ordinary starting area only: no traversal or whole-map loading.
        scene.residency.update(&mut renderer, 333.0, 2517.0);
        if scene.residency.complete() {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(60), "local material residency timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut supported = 0;
    let mut referenced_vertices = 0;
    let mut nonfinite_tangents = 0;
    let mut registered = std::collections::HashSet::new();
    assert!(renderer.resources.lock().unwrap().sky_materials > 0, "real shared residency must connect layered sky");
    for (&index, tile) in &scene.residency.tiles {
        let super::super::TileState::Resident { resources, .. } = tile else { continue };
        let section = &scene.residency.sections[index];
        let bytes =
            dir.read_range(&world_index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        for solid in blackbox_solid::read_solids(&bytes).unwrap() {
            if solid.name.starts_with("SKY") {
                continue;
            }
            for (index, group) in solid.groups.iter().enumerate().filter(|(_, group)| group.effect_id == 3) {
                let buffer = &solid.vertex_buffers[group.vertex_buffer as usize];
                if buffer.stride != 60 {
                    continue;
                }
                supported += 1;
                let handles: Vec<_> = [0, 1, 3]
                    .into_iter()
                    .map(|slot| {
                        let hash = solid
                            .texture_hashes
                            .get(usize::from(group.texture_slots[slot]))
                            .expect("valid authored texture slot");
                        resources
                            .materials
                            .get(hash)
                            .or_else(|| scene.residency.shared.materials.get(hash))
                            .unwrap_or_else(|| panic!("{} missing resident texture {hash:08X}", solid.name))
                            .0
                    })
                    .collect();
                let key = [handles[0], handles[1], handles[2]];
                let handle = *resources
                    .normal_materials
                    .get(&(blackbox_gfx::NormalModel::Bump, key))
                    .expect("complete ID3 inputs must register a material");
                registered.insert(handle);
                let descriptor = &renderer.normal_descriptors[&handle];
                assert_eq!([descriptor.diffuse, descriptor.normal, descriptor.specular], key);
                let mesh = resources.meshes[&solid.name_hash].0;
                assert_eq!(renderer.mesh_draws[&mesh][index].shading, Shading::WorldNormal(handle), "{}", solid.name);
                let uploaded = &renderer.mesh_tangents[&mesh];
                assert_eq!(uploaded.len(), solid.tangents.len(), "{} tangent count", solid.name);
                // Authored payload can contain NaNs: preserve its bits and report those
                // referenced by this draw separately from upload correctness.
                for (vertex, (actual, expected)) in uploaded.iter().zip(&solid.tangents).enumerate() {
                    assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits), "{} tangent {vertex}", solid.name);
                }
                let first = group.first_index as usize;
                for &vertex in &solid.indices[first..first + group.num_indices as usize] {
                    referenced_vertices += 1;
                    let vertex = group.base_vertex as usize + usize::from(vertex);
                    if solid.tangents[vertex].iter().any(|value| !value.is_finite()) {
                        if nonfinite_tangents == 0 {
                            eprintln!("first authored nonfinite tangent: {} vertex {vertex}", solid.name);
                        }
                        nonfinite_tangents += 1;
                    }
                }
                assert!(solid.tangents.iter().any(|value| value[..3].iter().any(|value| value.abs() > 0.01)));
            }
        }
    }
    assert!(supported > 0 && !registered.is_empty(), "the real starting area must exercise normal-mapped geometry");
    let live = renderer.resources.lock().unwrap().normal_materials;
    assert!(live >= registered.len());
    eprintln!(
        "normal material residency: {supported} supported groups, {} registered local triplets, {live} total materials, zero unresolved bindings; {nonfinite_tangents}/{referenced_vertices} indexed tangent references nonfinite in source",
        registered.len()
    );
    scene.close(&mut renderer);
    scene.close(&mut renderer);
    let counts = renderer.resources.lock().unwrap();
    assert_eq!(counts.normal_materials, 0);
    assert_eq!(counts.sky_materials, 0);
    assert_eq!(counts.normal_destructions, live);
    assert!(renderer.normal_descriptors.is_empty());
}
