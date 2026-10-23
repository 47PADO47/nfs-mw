//! Hash replacement must retire the old GPU allocation, including within one upload batch.
use super::*;
use crate::app::test_backend::NullBackend;

#[test]
fn duplicate_hashes_in_one_upload_keep_last_mesh_and_release_every_allocation() {
    for keep_existing in [false, true] {
        let mut renderer = renderer();
        let mut resources = SectionResources::default();
        resources.upload_solid_list(
            &mut renderer,
            &[solid("first", 1), solid("last", 1), solid("other", 2)],
            &SectionResources::default(),
            keep_existing,
        );
        assert_eq!(resources.meshes.len(), 2);
        let last = resources.meshes[&1].0;
        assert_eq!(renderer.mesh_labels[&last], "last");
        assert_eq!(renderer.resources.lock().unwrap().meshes, 2);
        assert_eq!(renderer.resources.lock().unwrap().mesh_destructions, 1);
        resources.release(&mut renderer);
        assert!(renderer.mesh_labels.is_empty());
        assert_eq!(renderer.resources.lock().unwrap().mesh_destructions, 3);
    }
}

#[test]
fn successful_replacement_across_uploads_retires_only_the_old_mesh() {
    let mut renderer = renderer();
    let mut resources = SectionResources::default();
    resources.upload_solid_list(&mut renderer, &[solid("first", 1)], &SectionResources::default(), false);
    let old = resources.meshes[&1].0;
    resources.upload_solid_list(&mut renderer, &[solid("replacement", 1)], &SectionResources::default(), false);
    assert!(!renderer.mesh_labels.contains_key(&old));
    assert_eq!(renderer.resources.lock().unwrap().meshes, 1);
    assert_eq!(renderer.mesh_labels[&resources.meshes[&1].0], "replacement");
    resources.release(&mut renderer);
    assert!(renderer.mesh_labels.is_empty());
    assert_eq!(renderer.resources.lock().unwrap().mesh_destructions, 2);
}

#[test]
fn failed_or_skipped_replacement_keeps_the_existing_allocation_alive() {
    let mut renderer = renderer();
    let mut resources = SectionResources::default();
    resources.upload_solid_list(&mut renderer, &[solid("first", 1)], &SectionResources::default(), false);
    let old = resources.meshes[&1].0;
    let mut empty = solid("empty", 1);
    empty.indices.clear();
    resources.upload_solid_list(&mut renderer, &[empty], &SectionResources::default(), false);
    assert_eq!(resources.meshes[&1].0, old);
    resources.upload_solid_list(&mut renderer, &[solid("skipped", 1)], &SectionResources::default(), true);
    assert_eq!(resources.meshes[&1].0, old);
    assert_eq!(renderer.mesh_labels[&old], "first");
    assert_eq!(renderer.resources.lock().unwrap().meshes, 1);
    assert_eq!(renderer.resources.lock().unwrap().mesh_destructions, 0);
    resources.release(&mut renderer);
    assert!(renderer.mesh_labels.is_empty());
}

fn renderer() -> NullBackend {
    NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking()
}
fn solid(name: &str, name_hash: u32) -> Solid {
    Solid {
        name: name.into(),
        name_hash,
        version: 0x16,
        flags: 0,
        bounds_min: [0.0; 3],
        bounds_max: [1.0; 3],
        transform: Mat4::IDENTITY.to_cols_array(),
        num_polys: 1,
        density: 1.0,
        texture_hashes: vec![],
        light_material_hashes: vec![],
        markers: vec![],
        vertex_buffers: vec![],
        vertices: vec![blackbox_solid::Vertex {
            position: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
            color_bgra: [255; 4],
            uv: [0.0; 2],
        }],
        secondary_uvs: None,
        tangents: None,
        indices: vec![0, 0, 0],
        groups: vec![],
        skin: None,
    }
}
