//! Splitting a skinned solid into one mesh per bone, so each part can be drawn with its bone's transform.

use std::collections::BTreeMap;

use crate::{SkinVertex, Solid, Vertex};

/// The triangles of a skinned solid that one bone moves: a self-contained mesh in model space (the bind pose).
///
/// The game draws the part with the transform `skeleton_world[bone] * inverse_bind[bone]`. The vertices sit at the
/// bind pose, so that transform places them in the posed skeleton.
#[derive(Debug, Clone, PartialEq)]
pub struct RigidPart {
    /// The skeleton bone number.
    pub bone: u8,
    /// The vertices the part's triangles use, copied from [`Solid::vertices`]. A vertex that two parts
    /// share is copied into both.
    pub vertices: Vec<Vertex>,
    /// Triangle list into [`RigidPart::vertices`].
    pub indices: Vec<u32>,
}

/// Splits a skinned solid by bone. Each triangle goes to the bone with the largest summed weight over its
/// three vertices. A tie goes to the lower bone number, and a triangle with no weight at all goes to bone 0.
/// Parts come in ascending bone order, and a solid without skin has none. See [`RigidPart`] for the
/// transform to draw each part with.
pub fn rigid_split(solid: &Solid) -> Vec<RigidPart> {
    let Some(skin) = solid.skin.as_deref() else {
        return Vec::new();
    };
    let mut parts: BTreeMap<u8, PartBuilder> = BTreeMap::new();
    for group in &solid.groups {
        let first = group.first_index as usize;
        let Some(indices) = solid.indices.get(first..first + group.num_indices as usize) else {
            continue;
        };
        for triangle in indices.chunks_exact(3) {
            let corners = [0, 1, 2].map(|i| group.base_vertex + u32::from(triangle[i]));
            if corners.iter().any(|&v| v as usize >= skin.len()) {
                continue;
            }
            let bone = dominant_bone(corners.map(|v| skin[v as usize]));
            let part = parts.entry(bone).or_default();
            let local = corners.map(|v| part.local(&solid.vertices, v));
            part.indices.extend(local);
        }
    }
    parts.into_iter().map(|(bone, part)| RigidPart { bone, vertices: part.vertices, indices: part.indices }).collect()
}

/// The bone with the largest summed weight over the three vertices of a triangle (0 when every weight is 0).
fn dominant_bone(corners: [SkinVertex; 3]) -> u8 {
    let mut sums = [0f32; 256];
    for corner in corners {
        for (weight, bone) in corner.weights.into_iter().zip(corner.bones) {
            sums[usize::from(bone)] += weight;
        }
    }
    let mut best = 0usize;
    for (bone, &sum) in sums.iter().enumerate() {
        if sum > sums[best] {
            best = bone;
        }
    }
    best as u8
}

/// A part under construction: its vertices are copied in on first use.
#[derive(Default)]
struct PartBuilder {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    /// Solid vertex index to index in [`PartBuilder::vertices`].
    local: BTreeMap<u32, u32>,
}

impl PartBuilder {
    /// The part's index for solid vertex `v`, copying the vertex from `source` the first time it is used.
    fn local(&mut self, source: &[Vertex], v: u32) -> u32 {
        if let Some(&index) = self.local.get(&v) {
            return index;
        }
        let index = self.vertices.len() as u32;
        self.vertices.push(source[v as usize]);
        self.local.insert(v, index);
        index
    }
}
