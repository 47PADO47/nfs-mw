//! Installed road/window routing at the donut-shop reference, without a GPU or driving.

use super::super::resident::SectionResources;
use crate::app::test_backend::NullBackend;
use blackbox_gfx::{NormalModel, Shading};
use blackbox_solid::Solid;
use game_install::GameDir;
use glam::{Mat4, Vec2, Vec3};
use nfsmw_data::world::{DEFAULT_TRACK, SectionData, WorldIndex};
use std::collections::HashSet;

const ROAD: &str = "TRN_CT_ROAD_COLONIAL_01_CHOP_H7_R1";
const SHOP: &str = "XB_DONUTSHOP_1B_BK_00";
const WINDOW_REFLECTION: u32 = 0x7E16_A782;
const POINT: Vec3 = Vec3::new(1945.0, 1850.0, 149.9);

fn load(dir: &GameDir, index: &WorldIndex, name: &str) -> SectionData {
    let i = index.by_name(name).unwrap();
    let section = &index.sections[i];
    let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
    nfsmw_data::world::parse_section(i, &bytes).unwrap()
}

fn vertical_hit(a: Vec3, b: Vec3, c: Vec3, p: Vec2) -> Option<f32> {
    let det = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
    if det.abs() < 1e-8 {
        return None;
    }
    let u = ((b.y - c.y) * (p.x - c.x) + (c.x - b.x) * (p.y - c.y)) / det;
    let v = ((c.y - a.y) * (p.x - c.x) + (a.x - c.x) * (p.y - c.y)) / det;
    if u.min(v).min(1.0 - u - v) < -1e-5 {
        return None;
    }
    Some(u * a.z + v * b.z + (1.0 - u - v) * c.z)
}

fn assert_actual_road(tile: &SectionData, road: &Solid) {
    let mut hits = Vec::new();
    for section in &tile.scenery {
        for inst in &section.instances {
            let Some(info) = section.info_of(inst) else { continue };
            if info.solid_keys.first().copied() != Some(road.name_hash) {
                continue;
            }
            let transform = Mat4::from_cols_array_2d(&inst.matrix_columns());
            for (group_index, group) in road.groups.iter().enumerate() {
                let first = group.first_index as usize;
                let indices = &road.indices[first..first + group.num_indices as usize];
                for triangle in indices.as_chunks::<3>().0 {
                    let points: [Vec3; 3] = std::array::from_fn(|i| {
                        let index = group.base_vertex as usize + usize::from(triangle[i]);
                        transform.transform_point3(Vec3::from_array(road.vertices[index].position))
                    });
                    if let Some(z) = vertical_hit(points[0], points[1], points[2], POINT.truncate()) {
                        hits.push((group_index, z));
                    }
                }
            }
        }
    }
    assert_eq!(hits.len(), 1, "the selected solid must actually cover the reference point");
    assert_eq!(hits[0].0, 2);
    assert!((hits[0].1 - 149.79305).abs() < 0.002);
    assert!((hits[0].1 - POINT.z).abs() < 0.2);
    assert_eq!(road.groups[2].effect_id, 1);
    assert_eq!(road.vertex_buffers[road.groups[2].vertex_buffer as usize].stride, 60);
    let group = &road.groups[2];
    let hashes = [0, 1, 3].map(|slot| road.texture_hashes[group.texture_slots[slot] as usize]);
    assert_eq!(hashes, [0xA040_5C3D, 0x1903_5965, 0xF80A_8327]);
}

#[test]
#[ignore = "needs the original installed city selected by NFSMW_GAME_DIR"]
fn installed_original_road_and_donut_windows_resolve_materials_and_release() {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR");
    let dir = GameDir::open(path).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut tile = load(&dir, &index, "H47");
    let mut models = load(&dir, &index, "X0");
    tile.solids.retain(|s| s.name == ROAD);
    models.solids.retain(|s| s.name == SHOP);
    assert_eq!((tile.solids.len(), models.solids.len()), (1, 1));
    let road = &tile.solids[0];
    assert_actual_road(&tile, road);
    let shop = &models.solids[0];
    assert_eq!([shop.groups[1].effect_id, shop.groups[3].effect_id], [5, 5]);
    // These real groups alias every authored texture slot to the base image.
    // The original glass updater binds its reflection globally instead.
    assert_eq!(shop.groups[1].texture_slots, [2; 5]);
    assert_eq!(shop.groups[3].texture_slots, [3; 5]);
    let mut needed: HashSet<_> = road.texture_hashes.iter().chain(&shop.texture_hashes).copied().collect();
    needed.insert(WINDOW_REFLECTION);
    let mut textures = load(&dir, &index, "Y0").textures;
    let globals = nfsmw_data::read_unwrapped(&dir, "GLOBAL/INGAMEA.BUN").unwrap();
    for pack in blackbox_tpk::read_texture_packs(&globals).unwrap() {
        textures.extend(pack.textures);
    }
    textures.retain(|t| needed.contains(&t.name_hash));
    assert!(needed.iter().all(|hash| textures.iter().any(|t| t.name_hash == *hash)), "fixture texture completeness");
    let mut renderer = NullBackend::new(crate::settings::test_caps::native())
        .with_normal_tracking()
        .with_mesh_tracking()
        .with_texture_tracking();
    let mut shared = SectionResources::default();
    shared.upload_texture_list(&mut renderer, &textures);
    shared.upload_solid_list(&mut renderer, &models.solids, None, &SectionResources::default(), false);
    let mut local = SectionResources::default();
    local.upload_solid_list(&mut renderer, &tile.solids, None, &shared, false);
    let road_mesh = local.meshes[&road.name_hash].0;
    for (i, group) in road.groups.iter().enumerate() {
        let Shading::WorldNormal(handle) = renderer.mesh_draws[&road_mesh][i].shading else {
            panic!("actual road group {i} fell back from its original reflected family");
        };
        let desc = renderer.normal_descriptors[&handle];
        assert_eq!(desc.model, NormalModel::Reflective);
        let expected =
            [0, 1, 3].map(|slot| shared.materials[&road.texture_hashes[group.texture_slots[slot] as usize]].0);
        assert_eq!([desc.diffuse, desc.normal, desc.specular], expected);
    }
    let uploaded = &renderer.mesh_tangents[&road_mesh];
    assert_eq!(uploaded.len(), road.tangents.len());
    assert!(road.tangents.iter().any(|t| t[..3].iter().any(|v| v.abs() > 0.01)), "ID1 authored tangent decoding");
    for (i, (actual, authored)) in uploaded.iter().zip(&road.tangents).enumerate() {
        assert_eq!(actual.map(f32::to_bits), authored.map(f32::to_bits), "road tangent {i}");
    }
    let shop_mesh = shared.meshes[&shop.name_hash].0;
    let reflection = shared.materials[&WINDOW_REFLECTION].0;
    for i in [1, 3] {
        let Shading::WorldWindow(handle) = renderer.mesh_draws[&shop_mesh][i].shading else {
            panic!("actual donut-shop glass group {i} fell back from its original family");
        };
        let desc = renderer.window_descriptors[&handle];
        let diffuse = shared.materials[&shop.texture_hashes[shop.groups[i].texture_slots[0] as usize]].0;
        assert_eq!(desc.diffuse, diffuse);
        assert_eq!(desc.reflection, reflection);
        assert_ne!(desc.diffuse, desc.reflection);
    }
    let before = renderer.resources.lock().unwrap();
    let counts = (before.normal_materials, before.window_materials, before.meshes, before.textures);
    drop(before);
    let lookup = |hash| shared.materials.get(&hash).copied();
    let _ = super::normal_materials(&mut renderer, road, &lookup, &mut local.normal_materials);
    let _ = super::window_materials(&mut renderer, shop, &lookup, &mut shared.window_materials);
    let after = renderer.resources.lock().unwrap();
    assert_eq!((after.normal_materials, after.window_materials, after.meshes, after.textures), counts, "cache reuse");
    drop(after);
    local.release_normal_materials(&mut renderer);
    local.release_normal_materials(&mut renderer);
    shared.release_normal_materials(&mut renderer);
    shared.release_normal_materials(&mut renderer);
    local.release(&mut renderer);
    shared.release(&mut renderer);
    let final_counts = renderer.resources.lock().unwrap();
    assert_eq!(
        (final_counts.normal_materials, final_counts.window_materials, final_counts.meshes, final_counts.textures),
        (0, 0, 0, 0)
    );
    assert_eq!((final_counts.normal_destructions, final_counts.window_destructions), (counts.0, counts.1));
}

#[test]
#[ignore = "needs installed time-of-day attributes selected by NFSMW_GAME_DIR"]
fn installed_original_world_surface_light_changes_with_authored_time_of_day() {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR");
    let dir = GameDir::open(path).unwrap();
    let mut scene = super::super::WorldScene::open(
        &dir,
        super::super::Options {
            at: Some([POINT.x, POINT.y]),
            height: 1.0,
            heading: 0.0,
            pitch: 0.0,
            fog_distance: 1500.0,
            wait_for_load: false,
            drive: None,
        },
    )
    .unwrap();
    scene.sun.set_phase(0.5);
    let noon = scene.surface_light().expect("installed original surface lighting");
    let noon_sun = scene.sun.to_sun();
    scene.sun.set_phase(0.01);
    let dusk = scene.surface_light().expect("installed original surface lighting");
    assert_ne!(noon_sun, scene.sun.to_sun());
    assert_ne!((noon.ambient, noon.diffuse, noon.specular), (dusk.ambient, dusk.diffuse, dusk.specular));
    assert_eq!((noon.ambient_alpha, dusk.ambient_alpha), (4.0, 4.0));
    assert_eq!((noon.specular_power, dusk.specular_power), (11.0, 11.0));
}
