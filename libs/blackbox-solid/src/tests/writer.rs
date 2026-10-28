//! `write_geometry_pack`: what is written reads back through the reader as the same data, in the layout of the stock
//! files (`docs/formats/models.md`).

use blackbox_chunk::{Chunk, ids};

use super::builder::bstring_hash;
use crate::bytes::u32_at;
use crate::{
    BOUNDS_MARGIN, GroupProblem, GroupSource, PositionMarker, Solid, SolidSource, Vertex, WriteError, read_solids,
    write_geometry_pack,
};

/// `count` vertices, distinguishable by position (`seed + i`) and uv.
fn vertices(count: usize, seed: f32) -> Vec<Vertex> {
    (0..count)
        .map(|i| {
            let x = seed + i as f32;
            Vertex {
                position: [x, -x, 0.5 * x],
                normal: [0.0, 0.0, 1.0],
                color_bgra: [i as u8, 2, 3, 255],
                uv: [x, 0.25],
            }
        })
        .collect()
}

fn group(vertex_count: usize, seed: f32, indices: &[u16]) -> GroupSource {
    GroupSource::new(vertices(vertex_count, seed), indices.to_vec())
}

fn marker(name: &str, at: [f32; 3]) -> PositionMarker {
    let mut matrix = crate::IDENTITY;
    matrix[12..15].copy_from_slice(&at);
    PositionMarker { name_hash: bstring_hash(name), int_param: -3, float_params: [0.5, 2.0], matrix }
}

/// Two solids: a body with three groups (two effect-4 groups sharing a buffer, then an effect-0 group with its own),
/// and a wheel with one group and markers.
fn sources() -> Vec<SolidSource> {
    let mut a = group(4, 0.0, &[0, 1, 2, 2, 1, 3]);
    a.texture_slots = [0, 1, 2, 1, 0];
    a.light_material = 1;
    a.material_name = Some("BODY_CHROME".into());
    let mut b = group(5, 100.0, &[0, 1, 2, 2, 3, 4, 4, 0, 2]);
    b.texture_slots = [2, 2, 2, 2, 2];
    b.flags = 0x14180;
    b.material_name = Some("WINDOW_FRONT".into());
    let mut c = group(3, 200.0, &[2, 1, 0]);
    c.effect_id = 0;
    c.flags = 0;
    let mut body = SolidSource::new("TESTCAR_BASE_A", vec![a, b, c]);
    body.texture_hashes = vec![bstring_hash("TESTCAR_BODY"), 0x1234_5678, bstring_hash("GLASS")];
    body.light_material_hashes = vec![0xAA, 0xBB];
    body.transform[13] = 0.75;
    body.flags = 0x0080;

    let mut wheel = SolidSource::new("TESTCAR_KIT00_FRONT_WHEEL_A", vec![group(6, -3.0, &[0, 1, 2, 3, 4, 5])]);
    wheel.texture_hashes = vec![bstring_hash("TESTCAR_WHEEL")];
    wheel.markers = vec![marker("FRONT_BRAKE", [0.0, 0.046, 0.0]), marker("LEFT_EXHAUST", [-2.1, 0.286, 0.157])];
    vec![body, wheel]
}

fn top_chunks(data: &[u8]) -> Vec<Chunk<'_>> {
    blackbox_chunk::chunks(data).collect::<Result<_, _>>().unwrap()
}

fn find<'a>(solids: &'a [Solid], name: &str) -> &'a Solid {
    solids.iter().find(|s| s.name == name).unwrap()
}

/// The vertices a group draws, resolved the way a renderer would.
fn resolved(solid: &Solid, g: usize) -> Vec<Vertex> {
    let group = &solid.groups[g];
    let first = group.first_index as usize;
    let drawn = &solid.indices[first..first + group.num_indices as usize];
    drawn.iter().map(|&i| solid.vertices[group.base_vertex as usize + usize::from(i)]).collect()
}

#[test]
fn two_solids_read_back_as_written() {
    let written = sources();
    let file = write_geometry_pack(&written).unwrap();
    let solids = read_solids(&file).unwrap();
    assert_eq!(
        solids.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["TESTCAR_BASE_A", "TESTCAR_KIT00_FRONT_WHEEL_A"]
    );

    for source in &written {
        let solid = find(&solids, &source.name);
        assert_eq!((solid.version, solid.name_hash), (0x16, bstring_hash(&source.name)));
        assert_eq!(solid.flags, source.flags);
        assert_eq!((solid.bounds_min, solid.bounds_max), (source.bounds_min, source.bounds_max));
        assert_eq!(solid.transform, source.transform);
        assert_eq!(solid.texture_hashes, source.texture_hashes);
        assert_eq!(solid.light_material_hashes, source.light_material_hashes);
        assert_eq!(solid.markers, source.markers);
        assert_eq!(solid.groups.len(), source.groups.len());
        assert_eq!(usize::from(solid.num_polys), source.groups.iter().map(|g| g.indices.len() / 3).sum::<usize>());
        assert!(solid.skin.is_none() && solid.tangents.is_none() && solid.secondary_uvs.is_none());
        for (i, g) in source.groups.iter().enumerate() {
            let read = &solid.groups[i];
            assert_eq!(read.texture_slots, g.texture_slots);
            assert_eq!(read.light_material, g.light_material);
            assert_eq!((read.effect_id, read.flags), (g.effect_id, g.flags));
            assert_eq!(read.num_vertices as usize, g.vertices.len());
            assert_eq!(
                (read.num_triangles as usize, read.num_indices as usize),
                (g.indices.len() / 3, g.indices.len())
            );
            assert_eq!(read.bone_table, None);
            // The vertices a group draws are the ones its own indices point at.
            let expected: Vec<Vertex> = g.indices.iter().map(|&k| g.vertices[usize::from(k)]).collect();
            assert_eq!(resolved(solid, i), expected, "{} group {i}", source.name);
        }
    }
}

#[test]
fn groups_with_one_effect_share_a_buffer_and_indices_count_from_its_start() {
    let file = write_geometry_pack(&sources()).unwrap();
    let solids = read_solids(&file).unwrap();
    let body = find(&solids, "TESTCAR_BASE_A");
    assert_eq!(
        body.vertex_buffers.iter().map(|b| (b.stride, b.first_vertex, b.vertex_count)).collect::<Vec<_>>(),
        [(36, 0, 9), (36, 9, 3)]
    );
    assert_eq!(
        body.groups.iter().map(|g| (g.vertex_buffer, g.base_vertex)).collect::<Vec<_>>(),
        [(0, 0), (0, 0), (1, 9)]
    );
    assert_eq!(body.groups.iter().map(|g| g.first_index).collect::<Vec<_>>(), [0, 6, 15]);
    // Group 1's own indices 0..5 are stored after group 0's four vertices.
    assert_eq!(body.indices, [0, 1, 2, 2, 1, 3, 4, 5, 6, 6, 7, 8, 8, 4, 6, 2, 1, 0]);
    assert_eq!(body.vertices.len(), 12);
}

#[test]
fn group_and_solid_bounds_are_the_vertices_plus_the_stock_margin() {
    let file = write_geometry_pack(&sources()).unwrap();
    let body = &read_solids(&file).unwrap()[0];
    let m = BOUNDS_MARGIN;
    // Group 0: x = 0..3, y = -x, z = x / 2.
    assert_eq!(body.groups[0].bounds_min, [0.0 - m, -3.0 - m, 0.0 - m]);
    assert_eq!(body.groups[0].bounds_max, [3.0 + m, 0.0 + m, 1.5 + m]);
    assert_eq!(body.bounds_min, [0.0 - m, -202.0 - m, 0.0 - m]);
    assert_eq!(body.bounds_max, [202.0 + m, 0.0 + m, 101.0 + m]);
}

#[test]
fn density_is_polygons_per_volume_unless_given() {
    let mut cube = SolidSource::new("CUBE", vec![group(3, 0.0, &[0, 1, 2])]);
    cube.bounds_min = [0.0; 3];
    cube.bounds_max = [2.0, 1.0, 0.5];
    let mut given = cube.clone();
    given.name = "GIVEN".into();
    given.density = Some(42.0);
    let file = write_geometry_pack(&[cube, given]).unwrap();
    let solids = read_solids(&file).unwrap();
    assert_eq!(find(&solids, "CUBE").density, 1.0);
    assert_eq!(find(&solids, "GIVEN").density, 42.0);
    assert_eq!(find(&solids, "CUBE").num_polys, 1);
}

#[test]
fn the_polygon_count_can_be_overridden() {
    let mut solid = SolidSource::new("POLYS", vec![group(3, 0.0, &[0, 1, 2])]);
    solid.polygon_count = Some(7);
    let read = &read_solids(&write_geometry_pack(&[solid]).unwrap()).unwrap()[0];
    assert_eq!((read.num_polys, read.groups[0].num_triangles), (7, 1));
}

#[test]
fn mesh_container_tables_match_the_solids_and_the_file() {
    let file = write_geometry_pack(&sources()).unwrap();
    let top = top_chunks(&file);
    assert_eq!(top.len(), 1);
    let pack = top[0];
    assert_eq!((pack.id, pack.offset, pack.end()), (ids::GEOMETRY_PACK, 0, file.len()));
    let kids: Vec<_> = pack.children().collect::<Result<_, _>>().unwrap();
    // An empty padding chunk puts the mesh container on 0x10, and the solid packs follow it.
    assert_eq!((kids[0].id, kids[0].payload.len()), (0, 0));
    let info = kids[1];
    assert_eq!((info.id, info.offset), (ids::MESH_CONTAINER_INFO, 0x10));
    let tables: Vec<_> = info.children().collect::<Result<_, _>>().unwrap();
    assert_eq!(
        tables.iter().map(|c| c.id).collect::<Vec<_>>(),
        [ids::MESH_CONTAINER_HEADER, ids::MESH_CONTAINER_KEYS, ids::MESH_CONTAINER_OFFSETS, ids::MESH_CONTAINER_EMPTY]
    );
    let header = tables[0].aligned_payload(0x10);
    assert_eq!(header.len(), 144);
    assert_eq!(u32_at(header, 0x0C), 2, "solid count");
    assert_eq!(&header[0x10..0x1C], b"GEOMETRY.BIN");
    assert_eq!(u32_at(header, 0x68) as usize, file.len(), "file size");
    assert!(tables[3].children().next().is_none());

    let packs: Vec<_> = kids.iter().filter(|c| c.id == ids::SOLID_PACK).copied().collect();
    assert_eq!(packs.len(), 2);
    assert!(packs.iter().all(|p| p.offset % 0x80 == 0), "solid packs sit on 0x80 boundaries");

    // Keys: (hash, 0) sorted by hash; offsets: (hash, offset, size, size, 0, 0) in the same order.
    let mut hashes = [bstring_hash("TESTCAR_BASE_A"), bstring_hash("TESTCAR_KIT00_FRONT_WHEEL_A")];
    hashes.sort_unstable();
    let keys = tables[1].payload;
    assert_eq!(keys.len(), 16);
    assert_eq!([u32_at(keys, 0), u32_at(keys, 8)], hashes);
    assert_eq!([u32_at(keys, 4), u32_at(keys, 12)], [0, 0]);
    let offsets = tables[2].payload;
    assert_eq!(offsets.len(), 48);
    for (n, hash) in hashes.into_iter().enumerate() {
        let entry = &offsets[n * 24..(n + 1) * 24];
        assert_eq!(u32_at(entry, 0), hash);
        let target =
            packs.iter().find(|p| p.offset == u32_at(entry, 4) as usize).expect("offset points at a SolidPack");
        assert_eq!(u32_at(entry, 8) as usize, target.end() - target.offset, "size includes the chunk header");
        assert_eq!(u32_at(entry, 12), u32_at(entry, 8));
        assert_eq!((u32_at(entry, 16), u32_at(entry, 20)), (0, 0));
        // The pack it points at is the solid it is listed under.
        let info = target.child(ids::SOLID_INFO).unwrap().aligned_payload(0x10);
        assert_eq!(u32_at(info, 0x10), hash);
    }
}

#[test]
fn payload_alignment_follows_the_stock_files() {
    let file = write_geometry_pack(&sources()).unwrap();
    for pack in top_chunks(&file)[0].children().map(Result::unwrap).filter(|c| c.id == ids::SOLID_PACK) {
        let at = |id: u32, parent: Chunk<'_>| parent.child(id).unwrap().payload_offset();
        assert_eq!(at(ids::SOLID_INFO, pack) % 0x10, 0);
        let mesh = pack.child(ids::MESH_INFO_CONTAINER).unwrap();
        let aligned = [
            (ids::MESH_INFO_HEADER, 0x10),
            (ids::MESH_SHADING_GROUPS, 0x10),
            (ids::MESH_INDICES, 0x10),
            (ids::MESH_VERTEX_BUFFER, 0x80),
        ];
        for (id, align) in aligned {
            let chunk = mesh.child(id).unwrap();
            let start = chunk.payload_offset() + (chunk.payload.len() - chunk.aligned_payload(align).len());
            assert_eq!(start % align, 0, "chunk {id:#X}");
            // The fill is 0x11 bytes only.
            let fill = &chunk.payload[..chunk.payload.len() - chunk.aligned_payload(align).len()];
            assert!(fill.iter().all(|&b| b == 0x11));
        }
        if let Some(markers) = pack.child(ids::SOLID_MARKERS) {
            assert_eq!(markers.aligned_payload(0x10).len() % 0x50, 0);
        }
    }
}

#[test]
fn solid_info_matches_the_stock_record() {
    let file = write_geometry_pack(&sources()).unwrap();
    let pack = top_chunks(&file)[0].find(ids::SOLID_PACK).unwrap();
    let info = pack.child(ids::SOLID_INFO).unwrap().aligned_payload(0x10);
    assert_eq!(info[0x0C], 0x16);
    assert_eq!(u32_at(info, 0x10), bstring_hash("TESTCAR_BASE_A"));
    // Counts: NumVerts 0, no bones, three textures, two light materials, markers counted from their chunk.
    assert_eq!(&info[0x16..0x1C], &[0, 0, 0, 3, 2, 0]);
    // "TESTCAR_BASE_A" is 14 characters: 15 bytes with the NUL, padded to 16.
    assert_eq!(info.len(), 0xA0 + 16);
    assert_eq!(&info[0xA0..0xAF], b"TESTCAR_BASE_A\0");
}

#[test]
fn material_names_and_the_lists_are_written_as_chunks() {
    let file = write_geometry_pack(&sources()).unwrap();
    let packs =
        top_chunks(&file)[0].children().map(Result::unwrap).filter(|c| c.id == ids::SOLID_PACK).collect::<Vec<_>>();
    let mesh = packs[0].child(ids::MESH_INFO_CONTAINER).unwrap();
    let names: Vec<_> =
        mesh.children().map(Result::unwrap).filter(|c| c.id == ids::MESH_VLT_MATERIALS).map(|c| c.payload).collect();
    // One per group in order, NUL padded to a multiple of 4; the group without a name gets an empty one.
    assert_eq!(names, [&b"BODY_CHROME\0"[..], b"WINDOW_FRONT\0\0\0\0", b"\0\0\0\0"]);
    // The wheel names none of its groups, so it has no such chunks.
    let wheel = packs[1].child(ids::MESH_INFO_CONTAINER).unwrap();
    assert!(wheel.children().map(Result::unwrap).all(|c| c.id != ids::MESH_VLT_MATERIALS));
    // Mesh header: group, buffer and index counts, and the OR of the group flags.
    let header = mesh.child(ids::MESH_INFO_HEADER).unwrap().aligned_payload(0x10);
    assert_eq!(header.len(), 48);
    assert_eq!(
        [u32_at(header, 0x08), u32_at(header, 0x0C), u32_at(header, 0x10), u32_at(header, 0x18), u32_at(header, 0x2C)],
        [18, 0x14180 | 0x4180, 3, 2, 18]
    );
}

#[test]
fn empty_texture_and_light_lists_are_left_out() {
    let solid = SolidSource::new("PLAIN", vec![group(3, 0.0, &[0, 1, 2])]);
    let file = write_geometry_pack(&[solid]).unwrap();
    let pack = top_chunks(&file)[0].find(ids::SOLID_PACK).unwrap();
    assert!(pack.child(ids::SOLID_TEXTURES).is_none() && pack.child(ids::SOLID_LIGHT_MATERIALS).is_none());
    assert!(pack.child(ids::SOLID_MARKERS).is_none());
    let read = &read_solids(&file).unwrap()[0];
    assert!(read.texture_hashes.is_empty() && read.markers.is_empty());
}

#[test]
fn many_solids_stay_sorted_and_aligned() {
    let solids: Vec<SolidSource> = (0..40)
        .map(|i| SolidSource::new(format!("PART_{i:02}"), vec![group(3 + i % 5, i as f32, &[0, 1, 2])]))
        .collect();
    let file = write_geometry_pack(&solids).unwrap();
    let read = read_solids(&file).unwrap();
    assert_eq!(
        read.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
        solids.iter().map(|s| s.name.clone()).collect::<Vec<_>>()
    );
    let info = top_chunks(&file)[0].child(ids::MESH_CONTAINER_INFO).unwrap();
    let keys = info.child(ids::MESH_CONTAINER_KEYS).unwrap().payload;
    let hashes: Vec<u32> = keys.as_chunks::<8>().0.iter().map(|k| u32_at(k, 0)).collect();
    assert_eq!(hashes.len(), 40);
    assert!(hashes.windows(2).all(|w| w[0] < w[1]));
    for pack in top_chunks(&file)[0].children().map(Result::unwrap).filter(|c| c.id == ids::SOLID_PACK) {
        assert_eq!(pack.offset % 0x80, 0);
    }
}

#[test]
fn a_parsed_solid_rewrites_to_the_same_solid() {
    let first = read_solids(&write_geometry_pack(&sources()).unwrap()).unwrap();
    let again: Vec<SolidSource> = first.iter().map(|s| SolidSource::from_solid(s).unwrap()).collect();
    let second = read_solids(&write_geometry_pack(&again).unwrap()).unwrap();
    assert_eq!(first, second);
}

fn error_of(solid: SolidSource) -> WriteError {
    write_geometry_pack(&[solid]).unwrap_err()
}

fn group_problem(solid: SolidSource) -> GroupProblem {
    match error_of(solid) {
        WriteError::Group { problem, .. } => problem,
        other => panic!("expected a group problem, got {other}"),
    }
}

#[test]
fn bad_input_is_refused() {
    assert!(matches!(write_geometry_pack(&[]), Err(WriteError::NoSolids)));
    assert!(matches!(error_of(SolidSource::new("", vec![group(3, 0.0, &[0, 1, 2])])), WriteError::BadName { .. }));
    assert!(matches!(error_of(SolidSource::new("A\0B", vec![group(3, 0.0, &[0, 1, 2])])), WriteError::BadName { .. }));
    assert!(matches!(error_of(SolidSource::new("EMPTY", vec![])), WriteError::NoGroups { .. }));

    let solid = |g| SolidSource::new("S", vec![g]);
    assert_eq!(group_problem(solid(group(0, 0.0, &[0, 1, 2]))), GroupProblem::NoVertices);
    assert_eq!(group_problem(solid(group(3, 0.0, &[]))), GroupProblem::NoIndices);
    assert_eq!(group_problem(solid(group(3, 0.0, &[0, 1, 2, 0]))), GroupProblem::PartialTriangle(4));
    assert_eq!(
        group_problem(solid(group(3, 0.0, &[0, 1, 3]))),
        GroupProblem::IndexOutOfRange { index: 3, vertices: 3 }
    );
    for wide in [2, 19] {
        let mut g = group(3, 0.0, &[0, 1, 2]);
        g.effect_id = wide;
        assert_eq!(group_problem(solid(g)), GroupProblem::WideEffect(wide));
    }
    // A tangent effect needs one tangent per vertex; any other effect takes none.
    let mut g = group(3, 0.0, &[0, 1, 2]);
    g.effect_id = 3;
    assert_eq!(group_problem(solid(g)), GroupProblem::Tangents { effect_id: 3, tangents: 0, vertices: 3 });
    let mut g = group(3, 0.0, &[0, 1, 2]);
    g.tangents = vec![[1.0, 0.0, 0.0, 1.0]; 3];
    assert_eq!(group_problem(solid(g)), GroupProblem::Tangents { effect_id: 4, tangents: 3, vertices: 3 });
    let mut g = group(3, 0.0, &[0, 1, 2]);
    g.light_material = 0;
    assert_eq!(group_problem(solid(g)), GroupProblem::LightMaterialOutOfRange { light_material: 0, listed: 0 });
}

#[test]
fn safe_effect_ids_are_accepted() {
    for effect in [0, 4, 5, 6] {
        let mut g = group(3, 0.0, &[0, 1, 2]);
        g.effect_id = effect;
        let file = write_geometry_pack(&[SolidSource::new("E", vec![g])]).unwrap();
        assert_eq!(read_solids(&file).unwrap()[0].groups[0].effect_id, effect);
    }
}

#[test]
fn a_tangent_effect_writes_60_byte_vertices_that_read_back_with_their_tangents() {
    let mut plain = group(3, 0.0, &[0, 1, 2]);
    plain.effect_id = 4;
    let mut bumped = group(4, 10.0, &[0, 1, 2, 2, 1, 3]);
    bumped.effect_id = 3;
    bumped.tangents = (0..4).map(|i| [1.0, i as f32, 0.0, -1.0]).collect();
    let file = write_geometry_pack(&[SolidSource::new("BUMP", vec![plain, bumped.clone()])]).unwrap();
    let solid = &read_solids(&file).unwrap()[0];
    assert_eq!(solid.vertex_buffers.iter().map(|b| b.stride).collect::<Vec<_>>(), [36, 60]);
    let tangents = solid.tangents.as_ref().expect("the tangent buffer is read");
    assert_eq!(tangents.len(), 7);
    assert_eq!(&tangents[3..], bumped.tangents.as_slice());
    assert_eq!(tangents[0], [0.0; 4]);
    assert_eq!(solid.vertices[3..], bumped.vertices[..]);
    assert_eq!(solid.secondary_uv(4), bumped.vertices[1].uv);
}

#[test]
fn duplicate_names_and_oversized_buffers_are_refused() {
    let a = SolidSource::new("SAME", vec![group(3, 0.0, &[0, 1, 2])]);
    assert!(matches!(write_geometry_pack(&[a.clone(), a]), Err(WriteError::DuplicateHash { .. })));

    // Two groups of one effect make one buffer, and 16-bit indices cannot reach past 65536 vertices.
    let big = |n| group(n, 0.0, &[0, 1, 2]);
    let ok = SolidSource::new("OK", vec![big(0x8000), big(0x8000)]);
    assert_eq!(
        read_solids(&write_geometry_pack(&[ok]).unwrap()).unwrap()[0].indices,
        [0, 1, 2, 0x8000, 0x8001, 0x8002]
    );
    let too_big = SolidSource::new("BIG", vec![big(0x8000), big(0x8001)]);
    assert_eq!(group_problem(too_big), GroupProblem::BufferTooLarge { vertices: 0x1_0001 });
    // Different effects are different buffers, so the same vertex total is fine.
    let mut other = big(0x8001);
    other.effect_id = 0;
    assert!(write_geometry_pack(&[SolidSource::new("SPLIT", vec![big(0x8000), other])]).is_ok());
}

#[test]
fn more_than_255_textures_are_refused() {
    let mut s = SolidSource::new("T", vec![group(3, 0.0, &[0, 1, 2])]);
    s.texture_hashes = (0..256).collect();
    assert!(matches!(error_of(s), WriteError::TooMany { what: "textures", count: 256, .. }));
}

#[test]
fn skinned_solids_cannot_be_turned_into_sources() {
    let file = super::builder::solid_file(
        "SKIN",
        &[super::builder::Group { effect: 0, vertices: 3, indices: vec![0, 1, 2], bone_table: None }],
        &[super::builder::vertex_buffer(3, 60, 0.0)],
    );
    let solid = &read_solids(&file).unwrap()[0];
    assert!(matches!(SolidSource::from_solid(solid), Err(WriteError::Unsupported { .. })));
}

#[test]
fn an_odd_index_total_is_padded_with_a_zero_the_groups_do_not_use() {
    // One triangle: three indices, padded to four. The group still draws three.
    let file = write_geometry_pack(&[SolidSource::new("ODD", vec![group(3, 0.0, &[2, 1, 0])])]).unwrap();
    let solid = &read_solids(&file).unwrap()[0];
    assert_eq!(solid.indices, [2, 1, 0, 0]);
    assert_eq!(solid.groups[0].num_indices, 3);
    let pack = top_chunks(&file)[0].find(ids::SOLID_PACK).unwrap();
    let header = pack.child(ids::MESH_INFO_CONTAINER).unwrap().child(ids::MESH_INFO_HEADER).unwrap();
    assert_eq!(u32_at(header.aligned_payload(0x10), 0x2C), 3, "the header counts the real indices");
}
