//! One `SolidPack`: header, texture and light-material lists, then the mesh.

use blackbox_chunk::{Chunk, ids};

use crate::bytes::{cstr, f32_at, u16_at, u32_at, vec3_at};
use crate::layout::MarkerLayout;
use crate::layout::{self, VERSION_OFFSET};
use crate::{Error, PositionMarker, Result, Solid};

/// Parse one `SolidPack` (`0x80134010`) chunk.
pub fn read_solid(pack: Chunk<'_>) -> Result<Solid> {
    let err = |detail: String| Error::Solid { offset: pack.offset, detail };

    let info_chunk = pack.child(ids::SOLID_INFO).ok_or_else(|| err("no SolidInfo".into()))?;
    // Every known layout aligns SolidInfo to 0x10; the version byte then picks the layout.
    let info = info_chunk.aligned_payload(0x10);
    let version = *info.get(VERSION_OFFSET).ok_or_else(|| err("SolidInfo too short".into()))?;
    let layout = layout::for_version(version).ok_or(Error::UnsupportedVersion { offset: pack.offset, version })?;
    let l = &layout.info;
    let info = info_chunk.aligned_payload(l.align);
    if info.len() < l.name {
        return Err(err(format!("SolidInfo too short ({} bytes)", info.len())));
    }
    let mut transform = [0f32; 16];
    for (i, t) in transform.iter_mut().enumerate() {
        *t = f32_at(info, l.transform + i * 4);
    }
    // (hash, 0) pairs.
    let hash_list = |id| -> Vec<u32> {
        pack.child(id).map(|c| c.payload.as_chunks::<8>().0.iter().map(|r| u32_at(r, 0)).collect()).unwrap_or_default()
    };

    let mut solid = Solid {
        name: cstr(&info[l.name..]),
        name_hash: u32_at(info, l.name_hash),
        version,
        flags: u16_at(info, l.flags),
        bounds_min: vec3_at(info, l.bounds_min),
        bounds_max: vec3_at(info, l.bounds_max),
        transform,
        num_polys: u16_at(info, l.num_polys),
        density: f32_at(info, l.density),
        texture_hashes: hash_list(ids::SOLID_TEXTURES),
        light_material_hashes: hash_list(ids::SOLID_LIGHT_MATERIALS),
        markers: pack.child(ids::SOLID_MARKERS).map(|c| read_markers(c, &layout.marker)).unwrap_or_default(),
        vertex_buffers: Vec::new(),
        vertices: Vec::new(),
        indices: Vec::new(),
        groups: Vec::new(),
    };
    if let Some(mesh) = pack.child(ids::MESH_INFO_CONTAINER) {
        super::mesh::read_mesh(mesh, layout, &mut solid).map_err(err)?;
    }
    Ok(solid)
}

/// `SolidMarkers`: fixed-size records after the alignment padding.
fn read_markers(chunk: Chunk<'_>, l: &MarkerLayout) -> Vec<PositionMarker> {
    chunk
        .aligned_payload(l.align)
        .chunks_exact(l.len)
        .map(|r| PositionMarker {
            name_hash: u32_at(r, l.name_hash),
            int_param: u32_at(r, l.int_param) as i32,
            float_params: [f32_at(r, l.float_params), f32_at(r, l.float_params + 4)],
            matrix: std::array::from_fn(|i| f32_at(r, l.matrix + i * 4)),
        })
        .collect()
}
