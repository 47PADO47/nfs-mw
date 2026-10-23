//! The mesh: shading groups, indices and vertex buffers.
//!
//! A solid has one vertex buffer per run of consecutive shading groups with the
//! same effect id; the groups of a run share the buffer in order, and their
//! indices are relative to the buffer's start. Verified on every car and world
//! solid of NFS: MW (`docs/formats/models.md`).

use blackbox_chunk::{Chunk, ids};

use super::skin::read_skin;
use crate::bytes::{f32_at, u32_at, vec3_at};
use crate::layout::SolidLayout;
use crate::{BASE_VERTEX_STRIDE, ShadingGroup, SkinVertex, Solid, Vertex, VertexBuffer};

/// The skin of an unskinned vertex in a solid that has skinned buffers: the root, rigidly.
const ROOT_SKIN: SkinVertex = SkinVertex { weights: [1.0, 0.0, 0.0], bones: [0; 3] };

pub(super) fn read_mesh(mesh: Chunk<'_>, layout: &SolidLayout, solid: &mut Solid) -> Result<(), String> {
    if let Some(groups) = mesh.child(ids::MESH_SHADING_GROUPS) {
        let groups = groups.aligned_payload(layout.groups_align);
        let len = layout.group.len;
        if groups.len() % len != 0 {
            return Err(format!("shading groups: {} bytes is not a multiple of {len}", groups.len()));
        }
        solid.groups = groups.chunks_exact(len).map(|r| read_group(r, layout)).collect();
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
    read_vertex_buffers(&buffers, layout, solid)?;
    check_index_ranges(solid)
}

fn read_group(r: &[u8], layout: &SolidLayout) -> ShadingGroup {
    let l = &layout.group;
    let t = l.texture_slots;
    let effect_id = u32_at(r, l.effect_id);
    let bone_table: Option<[u8; 16]> = match layout.skinned_effects.contains(&effect_id) {
        true => Some(std::array::from_fn(|i| r[l.bone_table + i])),
        false => None,
    };
    ShadingGroup {
        bounds_min: vec3_at(r, l.bounds_min),
        bounds_max: vec3_at(r, l.bounds_max),
        texture_slots: [r[t], r[t + 1], r[t + 2], r[t + 3], r[t + 4]],
        light_material: r[t + 5],
        effect_id,
        flags: u32_at(r, l.flags),
        num_vertices: u32_at(r, l.num_vertices),
        num_triangles: u32_at(r, l.num_triangles),
        first_index: u32_at(r, l.first_index),
        num_indices: u32_at(r, l.num_indices),
        base_vertex: 0,
        vertex_buffer: 0,
        bone_table,
    }
}

/// One vertex buffer of a solid, checked: the effect run it serves, its vertex count and its stride.
struct BufferPlan<'a> {
    run: std::ops::Range<usize>,
    data: &'a [u8],
    count: usize,
    stride: usize,
}

/// Assign groups to buffers (one buffer per run of equal effect ids), decode every buffer.
fn read_vertex_buffers(buffers: &[&[u8]], layout: &SolidLayout, solid: &mut Solid) -> Result<(), String> {
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
    let mut plans = Vec::with_capacity(runs.len());
    for (index, (run, data)) in runs.into_iter().zip(buffers).enumerate() {
        let count: u32 = solid.groups[run.clone()].iter().map(|g| g.num_vertices).sum();
        if count == 0 || data.len() % count as usize != 0 {
            return Err(format!("vertex buffer {index} of {} bytes does not hold {count} vertices", data.len()));
        }
        let stride = data.len() / count as usize;
        if stride < BASE_VERTEX_STRIDE {
            return Err(format!("vertex buffer {index}: unexpected stride {stride}"));
        }
        plans.push(BufferPlan { run, data, count: count as usize, stride });
    }
    // Every count is bounded by its buffer's length, so this reserves what the data holds, once.
    let total: usize = plans.iter().map(|p| p.count).sum();
    solid.vertices.reserve_exact(total);
    solid.vertex_buffers.reserve_exact(plans.len());
    let mut skin: Option<Vec<SkinVertex>> = None;
    for (index, plan) in plans.into_iter().enumerate() {
        let BufferPlan { run, data, count, stride } = plan;
        let before = solid.vertices.len();
        let effect_id = solid.groups[run.start].effect_id;
        if layout.skinned_effects.contains(&effect_id) {
            let rows = read_skin(&solid.groups[run.clone()], data, stride)?;
            let all = skin.get_or_insert_with(|| {
                let mut all = Vec::with_capacity(total);
                all.extend(std::iter::repeat_n(ROOT_SKIN, before));
                all
            });
            all.extend(rows);
        } else if let Some(all) = skin.as_mut() {
            all.extend(std::iter::repeat_n(ROOT_SKIN, count));
        }
        let tangent = stride == 60 && layout.tangent_effects.contains(&effect_id);
        let has_secondary = matches!((stride, tangent), (44, _) | (60, true));
        let first_vertex = before as u32;
        for g in &mut solid.groups[run] {
            g.vertex_buffer = index as u32;
            g.base_vertex = first_vertex;
        }
        solid.vertices.extend(data.chunks_exact(stride).map(read_vertex));
        let appended = Appended { vertices: &solid.vertices, before, data, stride, total };
        extend_channel(
            &mut solid.secondary_uvs,
            has_secondary.then_some(|v: &[u8]| [f32_at(v, 36), f32_at(v, 40)]),
            |v| v.uv,
            &appended,
        );
        extend_channel(
            &mut solid.tangents,
            tangent.then_some(|v: &[u8]| [f32_at(v, 44), f32_at(v, 48), f32_at(v, 52), f32_at(v, 56)]),
            |_| [0.0; 4],
            &appended,
        );
        solid.vertex_buffers.push(VertexBuffer { stride, first_vertex, vertex_count: count as u32 });
    }
    solid.skin = skin;
    Ok(())
}

/// A buffer just decoded into the solid's vertices, for [`extend_channel`].
struct Appended<'a> {
    /// All the solid's vertices so far, the buffer's included.
    vertices: &'a [Vertex],
    /// Index of the buffer's first vertex in `vertices`.
    before: usize,
    data: &'a [u8],
    stride: usize,
    /// Vertex count of the whole solid, for the channel's capacity.
    total: usize,
}

/// Append one buffer's values to an optional per-vertex channel. `read` decodes a vertex's value when the buffer
/// has the channel; `fill` gives the value of a vertex without one. The channel only comes into being at the first
/// buffer that has it, backfilled for the vertices before it, so a solid with no such buffer holds no channel at all.
fn extend_channel<T: Copy>(
    channel: &mut Option<Vec<T>>,
    read: Option<impl Fn(&[u8]) -> T>,
    fill: impl Fn(&Vertex) -> T,
    buffer: &Appended<'_>,
) {
    if read.is_none() && channel.is_none() {
        return;
    }
    let values = channel.get_or_insert_with(|| {
        let mut values = Vec::with_capacity(buffer.total);
        values.extend(buffer.vertices[..buffer.before].iter().map(&fill));
        values
    });
    match read {
        Some(read) => values.extend(buffer.data.chunks_exact(buffer.stride).map(read)),
        None => values.extend(buffer.vertices[buffer.before..].iter().map(&fill)),
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

fn check_index_ranges(solid: &Solid) -> Result<(), String> {
    for g in &solid.groups {
        let end = g.first_index as usize + g.num_indices as usize;
        if end > solid.indices.len() {
            return Err(format!("shading group indices end at {end}, past {}", solid.indices.len()));
        }
    }
    Ok(())
}
