//! The `MeshInfoContainer`: header, shading groups, indices, vertex buffers and material names.

use blackbox_chunk::{ChunkWriter, ids};

use super::plan::Plan;
use super::put::{put_u32, put_vec3};
use super::source::{GroupSource, SolidSource, padded_bounds};
use crate::layout::{GroupLayout, SolidLayout};
use crate::{BASE_VERTEX_STRIDE, Vertex};

/// `MeshInfoHeader` (`SolidMeshDescriptor`) is 48 bytes, aligned to 0x10. Checked on 14,843 car solids and 20,377
/// world solids; the reader does not use it.
const HEADER_ALIGN: usize = 0x10;
const HEADER_LEN: usize = 48;
/// 18 in every solid of the install.
const KIND_AT: usize = 0x08;
const KIND: u32 = 18;
/// The OR of the groups' flags.
const FLAGS_AT: usize = 0x0C;
const GROUP_COUNT_AT: usize = 0x10;
/// The number of `MeshVertexBuffer` chunks (checked on all 20,377 world solids, which have 0 to 10).
const BUFFER_COUNT_AT: usize = 0x18;
const INDEX_COUNT_AT: usize = 0x2C;

pub(super) fn write_mesh(w: &mut ChunkWriter, solid: &SolidSource, plan: &Plan, layout: &SolidLayout) {
    w.container(ids::MESH_INFO_CONTAINER, |w| {
        w.leaf_aligned(ids::MESH_INFO_HEADER, HEADER_ALIGN, &header(solid, plan));
        w.leaf_aligned(ids::MESH_SHADING_GROUPS, layout.groups_align, &group_records(solid, &layout.group));
        w.leaf_aligned(ids::MESH_INDICES, layout.indices_align, &index_bytes(solid, plan));
        for run in &plan.runs {
            let vertices = vertex_bytes(&solid.groups[run.clone()], layout);
            w.leaf_aligned(ids::MESH_VERTEX_BUFFER, layout.vertices_align, &vertices);
        }
        if solid.groups.iter().any(|g| g.material_name.is_some()) {
            for group in &solid.groups {
                w.leaf(ids::MESH_VLT_MATERIALS, &material_name(group));
            }
        }
    });
}

fn header(solid: &SolidSource, plan: &Plan) -> Vec<u8> {
    let mut b = vec![0u8; HEADER_LEN];
    put_u32(&mut b, KIND_AT, KIND);
    put_u32(&mut b, FLAGS_AT, solid.groups.iter().fold(0, |all, g| all | g.flags));
    put_u32(&mut b, GROUP_COUNT_AT, solid.groups.len() as u32);
    put_u32(&mut b, BUFFER_COUNT_AT, plan.runs.len() as u32);
    put_u32(&mut b, INDEX_COUNT_AT, solid.groups.iter().map(|g| g.indices.len() as u32).sum());
    b
}

/// One 104-byte record per group. The first index of each is where the previous one's indices end.
fn group_records(solid: &SolidSource, l: &GroupLayout) -> Vec<u8> {
    let mut out = Vec::with_capacity(solid.groups.len() * l.len);
    let mut first_index = 0u32;
    for g in &solid.groups {
        let mut r = vec![0u8; l.len];
        let (min, max) = g.bounds.unwrap_or_else(|| padded_bounds(g.vertices.iter()));
        put_vec3(&mut r, l.bounds_min, min);
        put_vec3(&mut r, l.bounds_max, max);
        r[l.texture_slots..l.texture_slots + 5].copy_from_slice(&g.texture_slots);
        r[l.texture_slots + 5] = g.light_material;
        put_u32(&mut r, l.effect_id, g.effect_id);
        put_u32(&mut r, l.flags, g.flags);
        put_u32(&mut r, l.num_vertices, g.vertices.len() as u32);
        put_u32(&mut r, l.num_triangles, g.indices.len() as u32 / 3);
        put_u32(&mut r, l.first_index, first_index);
        put_u32(&mut r, l.num_indices, g.indices.len() as u32);
        first_index += g.indices.len() as u32;
        out.extend(r);
    }
    out
}

/// Every group's indices, moved from "count from the group's first vertex" to "count from the start of the vertex
/// buffer", which is what the file stores. An odd total gets one zero index after it, so the chunk is a whole number
/// of 4-byte units like the stock ones (1,707 of the 5,192 car solids checked have this pad; no group refers to it).
fn index_bytes(solid: &SolidSource, plan: &Plan) -> Vec<u8> {
    let mut out = Vec::new();
    for (g, &base) in solid.groups.iter().zip(&plan.base_in_run) {
        // Validated: a buffer holds at most 65536 vertices and every index is below its group's vertex count.
        out.extend(g.indices.iter().flat_map(|&i| ((usize::from(i) + base) as u16).to_le_bytes()));
    }
    out.resize(out.len().next_multiple_of(4), 0);
    out
}

/// The vertices of one run of groups, in group order: 36 bytes each, or 60 for a run of a tangent effect (the 36 bytes,
/// the first UV again as the second set, then the tangent as four floats; the layout the reader decodes).
fn vertex_bytes(run: &[GroupSource], layout: &SolidLayout) -> Vec<u8> {
    let tangent = run.first().is_some_and(|g| layout.tangent_effects.contains(&g.effect_id));
    let stride = if tangent { TANGENT_VERTEX_STRIDE } else { BASE_VERTEX_STRIDE };
    let mut out = Vec::with_capacity(run.iter().map(|g| g.vertices.len()).sum::<usize>() * stride);
    for group in run {
        for (i, v) in group.vertices.iter().enumerate() {
            push_vertex(&mut out, v);
            if tangent {
                push_tangent_tail(&mut out, v, group.tangents[i]);
            }
        }
    }
    out
}

/// Bytes of a tangent-effect vertex.
const TANGENT_VERTEX_STRIDE: usize = 60;

/// The second UV set (a copy of the first) and the tangent that follow the 36 common bytes.
fn push_tangent_tail(out: &mut Vec<u8>, v: &Vertex, tangent: [f32; 4]) {
    for f in v.uv.into_iter().chain(tangent) {
        out.extend_from_slice(&f.to_le_bytes());
    }
}

fn push_vertex(out: &mut Vec<u8>, v: &Vertex) {
    for f in v.position.iter().chain(&v.normal) {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out.extend_from_slice(&v.color_bgra);
    for f in v.uv {
        out.extend_from_slice(&f.to_le_bytes());
    }
}

/// The NUL-terminated name, padded with NULs to a multiple of 4 bytes like the stock chunks.
fn material_name(group: &GroupSource) -> Vec<u8> {
    let name = group.material_name.as_deref().unwrap_or_default().as_bytes();
    let mut b = vec![0u8; (name.len() + 1).next_multiple_of(4)];
    b[..name.len()].copy_from_slice(name);
    b
}
