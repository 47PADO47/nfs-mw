//! Solid upload: one GPU mesh per solid, one draw range per shading group.

use blackbox_render::{BlendMode, DrawRange, MeshDesc, MeshHandle, Renderer, Shading, TextureHandle, Vertex};
use blackbox_solid::Solid;

/// Resolves a texture name hash to an uploaded texture and how to blend it.
/// Unknown textures draw white and opaque.
pub trait MaterialLookup {
    fn material(&self, texture_hash: u32) -> Option<(TextureHandle, BlendMode)>;
}

impl<F: Fn(u32) -> Option<(TextureHandle, BlendMode)>> MaterialLookup for F {
    fn material(&self, texture_hash: u32) -> Option<(TextureHandle, BlendMode)> {
        self(texture_hash)
    }
}

/// The renderer vertices and draw ranges for a solid.
pub fn solid_mesh(solid: &Solid, materials: &impl MaterialLookup, shading: Shading) -> (Vec<Vertex>, Vec<DrawRange>) {
    let vertices = solid
        .vertices
        .iter()
        .map(|v| Vertex { position: v.position, normal: v.normal, color_bgra: v.color_bgra, uv: v.uv })
        .collect();
    let draws = solid
        .groups
        .iter()
        .map(|g| {
            let material = g.diffuse_texture(solid).and_then(|h| materials.material(h));
            DrawRange {
                first_index: g.first_index,
                index_count: g.num_indices,
                base_vertex: g.base_vertex as i32,
                texture: material.map(|(t, _)| t),
                blend: material.map_or(BlendMode::Opaque, |(_, b)| b),
                shading,
            }
        })
        .collect();
    (vertices, draws)
}

/// Upload a solid. Returns `None` for solids without geometry.
pub fn upload_solid(
    renderer: &mut Renderer,
    solid: &Solid,
    materials: &impl MaterialLookup,
    shading: Shading,
) -> Option<MeshHandle> {
    if solid.vertices.is_empty() || solid.indices.is_empty() {
        return None;
    }
    let (vertices, draws) = solid_mesh(solid, materials, shading);
    Some(renderer.create_mesh(&MeshDesc { label: &solid.name, vertices: &vertices, indices: &solid.indices, draws }))
}
