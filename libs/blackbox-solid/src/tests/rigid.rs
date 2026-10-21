//! `rigid_split`: each triangle goes to its dominant bone, and each part keeps the vertex data it uses.

use super::builder::{Group, skinned_vertex_buffer, solid_file, vertex_buffer};
use crate::{RigidPart, read_solids, rigid_split};

/// Slots 0 to 2 name bones 5, 9 and 2.
const TABLE: [u8; 16] = [5, 9, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

fn positions(part: &RigidPart) -> Vec<f32> {
    part.vertices.iter().map(|v| v.position[0]).collect()
}

#[test]
fn triangles_go_to_their_dominant_bone() {
    // Six vertices and three triangles. (0, 1, 2) is bone 5: its weight is 1.9 against 0.7 for bone 2 and 0.4 for 9.
    // (2, 3, 4) is bone 9 with 2.3 against 0.7. (5, 0, 1) is bone 5, and vertex 2 is in both bone 5 and bone 9 parts.
    let vertices = skinned_vertex_buffer(&[
        ([1.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        ([0.9, 0.1, 0.0], [0.0, 1.0, 0.0]),
        ([0.3, 0.7, 0.0], [1.0, 2.0, 0.0]),
        ([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        ([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        ([1.0, 0.0, 0.0], [2.0, 0.0, 0.0]),
    ]);
    let groups = [Group { effect: 2, vertices: 6, indices: vec![0, 1, 2, 2, 3, 4, 5, 0, 1], bone_table: Some(TABLE) }];
    let file = solid_file("SPLIT", &groups, &[vertices]);
    let s = &read_solids(&file).unwrap()[0];

    let parts = rigid_split(s);
    assert_eq!(parts.iter().map(|p| p.bone).collect::<Vec<_>>(), [5, 9]);
    let [five, nine] = parts.as_slice() else {
        panic!("two parts expected");
    };
    assert_eq!(positions(five), [0.0, 1.0, 2.0, 5.0]);
    assert_eq!(five.indices, [0, 1, 2, 3, 0, 1]);
    assert_eq!(positions(nine), [2.0, 3.0, 4.0]);
    assert_eq!(nine.indices, [0, 1, 2]);
    // The copied vertices are the solid's own, attributes and all.
    assert_eq!(five.vertices[3], s.vertices[5]);
    assert_eq!(nine.vertices[0], s.vertices[2]);
    assert_eq!(five.vertices[0].color_bgra, [1, 2, 3, 4]);
}

#[test]
fn an_unskinned_solid_has_no_parts() {
    let groups = [Group { effect: 0, vertices: 3, indices: vec![0, 1, 2], bone_table: None }];
    let file = solid_file("PLAIN", &groups, &[vertex_buffer(3, 36, 0.0)]);
    let s = &read_solids(&file).unwrap()[0];
    assert!(rigid_split(s).is_empty());
}
