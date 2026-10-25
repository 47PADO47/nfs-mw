//! One `SolidPack` chunk: header, texture and light-material lists, markers, mesh.

use blackbox_chunk::{ChunkWriter, ids};

use super::error::WriteError;
use super::info::info_payload;
use super::mesh::write_mesh;
use super::plan::plan;
use super::put::{hash_pairs, put_f32, put_u32};
use super::source::SolidSource;
use crate::PositionMarker;
use crate::layout::{MarkerLayout, SolidLayout};

/// The chunk, header included, as a stream of its own. It starts at offset 0 of the returned buffer, so the buffer has
/// to be put at an address that is a multiple of 0x80 for the payload alignments to hold (the pack file does that).
///
/// Children in the stock order. The texture and light-material lists are left out when empty.
pub(super) fn build_solid_pack(solid: &SolidSource, layout: &SolidLayout) -> Result<Vec<u8>, WriteError> {
    let plan = plan(&solid.groups);
    // Validated: at most 65535 triangles in total.
    let counted = solid.groups.iter().map(|g| g.indices.len() / 3).sum::<usize>() as u16;
    let triangles = solid.polygon_count.unwrap_or(counted);
    let mut w = ChunkWriter::new();
    w.container(ids::SOLID_PACK, |w| {
        w.leaf_aligned(ids::SOLID_INFO, layout.info.align, &info_payload(solid, triangles, layout));
        if !solid.texture_hashes.is_empty() {
            w.leaf(ids::SOLID_TEXTURES, &hash_pairs(solid.texture_hashes.iter().copied()));
        }
        if !solid.light_material_hashes.is_empty() {
            w.leaf(ids::SOLID_LIGHT_MATERIALS, &hash_pairs(solid.light_material_hashes.iter().copied()));
        }
        if !solid.markers.is_empty() {
            w.leaf_aligned(ids::SOLID_MARKERS, layout.marker.align, &marker_records(&solid.markers, &layout.marker));
        }
        write_mesh(w, solid, &plan, layout);
    });
    Ok(w.finish()?)
}

fn marker_records(markers: &[PositionMarker], l: &MarkerLayout) -> Vec<u8> {
    let mut out = Vec::with_capacity(markers.len() * l.len);
    for m in markers {
        let mut r = vec![0u8; l.len];
        put_u32(&mut r, l.name_hash, m.name_hash);
        put_u32(&mut r, l.int_param, m.int_param as u32);
        for (i, f) in m.float_params.into_iter().enumerate() {
            put_f32(&mut r, l.float_params + i * 4, f);
        }
        for (i, f) in m.matrix.into_iter().enumerate() {
            put_f32(&mut r, l.matrix + i * 4, f);
        }
        out.extend(r);
    }
    out
}
