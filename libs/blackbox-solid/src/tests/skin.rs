//! Skin of effect-2 (skinned) solids: blend weights and slots read from 60-byte vertices and resolved through each
//! group's bone table. Spec: `docs/formats/models.md`, "Skinned vertices and bones".

use super::builder::{Group, skinned_vertex_buffer, solid_file, vertex_buffer};
use crate::{Error, SkinVertex, read_solids};

/// The bone numbers of slots 0 to 3 are 5, 9, 2 and 12, so a resolved bone shows which slot it came from.
const TABLE: [u8; 16] = [5, 9, 2, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
/// A second table for a second group of the same buffer.
const OTHER: [u8; 16] = [30, 31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

#[test]
fn skin_is_read_and_resolved_through_the_group_table() {
    let vertices = skinned_vertex_buffer(&[([0.5, 0.25, 0.25], [0.0, 1.0, 2.0]), ([1.0, 0.0, 0.0], [3.0, 0.0, 0.0])]);
    let groups = [Group { effect: 2, vertices: 2, indices: vec![0, 1, 0], bone_table: Some(TABLE) }];
    let file = solid_file("SKIN", &groups, &[vertices]);
    let s = &read_solids(&file).unwrap()[0];
    assert_eq!(s.groups[0].bone_table, Some(TABLE));
    assert_eq!(s.vertices[1].position[0], 1.0, "the common attributes are still read");
    let skin = s.skin.as_ref().expect("a skinned solid has skin");
    assert_eq!(skin.len(), s.vertices.len());
    assert_eq!(skin[0], SkinVertex { weights: [0.5, 0.25, 0.25], bones: [5, 9, 2] });
    assert_eq!(skin[1], SkinVertex { weights: [1.0, 0.0, 0.0], bones: [12, 5, 5] });
}

#[test]
fn groups_of_one_buffer_resolve_through_their_own_tables() {
    // Group 0 covers vertices 0 and 1 of the buffer, group 1 vertex 2; their indices are relative to the buffer.
    let vertices = skinned_vertex_buffer(&[
        ([1.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        ([1.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        ([1.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
    ]);
    let groups = [
        Group { effect: 2, vertices: 2, indices: vec![0, 1, 0], bone_table: Some(TABLE) },
        Group { effect: 2, vertices: 1, indices: vec![2, 2, 2], bone_table: Some(OTHER) },
    ];
    let file = solid_file("TWO", &groups, &[vertices]);
    let s = &read_solids(&file).unwrap()[0];
    assert_eq!(s.vertex_buffers.len(), 1, "both groups share one effect run");
    assert_eq!(s.groups[1].bone_table, Some(OTHER));
    let bones: Vec<[u8; 3]> = s.skin.as_ref().unwrap().iter().map(|v| v.bones).collect();
    assert_eq!(bones, [[5, 5, 5], [9, 9, 9], [30, 30, 30]]);
}

#[test]
fn a_normal_mapped_vertex_is_not_read_as_skin() {
    // Effect 1 (WorldNormalMap) also has 60-byte vertices, but their bytes 36 to 60 are not skin. The slots here are
    // not whole numbers in 0..16, so reading them as skin would fail the solid.
    let vertices = skinned_vertex_buffer(&[([0.5, 0.25, 0.25], [99.0, 99.0, 99.0]); 3]);
    let groups = [Group { effect: 1, vertices: 3, indices: vec![0, 1, 2], bone_table: None }];
    let file = solid_file("NMAP", &groups, &[vertices]);
    let s = &read_solids(&file).unwrap()[0];
    assert_eq!(s.vertex_buffers[0].stride, 60);
    assert!(s.groups[0].bone_table.is_none());
    assert!(s.skin.is_none());
}

#[test]
fn an_unskinned_solid_has_no_skin() {
    let groups = [Group { effect: 0, vertices: 3, indices: vec![0, 1, 2], bone_table: None }];
    let file = solid_file("PLAIN", &groups, &[vertex_buffer(3, 36, 0.0)]);
    let s = &read_solids(&file).unwrap()[0];
    assert!(s.skin.is_none());
    assert!(s.groups[0].bone_table.is_none());
}

#[test]
fn a_blend_slot_outside_the_table_is_an_error() {
    let vertices = skinned_vertex_buffer(&[([1.0, 0.0, 0.0], [16.0, 0.0, 0.0])]);
    let groups = [Group { effect: 2, vertices: 1, indices: vec![0, 0, 0], bone_table: Some(TABLE) }];
    let file = solid_file("SLOT", &groups, &[vertices]);
    assert!(matches!(read_solids(&file), Err(Error::Solid { .. })));
}
