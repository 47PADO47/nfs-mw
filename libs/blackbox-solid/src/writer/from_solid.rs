//! Turning a parsed [`Solid`] back into a source, to rewrite or edit it.

use std::collections::HashMap;

use super::error::WriteError;
use super::source::{GroupSource, SolidSource};
use crate::{BASE_VERTEX_STRIDE, Solid};

impl SolidSource {
    /// The source that writes `solid` back. The group vertices are cut out of the solid's vertex list and the indices
    /// are made relative to them again. Material names are not part of a parsed [`Solid`], so they come out as `None`.
    ///
    /// Fails for a solid this writer cannot produce: one without shading groups, a skinned one, one with a vertex buffer wider than 36 bytes (the
    /// extra channels would be lost), or one with groups whose indices leave their own vertices.
    pub fn from_solid(solid: &Solid) -> Result<Self, WriteError> {
        let unsupported = |reason| WriteError::Unsupported { solid: solid.name.clone(), reason };
        if solid.groups.is_empty() {
            return Err(unsupported("it has no shading groups"));
        }
        if solid.skin.is_some() {
            return Err(unsupported("it is skinned"));
        }
        if solid.vertex_buffers.iter().any(|b| b.stride != BASE_VERTEX_STRIDE) {
            return Err(unsupported("it has a vertex buffer wider than 36 bytes"));
        }
        // Vertices of each buffer already given to an earlier group.
        let mut used: HashMap<u32, usize> = HashMap::new();
        let mut groups = Vec::with_capacity(solid.groups.len());
        for g in &solid.groups {
            let count = g.num_vertices as usize;
            let before = used.entry(g.vertex_buffer).or_default();
            let start = g.base_vertex as usize + *before;
            *before += count;
            let first = g.first_index as usize;
            let (Some(vertices), Some(stored)) =
                (solid.vertices.get(start..start + count), solid.indices.get(first..first + g.num_indices as usize))
            else {
                return Err(unsupported("a group reaches outside the vertices or indices"));
            };
            let skipped = start - g.base_vertex as usize;
            let mut indices = Vec::with_capacity(stored.len());
            for &i in stored {
                let Some(own) = usize::from(i).checked_sub(skipped).filter(|&own| own < count) else {
                    return Err(unsupported("a group has indices outside its own vertices"));
                };
                indices.push(own as u16);
            }
            groups.push(GroupSource {
                texture_slots: g.texture_slots,
                light_material: g.light_material,
                effect_id: g.effect_id,
                flags: g.flags,
                material_name: None,
                bounds: Some((g.bounds_min, g.bounds_max)),
                vertices: vertices.to_vec(),
                tangents: Vec::new(),
                indices,
            });
        }
        Ok(Self {
            name: solid.name.clone(),
            flags: solid.flags,
            bounds_min: solid.bounds_min,
            bounds_max: solid.bounds_max,
            transform: solid.transform,
            density: Some(solid.density),
            polygon_count: Some(solid.num_polys),
            texture_hashes: solid.texture_hashes.clone(),
            light_material_hashes: solid.light_material_hashes.clone(),
            markers: solid.markers.clone(),
            groups,
        })
    }
}
