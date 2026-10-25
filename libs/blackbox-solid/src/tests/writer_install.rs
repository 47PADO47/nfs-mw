//! Against a real NFS: Most Wanted install (set `NFSMW_GAME_DIR`): the stock car solids, written back with
//! `write_geometry_pack`, read again as the same solids, and the pack's tables agree with the file. Nothing is
//! written to disk.

use std::collections::BTreeMap;

use blackbox_chunk::{Chunk, ids};

use crate::bytes::u32_at;
use crate::{Solid, SolidSource, read_solids_lossy, write_geometry_pack};

#[test]
#[ignore = "reads the game install (NFSMW_GAME_DIR)"]
fn stock_car_geometry_rewrites_to_the_same_solids() {
    let Some(dir) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let cars = std::path::Path::new(&dir).join("CARS");
    let (mut files, mut solids_checked, mut skipped) = (0usize, 0usize, 0usize);
    let mut mismatches = Vec::new();
    for entry in std::fs::read_dir(&cars).unwrap().flatten() {
        let Ok(data) = std::fs::read(entry.path().join("GEOMETRY.BIN")) else { continue };
        let Ok((solids, _)) = read_solids_lossy(&data) else { continue };
        let car = entry.file_name().to_string_lossy().into_owned();
        // Only solids the writer can produce (36-byte vertices, no wide effect), and the first of any name: two stock
        // files hold a second `KIT00_DRIVER_A`, which the writer refuses as a duplicate.
        let mut seen = std::collections::HashSet::new();
        let pairs: Vec<(&Solid, SolidSource)> = solids
            .iter()
            .filter_map(|s| Some((s, SolidSource::from_solid(s).ok()?)))
            .filter(|(s, _)| seen.insert(s.name_hash))
            .collect();
        skipped += solids.len() - pairs.len();
        if pairs.is_empty() {
            continue;
        }
        let sources: Vec<SolidSource> = pairs.iter().map(|(_, source)| source.clone()).collect();
        let written = match write_geometry_pack(&sources) {
            Ok(written) => written,
            Err(e) => {
                mismatches.push(format!("{car}: the pack was refused: {e}"));
                continue;
            }
        };
        let (again, failed) = read_solids_lossy(&written).unwrap();
        assert!(failed.is_empty(), "{car}: {failed:?}");
        files += 1;
        assert_eq!(again.len(), pairs.len(), "{car}");
        for ((before, _), after) in pairs.iter().zip(&again) {
            solids_checked += 1;
            let differing = differences(before, after);
            if !differing.is_empty() {
                mismatches.push(format!("{car}/{}: differs in {differing:?}", before.name));
            }
        }
        check_tables(&car, &written, again.len());
    }
    eprintln!("{files} car files rewritten, {solids_checked} solids compared, {skipped} not rewritable");
    assert!(files > 60, "the install has about 90 car folders with plain geometry");
    assert!(
        mismatches.is_empty(),
        "{} problems, first: {:?}",
        mismatches.len(),
        &mismatches[..mismatches.len().min(5)]
    );
}

/// Keys sorted, offsets pointing at `SolidPack`s of the listed hashes, as in the stock files.
fn check_tables(car: &str, file: &[u8], solids: usize) {
    let pack = blackbox_chunk::find(file, ids::GEOMETRY_PACK).unwrap();
    let info = pack.child(ids::MESH_CONTAINER_INFO).unwrap();
    let keys = info.child(ids::MESH_CONTAINER_KEYS).unwrap().payload;
    let offsets = info.child(ids::MESH_CONTAINER_OFFSETS).unwrap().payload;
    assert_eq!((keys.len() / 8, offsets.len() / 24), (solids, solids), "{car}");
    let hashes: Vec<u32> = keys.as_chunks::<8>().0.iter().map(|k| u32_at(k, 0)).collect();
    assert!(hashes.windows(2).all(|w| w[0] < w[1]), "{car}: keys sorted");
    for (hash, entry) in hashes.into_iter().zip(offsets.as_chunks::<24>().0.iter()) {
        assert_eq!(u32_at(entry, 0), hash, "{car}");
        let at = u32_at(entry, 4) as usize;
        let chunk = blackbox_chunk::chunks(&file[at..]).next().unwrap().unwrap();
        assert_eq!(chunk.id, ids::SOLID_PACK, "{car}");
        assert_eq!(u32_at(entry, 8) as usize, chunk.payload.len() + 8, "{car}");
        let name_hash = u32_at(chunk.child(ids::SOLID_INFO).unwrap().aligned_payload(0x10), 0x10);
        assert_eq!(name_hash, hash, "{car}");
    }
}

/// The names of the fields of two solids that are not equal.
fn differences(a: &Solid, b: &Solid) -> Vec<&'static str> {
    let checks = [
        ("name", a.name == b.name),
        ("name_hash", a.name_hash == b.name_hash),
        ("flags", a.flags == b.flags),
        ("bounds", a.bounds_min == b.bounds_min && a.bounds_max == b.bounds_max),
        ("transform", a.transform == b.transform),
        ("num_polys", a.num_polys == b.num_polys),
        ("density", a.density == b.density),
        ("texture_hashes", a.texture_hashes == b.texture_hashes),
        ("light_material_hashes", a.light_material_hashes == b.light_material_hashes),
        ("markers", a.markers == b.markers),
        ("vertex_buffers", a.vertex_buffers == b.vertex_buffers),
        ("vertices", a.vertices == b.vertices),
        ("indices", a.indices == b.indices),
        ("group count", a.groups.len() == b.groups.len()),
        (
            "group bounds",
            a.groups.iter().zip(&b.groups).all(|(x, y)| x.bounds_min == y.bounds_min && x.bounds_max == y.bounds_max),
        ),
        (
            "group slots",
            a.groups
                .iter()
                .zip(&b.groups)
                .all(|(x, y)| x.texture_slots == y.texture_slots && x.light_material == y.light_material),
        ),
        (
            "group effect and flags",
            a.groups.iter().zip(&b.groups).all(|(x, y)| x.effect_id == y.effect_id && x.flags == y.flags),
        ),
        (
            "group ranges",
            a.groups.iter().zip(&b.groups).all(|(x, y)| {
                (x.num_vertices, x.num_triangles, x.first_index, x.num_indices, x.base_vertex, x.vertex_buffer)
                    == (y.num_vertices, y.num_triangles, y.first_index, y.num_indices, y.base_vertex, y.vertex_buffer)
            }),
        ),
        ("group bone tables", a.groups.iter().zip(&b.groups).all(|(x, y)| x.bone_table == y.bone_table)),
    ];
    checks.into_iter().filter(|(_, same)| !same).map(|(field, _)| field).collect()
}

/// The payload of every child of a `SolidPack` and of its mesh container, by chunk id. Payloads include the `0x11`
/// fill, so equal payloads also mean equal alignment.
fn payloads(pack: Chunk<'_>) -> BTreeMap<u32, Vec<u8>> {
    let mut parts = BTreeMap::new();
    for child in pack.children().flatten() {
        if child.id != ids::MESH_INFO_CONTAINER {
            parts.insert(child.id, child.payload.to_vec());
            continue;
        }
        parts.extend(child.children().flatten().map(|c| (c.id, c.payload.to_vec())));
    }
    parts
}

#[test]
#[ignore = "reads the game install (NFSMW_GAME_DIR)"]
fn rewritten_chunks_are_byte_identical_to_the_stock_ones() {
    let Some(dir) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    // The chunks the writer produces. Left out: normal smoothing (0x17 to 0x19) and the material names (0x134C02),
    // which a parsed solid does not carry.
    let written = [
        ids::SOLID_INFO,
        ids::SOLID_TEXTURES,
        ids::SOLID_LIGHT_MATERIALS,
        ids::SOLID_MARKERS,
        ids::MESH_INFO_HEADER,
        ids::MESH_SHADING_GROUPS,
        ids::MESH_INDICES,
        ids::MESH_VERTEX_BUFFER,
    ];
    let mut compared = 0;
    for car in ["BMWM3GTR", "911GT2", "CAMARO", "COPSUV"] {
        let data = std::fs::read(std::path::Path::new(&dir).join("CARS").join(car).join("GEOMETRY.BIN")).unwrap();
        let (solids, _) = read_solids_lossy(&data).unwrap();
        let mut seen = std::collections::HashSet::new();
        // The bounds, density and volume are computed, not copied; only the polygon count (a few solids differ from
        // the triangle total) is taken from the solid.
        let sources: Vec<SolidSource> = solids
            .iter()
            .filter_map(|s| SolidSource::from_solid(s).ok())
            .filter(|s| seen.insert(s.name.clone()))
            .map(|mut s| {
                s.density = None;
                s.groups.iter_mut().for_each(|g| g.bounds = None);
                s
            })
            .collect();
        let out = write_geometry_pack(&sources).unwrap();
        let hash = |p: Chunk<'_>| u32_at(p.child(ids::SOLID_INFO).unwrap().aligned_payload(0x10), 0x10);
        let stock: BTreeMap<u32, Chunk<'_>> =
            blackbox_chunk::find_all(&data, ids::SOLID_PACK).into_iter().map(|p| (hash(p), p)).collect();
        for pack in blackbox_chunk::find_all(&out, ids::SOLID_PACK) {
            let (mine, theirs) = (payloads(pack), payloads(stock[&hash(pack)]));
            for id in written {
                assert_eq!(mine.get(&id), theirs.get(&id), "{car} solid {:08X} chunk {id:08X}", hash(pack));
            }
            compared += 1;
        }
    }
    assert!(compared > 300, "{compared} solids compared");
}
