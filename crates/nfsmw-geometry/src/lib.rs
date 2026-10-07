//! Solids ("models") from `GeometryPack` chunks. Spec: `docs/formats/models.md`.
//!
//! Works on a byte slice that has already had any whole-file wrapper removed.
//! Bare JDLZ blobs standing in for `SolidPack` chunks (add-on cars) are inflated
//! here.

use nfsmw_bchunk::{Chunk, ids};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Chunk(#[from] nfsmw_bchunk::Error),
    #[error(transparent)]
    Compress(#[from] nfsmw_compress::Error),
    #[error("solid at 0x{offset:X}: {detail}")]
    Solid { offset: usize, detail: String },
}

pub type Result<T> = std::result::Result<T, Error>;

/// Size of one shading-group record (`MeshShaderInfos`).
pub const SHADING_GROUP_LEN: usize = 104;
/// The common 36-byte vertex: position, normal, D3DCOLOR, uv.
pub const BASE_VERTEX_STRIDE: usize = 36;

/// One vertex's common attributes (the first 36 bytes of every vertex format).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// D3DCOLOR as stored: bytes B, G, R, A.
    pub color_bgra: [u8; 4],
    pub uv: [f32; 2],
}

/// One material within a solid: a range of indices drawn with one effect and texture set.
#[derive(Debug, Clone, PartialEq)]
pub struct ShadingGroup {
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    /// Indices into [`Solid::texture_hashes`]: diffuse, normal, height, specular, opacity.
    pub texture_slots: [u8; 5],
    pub light_material: u8,
    pub effect_id: u32,
    pub flags: u32,
    pub num_vertices: u32,
    pub num_triangles: u32,
    /// First index of this group in [`Solid::indices`].
    pub first_index: u32,
    pub num_indices: u32,
}

impl ShadingGroup {
    /// Hash of the diffuse texture, if the slot points at a listed texture.
    pub fn diffuse_texture(&self, solid: &Solid) -> Option<u32> {
        solid.texture_hashes.get(usize::from(self.texture_slots[0])).copied()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Solid {
    pub name: String,
    pub name_hash: u32,
    pub version: u8,
    pub flags: u16,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    /// Pivot matrix, row-major as stored.
    pub transform: [f32; 16],
    /// bStringHash of each texture the solid uses.
    pub texture_hashes: Vec<u32>,
    pub light_material_hashes: Vec<u32>,
    /// Bytes per vertex in the original buffer (36 or 60).
    pub vertex_stride: usize,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub groups: Vec<ShadingGroup>,
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn vec3_at(b: &[u8], o: usize) -> [f32; 3] {
    [f32_at(b, o), f32_at(b, o + 4), f32_at(b, o + 8)]
}

/// Every solid in every `GeometryPack` of `data`.
pub fn read_solids(data: &[u8]) -> Result<Vec<Solid>> {
    let mut solids = Vec::new();
    for pack in nfsmw_bchunk::find_all(data, ids::GEOMETRY_PACK) {
        read_pack_children(pack, &mut solids)?;
    }
    Ok(solids)
}

fn read_pack_children(pack: Chunk<'_>, out: &mut Vec<Solid>) -> Result<()> {
    for child in pack.children() {
        let child = child?;
        if child.id == ids::SOLID_PACK {
            out.push(read_solid(child)?);
        } else if child.is_bare_jdlz() {
            // Add-on cars: a compressed SolidPack. Offsets inside are blob-relative.
            let inflated = nfsmw_compress::jdlz_decompress(child.payload)?;
            for inner in nfsmw_bchunk::chunks(&inflated) {
                let inner = inner?;
                if inner.id == ids::SOLID_PACK {
                    out.push(read_solid(inner)?);
                }
            }
        }
    }
    Ok(())
}

/// Parse one `SolidPack` (`0x80134010`) chunk.
pub fn read_solid(pack: Chunk<'_>) -> Result<Solid> {
    let err = |detail: String| Error::Solid { offset: pack.offset, detail };

    let info = pack.child(ids::SOLID_INFO).ok_or_else(|| err("no SolidInfo".into()))?;
    let info = info.aligned_payload(0x10);
    if info.len() < 0xA0 {
        return Err(err(format!("SolidInfo too short ({} bytes)", info.len())));
    }
    let name_bytes = &info[0xA0..];
    let name_len = name_bytes.iter().position(|&b| b == 0).unwrap_or(name_bytes.len());
    let name = String::from_utf8_lossy(&name_bytes[..name_len]).into_owned();
    let mut transform = [0f32; 16];
    for (i, t) in transform.iter_mut().enumerate() {
        *t = f32_at(info, 0x40 + i * 4);
    }

    let hash_list = |id| -> Vec<u32> {
        pack.child(id).map(|c| c.payload.as_chunks::<8>().0.iter().map(|r| u32_at(r, 0)).collect()).unwrap_or_default()
    };

    let mut solid = Solid {
        name,
        name_hash: u32_at(info, 0x10),
        version: info[0x0C],
        flags: u16_at(info, 0x0E),
        bounds_min: vec3_at(info, 0x20),
        bounds_max: vec3_at(info, 0x30),
        transform,
        texture_hashes: hash_list(ids::SOLID_TEXTURES),
        light_material_hashes: hash_list(ids::SOLID_LIGHT_MATERIALS),
        vertex_stride: BASE_VERTEX_STRIDE,
        vertices: Vec::new(),
        indices: Vec::new(),
        groups: Vec::new(),
    };

    let Some(mesh) = pack.child(ids::MESH_INFO_CONTAINER) else {
        return Ok(solid); // some solids carry no mesh
    };

    if let Some(groups) = mesh.child(ids::MESH_SHADING_GROUPS) {
        let groups = groups.aligned_payload(0x10);
        if groups.len() % SHADING_GROUP_LEN != 0 {
            return Err(err(format!("shading groups: {} bytes is not a multiple of 104", groups.len())));
        }
        solid.groups = groups.as_chunks::<SHADING_GROUP_LEN>().0.iter().map(|r| read_group(r)).collect();
    }

    if let Some(ib) = mesh.child(ids::MESH_INDICES) {
        solid.indices = ib.aligned_payload(0x10).as_chunks::<2>().0.iter().map(|b| u16::from_le_bytes(*b)).collect();
    }

    let vertex_buffers: Vec<_> =
        mesh.children().filter_map(|c| c.ok()).filter(|c| c.id == ids::MESH_VERTEX_BUFFER).collect();
    let total_verts: usize = solid.groups.iter().map(|g| g.num_vertices as usize).sum();
    match vertex_buffers.as_slice() {
        [] => {}
        [vb] => {
            let vb = vb.aligned_payload(0x80);
            if total_verts == 0 || vb.len() % total_verts != 0 {
                return Err(err(format!("vertex buffer of {} bytes does not hold {total_verts} vertices", vb.len())));
            }
            solid.vertex_stride = vb.len() / total_verts;
            if solid.vertex_stride < BASE_VERTEX_STRIDE {
                return Err(err(format!("unexpected vertex stride {}", solid.vertex_stride)));
            }
            solid.vertices = vb.chunks_exact(solid.vertex_stride).map(read_vertex).collect();
        }
        _ => {
            return Err(err(format!("{} vertex buffers in one solid are not supported yet", vertex_buffers.len())));
        }
    }

    for g in &solid.groups {
        let end = g.first_index as usize + g.num_indices as usize;
        if end > solid.indices.len() {
            return Err(err(format!("shading group indices end at {end}, past {}", solid.indices.len())));
        }
    }
    Ok(solid)
}

fn read_group(r: &[u8]) -> ShadingGroup {
    ShadingGroup {
        bounds_min: vec3_at(r, 0x00),
        bounds_max: vec3_at(r, 0x0C),
        texture_slots: [r[0x18], r[0x19], r[0x1A], r[0x1B], r[0x1C]],
        light_material: r[0x1D],
        effect_id: u32_at(r, 0x30),
        flags: u32_at(r, 0x38),
        num_vertices: u32_at(r, 0x3C),
        num_triangles: u32_at(r, 0x40),
        first_index: u32_at(r, 0x44),
        num_indices: u32_at(r, 0x5C),
    }
}

fn read_vertex(v: &[u8]) -> Vertex {
    Vertex {
        position: vec3_at(v, 0),
        normal: vec3_at(v, 12),
        color_bgra: [v[24], v[25], v[26], v[27]],
        uv: [f32_at(v, 28), f32_at(v, 32)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
        let mut v = id.to_le_bytes().to_vec();
        v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        v.extend_from_slice(payload);
        v
    }

    /// Pads `payload` with 0x11 so that, placed after a header ending at `at`, it starts aligned.
    fn aligned(at: usize, align: usize, payload: &[u8]) -> Vec<u8> {
        let pad = at.next_multiple_of(align) - at;
        let mut v = vec![0x11; pad];
        v.extend_from_slice(payload);
        v
    }

    fn synthetic_solid() -> Vec<u8> {
        // SolidInfo
        let mut info = vec![0u8; 0xA0];
        info[0x0C] = 0x16;
        info[0x10..0x14].copy_from_slice(&nfsmw_hash_stub("TRI").to_le_bytes());
        info.extend_from_slice(b"TRI\0");
        // One group: 3 verts, 1 tri, indices 0..3, diffuse slot 0.
        let mut g = vec![0u8; SHADING_GROUP_LEN];
        g[0x3C..0x40].copy_from_slice(&3u32.to_le_bytes());
        g[0x40..0x44].copy_from_slice(&1u32.to_le_bytes());
        g[0x5C..0x60].copy_from_slice(&3u32.to_le_bytes());
        let mut vb = Vec::new();
        for i in 0..3u8 {
            let mut v = vec![0u8; 36];
            v[0..4].copy_from_slice(&f32::from(i).to_le_bytes());
            v[24..28].copy_from_slice(&[1, 2, 3, 4]);
            vb.extend_from_slice(&v);
        }
        let ib: Vec<u8> = [0u16, 1, 2].iter().flat_map(|i| i.to_le_bytes()).collect();

        // Lay the file out by hand so alignment is computed from real offsets.
        let mut file = Vec::new();
        let mut body = Vec::new(); // SolidPack payload, starts at 16 (two headers)
        let pack_payload_at = 16;
        let push = |body: &mut Vec<u8>, id: u32, align: usize, p: &[u8]| {
            let at = pack_payload_at + body.len() + 8;
            body.extend_from_slice(&chunk(id, &aligned(at, align, p)));
        };
        push(&mut body, ids::SOLID_INFO, 0x10, &info);
        push(&mut body, ids::SOLID_TEXTURES, 1, &[0xAA, 0, 0, 0, 0, 0, 0, 0]);
        let mut mesh = Vec::new();
        let mesh_at = pack_payload_at + body.len() + 8;
        let push_mesh = |mesh: &mut Vec<u8>, id: u32, align: usize, p: &[u8]| {
            let at = mesh_at + mesh.len() + 8;
            mesh.extend_from_slice(&chunk(id, &aligned(at, align, p)));
        };
        push_mesh(&mut mesh, ids::MESH_SHADING_GROUPS, 0x10, &g);
        push_mesh(&mut mesh, ids::MESH_INDICES, 0x10, &ib);
        push_mesh(&mut mesh, ids::MESH_VERTEX_BUFFER, 0x80, &vb);
        body.extend_from_slice(&chunk(ids::MESH_INFO_CONTAINER, &mesh));
        let pack = chunk(ids::SOLID_PACK, &body);
        file.extend_from_slice(&chunk(ids::GEOMETRY_PACK, &pack));
        file
    }

    fn nfsmw_hash_stub(s: &str) -> u32 {
        s.bytes().fold(0xFFFF_FFFFu32, |h, c| h.wrapping_mul(33).wrapping_add(u32::from(c)))
    }

    #[test]
    fn reads_synthetic_solid() {
        let file = synthetic_solid();
        let solids = read_solids(&file).unwrap();
        assert_eq!(solids.len(), 1);
        let s = &solids[0];
        assert_eq!(s.name, "TRI");
        assert_eq!(s.version, 0x16);
        assert_eq!(s.name_hash, nfsmw_hash_stub("TRI"));
        assert_eq!(s.texture_hashes, [0xAA]);
        assert_eq!(s.vertex_stride, 36);
        assert_eq!(s.vertices.len(), 3);
        assert_eq!(s.vertices[2].position, [2.0, 0.0, 0.0]);
        assert_eq!(s.vertices[0].color_bgra, [1, 2, 3, 4]);
        assert_eq!(s.indices, [0, 1, 2]);
        assert_eq!(s.groups[0].num_indices, 3);
        assert_eq!(s.groups[0].diffuse_texture(s), Some(0xAA));
    }
}
