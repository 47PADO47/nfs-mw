//! The skin of a skinned vertex buffer (effect id 2): blend weights and blend slots, with each slot resolved through
//! its group's bone table. Spec: `docs/formats/models.md`, "Skinned vertices and bones".

use crate::bytes::f32_at;
use crate::{ShadingGroup, SkinVertex};

/// Bytes per vertex of a skinned buffer: the common 36 bytes, the three blend weights and the three blend slots.
const SKINNED_STRIDE: usize = 60;
/// Where the three blend weights (f32 each) start in a vertex.
const WEIGHTS_AT: usize = 36;
/// Where the three blend slots (f32 each, whole numbers) start in a vertex.
const SLOTS_AT: usize = 48;
/// Blend slots name entries of the group's bone table, which has this many.
const TABLE_LEN: usize = 16;

/// The skin of one vertex buffer of `stride`-byte vertices in `data`. `groups` are the buffer's groups in order: group
/// k holds the `num_vertices` vertices that come after those of groups 0 to k-1. Fails on a stride other than 60, on a
/// group without a bone table, or on a blend slot that is not a whole number in 0..16.
pub(super) fn read_skin(groups: &[ShadingGroup], data: &[u8], stride: usize) -> Result<Vec<SkinVertex>, String> {
    if stride != SKINNED_STRIDE {
        return Err(format!("skinned vertices of {stride} bytes, expected {SKINNED_STRIDE}"));
    }
    let mut skin = Vec::with_capacity(data.len() / stride);
    let mut rest = data;
    for group in groups {
        let Some(table) = group.bone_table else {
            return Err("skinned vertex buffer has a group without a bone table".into());
        };
        let Some((mine, after)) = rest.split_at_checked(group.num_vertices as usize * stride) else {
            return Err("skinned vertex buffer is shorter than its groups".into());
        };
        for vertex in mine.chunks_exact(stride) {
            skin.push(read_skin_vertex(vertex, &table)?);
        }
        rest = after;
    }
    Ok(skin)
}

/// One vertex: the weights as stored, and each blend slot resolved to a bone number through `table`.
fn read_skin_vertex(v: &[u8], table: &[u8; TABLE_LEN]) -> Result<SkinVertex, String> {
    let weights: [f32; 3] = std::array::from_fn(|i| f32_at(v, WEIGHTS_AT + i * 4));
    let mut bones = [0u8; 3];
    for (i, bone) in bones.iter_mut().enumerate() {
        let slot = f32_at(v, SLOTS_AT + i * 4);
        if slot.fract() != 0.0 || !(0.0..TABLE_LEN as f32).contains(&slot) {
            return Err(format!("blend slot {slot} is not a whole number in 0..{TABLE_LEN}"));
        }
        *bone = table[slot as usize];
    }
    Ok(SkinVertex { weights, bones })
}
