//! Solid upload: one GPU mesh per solid, one draw range per shading group.

use blackbox_gfx::{BlendMode, DrawRange, MeshDesc, MeshHandle, RenderBackend, Shading, TextureHandle, Vertex};
use blackbox_solid::Solid;

/// Resolves a texture name hash to an uploaded texture and how to blend it.
/// Unknown textures draw white and opaque.
pub trait MaterialLookup {
    fn material(&self, texture_hash: u32) -> Option<(TextureHandle, BlendMode)>;

    /// Whether groups with this texture are drawn at all (placeholders that stand for
    /// "nothing", such as an unset decal, are not).
    fn draws(&self, _texture_hash: u32) -> bool {
        true
    }

    /// How groups that use the light material with this name hash are shaded, instead of the
    /// shading the solid is uploaded with.
    fn shading(&self, _light_material_hash: u32) -> Option<Shading> {
        None
    }
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
        .filter(|g| g.diffuse_texture(solid).is_none_or(|h| materials.draws(h)))
        .map(|g| {
            let material = g.diffuse_texture(solid).and_then(|h| materials.material(h));
            let light_material = solid.light_material_hashes.get(usize::from(g.light_material)).copied();
            let shading = light_material.and_then(|h| materials.shading(h)).unwrap_or(shading);
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
    backend: &mut dyn RenderBackend,
    solid: &Solid,
    materials: &impl MaterialLookup,
    shading: Shading,
) -> Option<MeshHandle> {
    if solid.vertices.is_empty() || solid.indices.is_empty() {
        return None;
    }
    let (vertices, draws) = solid_mesh(solid, materials, shading);
    Some(backend.create_mesh(&MeshDesc { label: &solid.name, vertices: &vertices, indices: &solid.indices, draws }))
}

#[cfg(test)]
mod tests {
    use blackbox_solid::{ShadingGroup, Solid, Vertex as SolidVertex};

    use super::*;

    fn group(light_material: u8) -> ShadingGroup {
        ShadingGroup {
            bounds_min: [0.0; 3],
            bounds_max: [0.0; 3],
            texture_slots: [0xFF; 5],
            light_material,
            effect_id: 4,
            flags: 0,
            num_vertices: 3,
            num_triangles: 1,
            first_index: 0,
            num_indices: 3,
            base_vertex: 0,
            vertex_buffer: 0,
        }
    }

    fn solid() -> Solid {
        Solid {
            name: "TEST".into(),
            name_hash: 0,
            version: 0,
            flags: 0,
            bounds_min: [0.0; 3],
            bounds_max: [0.0; 3],
            transform: [0.0; 16],
            num_polys: 0,
            density: 0.0,
            texture_hashes: Vec::new(),
            light_material_hashes: vec![0xAAAA, 0xBBBB],
            markers: Vec::new(),
            vertex_buffers: Vec::new(),
            vertices: vec![SolidVertex {
                position: [0.0; 3],
                normal: [0.0, 0.0, 1.0],
                color_bgra: [0; 4],
                uv: [0.0; 2],
            }],
            indices: vec![0, 0, 0],
            groups: vec![group(0), group(1), group(0xFF)],
        }
    }

    struct Shiny;

    impl MaterialLookup for Shiny {
        fn material(&self, _texture_hash: u32) -> Option<(TextureHandle, BlendMode)> {
            None
        }

        fn shading(&self, light_material_hash: u32) -> Option<Shading> {
            (light_material_hash == 0xBBBB).then_some(Shading::Sky)
        }
    }

    #[test]
    fn groups_take_the_shading_of_their_light_material() {
        let (_, draws) = solid_mesh(&solid(), &Shiny, Shading::Lit);
        let shadings: Vec<Shading> = draws.iter().map(|d| d.shading).collect();
        assert_eq!(shadings, [Shading::Lit, Shading::Sky, Shading::Lit]);
    }
}
