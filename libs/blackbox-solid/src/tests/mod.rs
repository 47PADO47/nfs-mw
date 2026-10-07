mod builder;

use builder::{Group, bstring_hash, solid_file, solid_file_with_markers, vertex_buffer};

use crate::{Error, layout, read_solids};

#[test]
fn single_buffer_solid() {
    let file =
        solid_file("TRI", &[Group { effect: 0, vertices: 3, indices: vec![0, 1, 2] }], &[vertex_buffer(3, 36, 0.0)]);
    let solids = read_solids(&file).unwrap();
    assert_eq!(solids.len(), 1);
    let s = &solids[0];
    assert_eq!((s.name.as_str(), s.version, s.name_hash), ("TRI", 0x16, bstring_hash("TRI")));
    assert_eq!(s.texture_hashes, [0xAA]);
    assert_eq!(s.vertex_buffers.len(), 1);
    assert_eq!(s.vertex_buffers[0].stride, 36);
    assert_eq!(s.vertices[2].position, [2.0, 0.0, 0.0]);
    assert_eq!(s.vertices[0].color_bgra, [1, 2, 3, 4]);
    assert_eq!(s.indices, [0, 1, 2]);
    assert_eq!(s.groups[0].diffuse_texture(s), Some(0xAA));
    assert_eq!(s.groups[0].light_material, 0xFF);
}

#[test]
fn effect_runs_share_vertex_buffers() {
    // Groups 0 and 1 share effect 0 (one 36-byte buffer); group 2 (effect 3) has its own 60-byte buffer.
    let groups = [
        Group { effect: 0, vertices: 3, indices: vec![0, 1, 2] },
        Group { effect: 0, vertices: 3, indices: vec![3, 4, 5] },
        Group { effect: 3, vertices: 4, indices: vec![0, 1, 2, 1, 2, 3] },
    ];
    let file = solid_file("RUNS", &groups, &[vertex_buffer(6, 36, 0.0), vertex_buffer(4, 60, 100.0)]);
    let s = &read_solids(&file).unwrap()[0];
    assert_eq!(
        s.vertex_buffers.iter().map(|b| (b.stride, b.first_vertex, b.vertex_count)).collect::<Vec<_>>(),
        [(36, 0, 6), (60, 6, 4)]
    );
    assert_eq!(s.groups.iter().map(|g| (g.vertex_buffer, g.base_vertex)).collect::<Vec<_>>(), [(0, 0), (0, 0), (1, 6)]);
    assert_eq!(s.vertices.len(), 10);
    // Group 2's index 0 is the first vertex of the second buffer.
    let g = &s.groups[2];
    let first = s.indices[g.first_index as usize] as u32 + g.base_vertex;
    assert_eq!(s.vertices[first as usize].position[0], 100.0);
}

#[test]
fn buffer_count_must_match_effect_runs() {
    let groups = [
        Group { effect: 0, vertices: 3, indices: vec![0, 1, 2] },
        Group { effect: 1, vertices: 3, indices: vec![0, 1, 2] },
    ];
    let file = solid_file("BAD", &groups, &[vertex_buffer(6, 36, 0.0)]);
    assert!(matches!(read_solids(&file), Err(Error::Solid { .. })));
}

#[test]
fn unknown_version_is_reported() {
    let mut file =
        solid_file("V", &[Group { effect: 0, vertices: 3, indices: vec![0, 1, 2] }], &[vertex_buffer(3, 36, 0.0)]);
    // Patch the version byte of the (0x10-aligned) SolidInfo payload.
    let info = file.windows(8).position(|w| w[..4] == 0x0013_4011u32.to_le_bytes()).unwrap() + 8;
    let info = info.next_multiple_of(0x10);
    file[info + layout::VERSION_OFFSET] = 0x19;
    assert!(matches!(read_solids(&file), Err(Error::UnsupportedVersion { version: 0x19, .. })));
}

#[test]
fn position_markers() {
    let brake = bstring_hash("FRONT_BRAKE");
    let file = solid_file_with_markers(
        "WHEEL",
        &[Group { effect: 0, vertices: 3, indices: vec![0, 1, 2] }],
        &[vertex_buffer(3, 36, 0.0)],
        &[(brake, [0.0, 0.046, 0.0]), (7, [1.0, 2.0, 3.0])],
    );
    let s = &read_solids(&file).unwrap()[0];
    assert_eq!(s.markers.len(), 2);
    assert_eq!(s.marker(brake).unwrap().translation(), [0.0, 0.046, 0.0]);
    assert_eq!(s.markers[1].translation(), [1.0, 2.0, 3.0]);
    assert!(s.marker(1).is_none());
}
