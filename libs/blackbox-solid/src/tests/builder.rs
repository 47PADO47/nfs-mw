//! Builds synthetic `GeometryPack` files with correct alignment padding.

use blackbox_chunk::ids;

pub(super) fn bstring_hash(s: &str) -> u32 {
    s.bytes().fold(0xFFFF_FFFFu32, |h, c| h.wrapping_mul(33).wrapping_add(u32::from(c)))
}

fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = id.to_le_bytes().to_vec();
    v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    v.extend_from_slice(payload);
    v
}

/// Appends a chunk whose payload is 0x11-padded to `align`, given that `body` starts at `body_at`.
fn push_aligned(body: &mut Vec<u8>, body_at: usize, id: u32, align: usize, payload: &[u8]) {
    let at = body_at + body.len() + 8;
    let mut padded = vec![0x11; at.next_multiple_of(align) - at];
    padded.extend_from_slice(payload);
    body.extend_from_slice(&chunk(id, &padded));
}

/// One synthetic shading group.
pub(super) struct Group {
    pub effect: u32,
    pub vertices: u32,
    /// Indices relative to the group's vertex buffer.
    pub indices: Vec<u16>,
}

fn group_record(g: &Group, first_index: u32) -> Vec<u8> {
    let mut r = vec![0u8; 104];
    r[0x1D] = 0xFF;
    r[0x30..0x34].copy_from_slice(&g.effect.to_le_bytes());
    r[0x3C..0x40].copy_from_slice(&g.vertices.to_le_bytes());
    r[0x40..0x44].copy_from_slice(&(g.indices.len() as u32 / 3).to_le_bytes());
    r[0x44..0x48].copy_from_slice(&first_index.to_le_bytes());
    r[0x5C..0x60].copy_from_slice(&(g.indices.len() as u32).to_le_bytes());
    r
}

/// A vertex buffer of `count` vertices of `stride` bytes; vertex `i` has x = `x0 + i`.
pub(super) fn vertex_buffer(count: u32, stride: usize, x0: f32) -> Vec<u8> {
    let mut vb = Vec::new();
    for i in 0..count {
        let mut v = vec![0u8; stride];
        v[0..4].copy_from_slice(&(x0 + i as f32).to_le_bytes());
        v[24..28].copy_from_slice(&[1, 2, 3, 4]);
        vb.extend_from_slice(&v);
    }
    vb
}

/// A file holding one GeometryPack with one solid.
pub(super) fn solid_file(name: &str, groups: &[Group], vertex_buffers: &[Vec<u8>]) -> Vec<u8> {
    solid_file_with_markers(name, groups, vertex_buffers, &[])
}

/// Like [`solid_file`], with position markers given as (name hash, translation).
pub(super) fn solid_file_with_markers(
    name: &str,
    groups: &[Group],
    vertex_buffers: &[Vec<u8>],
    markers: &[(u32, [f32; 3])],
) -> Vec<u8> {
    let mut info = vec![0u8; 0xA0];
    info[0x0C] = 0x16;
    info[0x10..0x14].copy_from_slice(&bstring_hash(name).to_le_bytes());
    info.extend_from_slice(name.as_bytes());
    info.push(0);

    let mut records = Vec::new();
    let mut indices = Vec::new();
    for g in groups {
        records.extend(group_record(g, indices.len() as u32));
        indices.extend(g.indices.iter().copied());
    }
    let index_bytes: Vec<u8> = indices.iter().flat_map(|i| i.to_le_bytes()).collect();

    // GeometryPack header (8) + SolidPack header (8): the SolidPack body starts at 16.
    let mut body = Vec::new();
    push_aligned(&mut body, 16, ids::SOLID_INFO, 0x10, &info);
    push_aligned(&mut body, 16, ids::SOLID_TEXTURES, 1, &[0xAA, 0, 0, 0, 0, 0, 0, 0]);
    if !markers.is_empty() {
        push_aligned(&mut body, 16, ids::SOLID_MARKERS, 0x10, &marker_records(markers));
    }
    let mesh_at = 16 + body.len() + 8;
    let mut mesh = Vec::new();
    push_aligned(&mut mesh, mesh_at, ids::MESH_SHADING_GROUPS, 0x10, &records);
    push_aligned(&mut mesh, mesh_at, ids::MESH_INDICES, 0x10, &index_bytes);
    for vb in vertex_buffers {
        push_aligned(&mut mesh, mesh_at, ids::MESH_VERTEX_BUFFER, 0x80, vb);
    }
    body.extend_from_slice(&chunk(ids::MESH_INFO_CONTAINER, &mesh));
    chunk(ids::GEOMETRY_PACK, &chunk(ids::SOLID_PACK, &body))
}

fn marker_records(markers: &[(u32, [f32; 3])]) -> Vec<u8> {
    let mut out = Vec::new();
    for (hash, t) in markers {
        let mut r = vec![0u8; 0x50];
        r[0..4].copy_from_slice(&hash.to_le_bytes());
        // Identity rotation, then the translation row.
        for (i, v) in
            [1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, t[0], t[1], t[2], 1.0].iter().enumerate()
        {
            r[0x10 + i * 4..0x14 + i * 4].copy_from_slice(&v.to_le_bytes());
        }
        out.extend(r);
    }
    out
}
