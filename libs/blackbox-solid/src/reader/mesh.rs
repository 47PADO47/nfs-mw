//! The mesh: shading groups, indices and vertex buffers.
//!
//! A solid has one vertex buffer per run of consecutive shading groups with the
//! same effect id; the groups of a run share the buffer in order, and their
//! indices are relative to the buffer's start. Verified on every car and world
//! solid of NFS: MW (`docs/formats/models.md`).

use blackbox_chunk::{Chunk, ids};

use crate::bytes::{f32_at, u32_at, vec3_at};
use crate::layout::{GroupLayout, SolidLayout};
use crate::{BASE_VERTEX_STRIDE, ShadingGroup, Solid, Vertex, VertexBuffer};

pub(super) fn read_mesh(mesh: Chunk<'_>, layout: &SolidLayout, solid: &mut Solid) -> Result<(), String> {
    if let Some(groups) = mesh.child(ids::MESH_SHADING_GROUPS) {
        let groups = groups.aligned_payload(layout.groups_align);
        let len = layout.group.len;
        if groups.len() % len != 0 {
            return Err(format!("shading groups: {} bytes is not a multiple of {len}", groups.len()));
        }
        solid.groups = groups.chunks_exact(len).map(|r| read_group(r, &layout.group)).collect();
    }
    if let Some(ib) = mesh.child(ids::MESH_INDICES) {
        solid.indices = ib
            .aligned_payload(layout.indices_align)
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes(*b))
            .collect();
    }
    let buffers: Vec<&[u8]> = mesh
        .children()
        .filter_map(|c| c.ok())
        .filter(|c| c.id == ids::MESH_VERTEX_BUFFER)
        .map(|c| c.aligned_payload(layout.vertices_align))
        .collect();
    read_vertex_buffers(&buffers, solid)?;
    check_index_ranges(solid)
}

fn read_group(r: &[u8], l: &GroupLayout) -> ShadingGroup {
    let t = l.texture_slots;
    ShadingGroup {
        bounds_min: vec3_at(r, l.bounds_min),
        bounds_max: vec3_at(r, l.bounds_max),
        texture_slots: [r[t], r[t + 1], r[t + 2], r[t + 3], r[t + 4]],
        light_material: r[t + 5],
        effect_id: u32_at(r, l.effect_id),
        flags: u32_at(r, l.flags),
        num_vertices: u32_at(r, l.num_vertices),
        num_triangles: u32_at(r, l.num_triangles),
        first_index: u32_at(r, l.first_index),
        num_indices: u32_at(r, l.num_indices),
        base_vertex: 0,
        vertex_buffer: 0,
    }
}

/// Assign groups to buffers (one buffer per run of equal effect ids), decode every buffer.
fn read_vertex_buffers(buffers: &[&[u8]], solid: &mut Solid) -> Result<(), String> {
    let mut runs: Vec<std::ops::Range<usize>> = Vec::new();
    for (i, g) in solid.groups.iter().enumerate() {
        match runs.last_mut() {
            Some(run) if solid.groups[run.start].effect_id == g.effect_id => run.end = i + 1,
            _ => runs.push(i..i + 1),
        }
    }
    if buffers.is_empty() {
        return Ok(());
    }
    if runs.len() != buffers.len() {
        return Err(format!("{} vertex buffers for {} effect runs", buffers.len(), runs.len()));
    }
    for (index, (run, data)) in runs.into_iter().zip(buffers).enumerate() {
        let count: u32 = solid.groups[run.clone()].iter().map(|g| g.num_vertices).sum();
        if count == 0 || data.len() % count as usize != 0 {
            return Err(format!("vertex buffer {index} of {} bytes does not hold {count} vertices", data.len()));
        }
        let stride = data.len() / count as usize;
        if stride < BASE_VERTEX_STRIDE {
            return Err(format!("vertex buffer {index}: unexpected stride {stride}"));
        }
        let first_vertex = solid.vertices.len() as u32;
        for g in &mut solid.groups[run] {
            g.vertex_buffer = index as u32;
            g.base_vertex = first_vertex;
        }
        solid.vertices.extend(data.chunks_exact(stride).map(read_vertex));
        solid.vertex_buffers.push(VertexBuffer { stride, first_vertex, vertex_count: count });
    }
    Ok(())
}

fn read_vertex(v: &[u8]) -> Vertex {
    Vertex {
        position: vec3_at(v, 0),
        normal: vec3_at(v, 12),
        color_bgra: [v[24], v[25], v[26], v[27]],
        uv: [f32_at(v, 28), f32_at(v, 32)],
    }
}

fn check_index_ranges(solid: &Solid) -> Result<(), String> {
    for g in &solid.groups {
        let end = g.first_index as usize + g.num_indices as usize;
        if end > solid.indices.len() {
            return Err(format!("shading group indices end at {end}, past {}", solid.indices.len()));
        }
    }
    Ok(())
}
