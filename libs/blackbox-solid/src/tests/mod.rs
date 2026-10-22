mod builder;
mod filtered;
mod rigid;
mod skin;
mod winding;

use builder::{Group, bstring_hash, solid_file, solid_file_with_markers, vertex_buffer};

use crate::{Error, layout, read_solids, read_solids_lossy};

#[test]
fn single_buffer_solid() {
    let file = solid_file(
        "TRI",
        &[Group { effect: 0, vertices: 3, indices: vec![0, 1, 2], bone_table: None }],
        &[vertex_buffer(3, 36, 0.0)],
    );
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
fn solid_packs_after_the_geometry_pack_are_read_too() {
    // The flat layout of some add-on cars: an (empty here) GeometryPack, then each SolidPack as a sibling.
    let wrapped = solid_file(
        "FLAT",
        &[Group { effect: 0, vertices: 3, indices: vec![0, 1, 2], bone_table: None }],
        &[vertex_buffer(3, 36, 0.0)],
    );
    let mut file = wrapped[..8].to_vec();
    file[4..8].copy_from_slice(&0u32.to_le_bytes());
    file.extend_from_slice(&wrapped[8..]);
    let solids = read_solids(&file).unwrap();
    assert_eq!(solids.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["FLAT"]);
    assert_eq!(solids[0].indices, [0, 1, 2]);
}

#[test]
fn a_broken_solid_fails_read_solids_but_not_the_lossy_read() {
    let good = solid_file(
        "GOOD",
        &[Group { effect: 0, vertices: 3, indices: vec![0, 1, 2], bone_table: None }],
        &[vertex_buffer(3, 36, 0.0)],
    );
    // 100 bytes of vertices cannot be 3 vertices.
    let bad = solid_file(
        "BAD",
        &[Group { effect: 0, vertices: 3, indices: vec![0, 1, 2], bone_table: None }],
        &[vec![0; 100]],
    );
    let mut file = good[..8].to_vec();
    file[4..8].copy_from_slice(&0u32.to_le_bytes());
    file.extend_from_slice(&good[8..]);
    file.extend_from_slice(&bad[8..]);
    assert!(read_solids(&file).is_err(), "the strict read stops at the broken solid");
    let (solids, failed) = read_solids_lossy(&file).unwrap();
    assert_eq!(solids.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["GOOD"]);
    assert_eq!(failed.len(), 1);
}

#[test]
fn effect_runs_share_vertex_buffers() {
    // Groups 0 and 1 share effect 0 (one 36-byte buffer); group 2 (effect 3) has its own 60-byte buffer.
    let groups = [
        Group { effect: 0, vertices: 3, indices: vec![0, 1, 2], bone_table: None },
        Group { effect: 0, vertices: 3, indices: vec![3, 4, 5], bone_table: None },
        Group { effect: 3, vertices: 4, indices: vec![0, 1, 2, 1, 2, 3], bone_table: None },
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
fn independent_sky_and_normal_uvs_survive_mixed_buffers() {
    let groups = [
        Group { effect: 0, vertices: 3, indices: vec![0, 1, 2], bone_table: None },
        Group { effect: 19, vertices: 3, indices: vec![0, 1, 2], bone_table: None },
        Group { effect: 3, vertices: 3, indices: vec![0, 1, 2], bone_table: None },
    ];
    let mut sky = vertex_buffer(3, 44, 0.0);
    for (i, v) in sky.as_chunks_mut::<44>().0.iter_mut().enumerate() {
        v[28..32].copy_from_slice(&0.25_f32.to_le_bytes());
        v[36..40].copy_from_slice(&(i as f32 + 0.5).to_le_bytes());
        v[40..44].copy_from_slice(&0.75_f32.to_le_bytes());
    }
    let mut normal = vertex_buffer(3, 60, 0.0);
    for v in normal.as_chunks_mut::<60>().0 {
        v[28..32].copy_from_slice(&0.125_f32.to_le_bytes());
        v[36..40].copy_from_slice(&99.0_f32.to_le_bytes());
        for (i, value) in [0.25_f32, -0.5, 1.0, -1.0].into_iter().enumerate() {
            v[44 + i * 4..48 + i * 4].copy_from_slice(&value.to_le_bytes());
        }
    }
    let file = solid_file("UVS", &groups, &[vertex_buffer(3, 36, 0.0), sky, normal]);
    let solid = &read_solids(&file).unwrap()[0];
    let uvs = solid.secondary_uvs.as_ref().expect("a buffer has second UVs");
    assert_eq!(uvs.len(), solid.vertices.len());
    assert_eq!(uvs[0], solid.vertices[0].uv);
    assert_eq!(uvs[3..6], [[0.5, 0.75], [1.5, 0.75], [2.5, 0.75]]);
    assert_eq!(uvs[6], [99.0, 0.0]);
    let tangents = solid.tangents.as_ref().expect("a buffer has tangents");
    assert_eq!(tangents.len(), solid.vertices.len());
    assert_eq!(&tangents[..6], &[[0.0; 4]; 6]);
    assert_eq!(&tangents[6..], &[[0.25, -0.5, 1.0, -1.0]; 3]);
    // The accessors give the same values, and the fallbacks outside the stored channels.
    assert_eq!(solid.secondary_uv(4), [1.5, 0.75]);
    assert_eq!(solid.secondary_uv(99), [0.0; 2]);
    assert_eq!(solid.tangent(7), [0.25, -0.5, 1.0, -1.0]);
    assert_eq!(solid.tangent(99), [0.0; 4]);
}

#[test]
fn extra_channels_are_absent_when_no_buffer_has_them() {
    let groups = [
        Group { effect: 0, vertices: 3, indices: vec![0, 1, 2], bone_table: None },
        Group { effect: 5, vertices: 3, indices: vec![0, 1, 2], bone_table: None },
    ];
    let file = solid_file("PLAIN", &groups, &[vertex_buffer(3, 36, 0.0), vertex_buffer(3, 36, 0.0)]);
    let solid = &read_solids(&file).unwrap()[0];
    assert_eq!(solid.vertices.len(), 6);
    assert_eq!(solid.secondary_uvs, None);
    assert_eq!(solid.tangents, None);
    assert!(solid.skin.is_none());
    assert_eq!(solid.secondary_uv(2), solid.vertices[2].uv);
    assert_eq!(solid.tangent(2), [0.0; 4]);
}

#[test]
fn a_channel_that_starts_late_is_backfilled_and_one_that_ends_early_is_padded() {
    // Effect 19 is the sky layer (44 bytes, second UVs); the plain buffers around it have none.
    let groups = [
        Group { effect: 19, vertices: 3, indices: vec![0, 1, 2], bone_table: None },
        Group { effect: 0, vertices: 3, indices: vec![0, 1, 2], bone_table: None },
    ];
    let mut sky = vertex_buffer(3, 44, 0.0);
    for v in sky.as_chunks_mut::<44>().0 {
        v[36..40].copy_from_slice(&7.0_f32.to_le_bytes());
    }
    let file = solid_file("SKYFIRST", &groups, &[sky, vertex_buffer(3, 36, 0.0)]);
    let solid = &read_solids(&file).unwrap()[0];
    let uvs = solid.secondary_uvs.as_ref().expect("the sky buffer has second UVs");
    assert_eq!(uvs.len(), 6);
    assert_eq!(uvs[0][0], 7.0);
    assert_eq!(uvs[3], solid.vertices[3].uv);
    assert_eq!(solid.tangents, None);
}

#[test]
fn buffer_count_must_match_effect_runs() {
    let groups = [
        Group { effect: 0, vertices: 3, indices: vec![0, 1, 2], bone_table: None },
        Group { effect: 1, vertices: 3, indices: vec![0, 1, 2], bone_table: None },
    ];
    let file = solid_file("BAD", &groups, &[vertex_buffer(6, 36, 0.0)]);
    assert!(matches!(read_solids(&file), Err(Error::Solid { .. })));
}

#[test]
fn unknown_version_is_reported() {
    let mut file = solid_file(
        "V",
        &[Group { effect: 0, vertices: 3, indices: vec![0, 1, 2], bone_table: None }],
        &[vertex_buffer(3, 36, 0.0)],
    );
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
        &[Group { effect: 0, vertices: 3, indices: vec![0, 1, 2], bone_table: None }],
        &[vertex_buffer(3, 36, 0.0)],
        &[(brake, [0.0, 0.046, 0.0]), (7, [1.0, 2.0, 3.0])],
    );
    let s = &read_solids(&file).unwrap()[0];
    assert_eq!(s.markers.len(), 2);
    assert_eq!(s.marker(brake).unwrap().translation(), [0.0, 0.046, 0.0]);
    assert_eq!(s.markers[1].translation(), [1.0, 2.0, 3.0]);
    assert!(s.marker(1).is_none());
}

#[test]
fn unknown_wide_declarations_do_not_become_tangent_data() {
    let mut vertices = vertex_buffer(3, 60, 0.0);
    for v in vertices.as_chunks_mut::<60>().0 {
        v[36..40].copy_from_slice(&99.0_f32.to_le_bytes());
        v[44..48].copy_from_slice(&88.0_f32.to_le_bytes());
    }
    let file =
        solid_file("BONE", &[Group { effect: 2, vertices: 3, indices: vec![0, 1, 2], bone_table: None }], &[vertices]);
    let solid = &read_solids(&file).unwrap()[0];
    assert_eq!(solid.secondary_uvs, None);
    assert_eq!(solid.tangents, None);
    assert_eq!(solid.secondary_uv(1), solid.vertices[1].uv);
    assert_eq!(solid.tangent(1), [0.0; 4]);
}
