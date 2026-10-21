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
    /// The part's triangles grouped by the shading group they came from, in index order. Each run is one draw
    /// range, so the part can be drawn with the textures and shading of its groups.
    pub runs: Vec<PartRun>,
}

/// A run of a part's triangles that came from one shading group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartRun {
    /// The index of the shading group in [`Solid::groups`].
    pub group: usize,
    /// The first index of the run in [`RigidPart::indices`].
    pub first_index: u32,
    /// The number of indices in the run (a multiple of three).
    pub index_count: u32,
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
    for (group_index, group) in solid.groups.iter().enumerate() {
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
            part.push(group_index, local);
        }
    }
    parts.into_iter().map(|(bone, part)| part.finish(bone)).collect()
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
    runs: Vec<PartRun>,
    /// Solid vertex index to index in [`PartBuilder::vertices`].
    local: BTreeMap<u32, u32>,
}

impl PartBuilder {
    /// Appends one triangle, given its corners in the part's vertices, from shading group `group`. A triangle of the
    /// same group as the last one extends that run.
    fn push(&mut self, group: usize, corners: [u32; 3]) {
        let first_index = self.indices.len() as u32;
        self.indices.extend(corners);
        match self.runs.last_mut() {
            Some(run) if run.group == group => run.index_count += 3,
            _ => self.runs.push(PartRun { group, first_index, index_count: 3 }),
        }
    }

    fn finish(self, bone: u8) -> RigidPart {
        RigidPart { bone, vertices: self.vertices, indices: self.indices, runs: self.runs }
    }

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
