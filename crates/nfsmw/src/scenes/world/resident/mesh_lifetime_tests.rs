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

#[test]
fn environment_lights_retain_markers_only_for_live_meshes_and_valid_roles() {
    let mut renderer = renderer();
    let mut resources = SectionResources::default();
    let mut model = solid("marked", 1);
    model.transform = Mat4::from_translation(glam::Vec3::new(10.0, 20.0, 30.0)).to_cols_array();
    let marker = |role, position| blackbox_solid::PositionMarker {
        name_hash: 0,
        int_param: role,
        float_params: [0.0; 2],
        matrix: Mat4::from_translation(position).to_cols_array(),
    };
    model.markers = vec![
        marker(3, glam::Vec3::new(11.0, 22.0, 33.0)),
        marker(5, glam::Vec3::new(14.0, 25.0, 36.0)),
        marker(-1, glam::Vec3::ZERO),
        marker(6, glam::Vec3::ZERO),
        marker(0, glam::Vec3::splat(f32::NAN)),
    ];
    resources.upload_solid_list(&mut renderer, &[model], &Default::default(), false);
    let first = resources.meshes[&1].0;
    let markers = &resources.flare_markers[&first];
    assert_eq!(markers.len(), 2);
    assert_eq!(markers[0].position, glam::Vec3::new(1.0, 2.0, 3.0), "subtract pivot exactly once");
    assert_eq!(markers[1].kind, nfsmw_data::world::lights::Kind::GreenBlink);
    let mut empty = solid("failed replacement", 1);
    empty.indices.clear();
    resources.upload_solid_list(&mut renderer, &[empty], &Default::default(), false);
    assert!(resources.flare_markers.contains_key(&first), "failed uploads retain both mesh and light data");
    resources.upload_solid_list(&mut renderer, &[solid("replacement", 1)], &Default::default(), false);
    assert!(resources.flare_markers.is_empty(), "unmarked replacement cannot retain the old lamp");
    resources.release(&mut renderer);
    assert!(renderer.mesh_labels.is_empty());
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
