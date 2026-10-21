//! Filtering skips rejected meshes but preserves accepted models and parser errors.
use super::builder::{Group, bstring_hash, solid_file, vertex_buffer};
use crate::{read_solids, read_solids_filtered};

#[test]
fn rejected_broken_mesh_is_not_decoded_and_selected_mesh_is_unchanged() {
    let groups = [Group { effect: 0, vertices: 3, indices: vec![0, 1, 2] }];
    let good = solid_file("GOOD", &groups, &[vertex_buffer(3, 36, 0.0)]);
    let bad = solid_file("BAD", &groups, &[vec![0; 100]]);
    // Use a standalone second fixture to exercise rejection of its malformed mesh.
    assert!(read_solids(&bad).is_err());
    assert!(read_solids_filtered(&bad, |hash| hash == bstring_hash("GOOD")).unwrap().is_empty());
    assert!(read_solids_filtered(&bad, |_| true).is_err());
    let selected = read_solids_filtered(&good, |hash| hash == bstring_hash("GOOD")).unwrap();
    assert_eq!(selected, read_solids(&good).unwrap());
    assert!(read_solids_filtered(&good, |_| false).unwrap().is_empty());
}

#[test]
fn filtering_does_not_hide_invalid_headers() {
    let mut file = solid_file("HEADER", &[], &[]);
    let info = blackbox_chunk::find_all(&file, blackbox_chunk::ids::SOLID_INFO)[0];
    let version = (info.offset + 8).next_multiple_of(0x10) + 0x0c;
    file[version] = 0xff;
    assert!(read_solids_filtered(&file, |_| false).is_err());
}
