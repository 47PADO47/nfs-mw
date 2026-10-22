//! Solid upload: one GPU mesh per solid, one draw range per shading group.

use blackbox_gfx::{BlendMode, DrawRange, MeshDesc, MeshHandle, RenderBackend, Shading, TextureHandle, Vertex};
use blackbox_solid::{RigidPart, Solid};

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

    /// A game's effect-index routing, used when no light-material override exists.
    /// Effect numbers have no renderer-wide meaning; callers provide the mapping.
    fn group_shading(&self, _group_index: usize) -> Option<Shading> {
        None
    }

    fn effect_shading(&self, _effect_id: u32) -> Option<Shading> {
        None
    }

    /// Whether groups with this texture, drawn with effect `effect_id`, keep both faces ([`DrawRange::two_sided`]).
    /// Both faces by default; a lookup that knows the texture packs answers with [`crate::Sidedness`].
    fn two_sided(&self, _texture_hash: u32, _effect_id: u32) -> bool {
        true
    }
}

impl<F: Fn(u32) -> Option<(TextureHandle, BlendMode)>> MaterialLookup for F {
    fn material(&self, texture_hash: u32) -> Option<(TextureHandle, BlendMode)> {
        self(texture_hash)
    }
}

/// The renderer vertices of a solid: the CPU half of an upload, which streamed solids get on their loader
/// thread (`nfsmw-data`'s `SectionData::solid_vertices`).
pub fn solid_vertices(solid: &Solid) -> Vec<Vertex> {
    solid.vertices.iter().map(render_vertex).collect()
}

/// One solid vertex as a renderer vertex (the common attributes only).
fn render_vertex(v: &blackbox_solid::Vertex) -> Vertex {
    Vertex { position: v.position, normal: v.normal, color_bgra: v.color_bgra, uv: v.uv }
}

/// The draw ranges of a solid, indexing the vertices of [`solid_vertices`]. Textures resolve through
/// `materials`, so this needs the renderer's handles: it runs on the render thread.
pub fn solid_mesh(solid: &Solid, materials: &impl MaterialLookup, shading: Shading) -> Vec<DrawRange> {
    solid
        .groups
        .iter()
        .enumerate()
        .filter(|(_, g)| is_drawn(solid, g, materials))
        .map(|(index, g)| group_draw(solid, index, (g.first_index, g.num_indices, g.base_vertex), materials, shading))
        .collect()
}

/// Whether a group's texture is drawn at all.
fn is_drawn(solid: &Solid, group: &blackbox_solid::ShadingGroup, materials: &impl MaterialLookup) -> bool {
    group.diffuse_texture(solid).is_none_or(|h| materials.draws(h))
}

/// The draw range of group `index` over the indices `(first, count, base vertex)` given, with the group's material
/// and shading. The shading is the first of the group's light material, its group index and its effect that gives
/// one, else `shading`.
fn group_draw(
    solid: &Solid,
    index: usize,
    (first_index, index_count, base_vertex): (u32, u32, u32),
    materials: &impl MaterialLookup,
    shading: Shading,
) -> DrawRange {
    let g = &solid.groups[index];
    let material = g.diffuse_texture(solid).and_then(|h| materials.material(h));
    let light_material = solid.light_material_hashes.get(usize::from(g.light_material)).copied();
    let shading = light_material
        .and_then(|h| materials.shading(h))
        .or_else(|| materials.group_shading(index))
        .or_else(|| materials.effect_shading(g.effect_id))
        .unwrap_or(shading);
    DrawRange {
        first_index,
        index_count,
        base_vertex: base_vertex as i32,
        texture: material.map(|(t, _)| t),
        blend: material.map_or(BlendMode::Opaque, |(_, b)| b),
        shading,
        // A group without a texture the lookup knows keeps both faces.
        two_sided: material.is_none() || g.diffuse_texture(solid).is_none_or(|h| materials.two_sided(h, g.effect_id)),
    }
}

/// The draw ranges of one rigid part, indexing the part's own vertices: one range per run of its triangles.
pub fn part_draws(
    solid: &Solid,
    part: &RigidPart,
    materials: &impl MaterialLookup,
    shading: Shading,
) -> Vec<DrawRange> {
    part.runs
        .iter()
        .filter_map(|run| {
            let group = solid.groups.get(run.group)?;
            if !is_drawn(solid, group, materials) {
                return None;
            }
            Some(group_draw(solid, run.group, (run.first_index, run.index_count, 0), materials, shading))
        })
        .collect()
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
    let vertices = solid_vertices(solid);
    let draws = solid_mesh(solid, materials, shading);
    Some(backend.create_mesh(&MeshDesc {
        label: &solid.name,
        vertices: &vertices,
        secondary_uvs: Some(&solid.secondary_uvs),
        tangents: Some(&solid.tangents),
        indices: &solid.indices,
        draws,
    }))
}

/// Upload one rigid part of a skinned solid (see `blackbox_solid::rigid_split`): its own vertices and triangles, drawn
/// with the textures and shading of the groups its triangles came from. The part's indices are its own, so a part whose
/// vertices do not fit 16-bit indices is not drawn (`None`), as are parts without triangles.
pub fn upload_rigid_part(
    backend: &mut dyn RenderBackend,
    solid: &Solid,
    part: &RigidPart,
    materials: &impl MaterialLookup,
    shading: Shading,
) -> Option<MeshHandle> {
    if part.vertices.is_empty() || part.indices.is_empty() {
        return None;
    }
    let indices = part.indices.iter().map(|&i| u16::try_from(i).ok()).collect::<Option<Vec<u16>>>()?;
    let vertices: Vec<Vertex> = part.vertices.iter().map(render_vertex).collect();
    let draws = part_draws(solid, part, materials, shading);
    Some(backend.create_mesh(&MeshDesc {
        label: &solid.name,
        vertices: &vertices,
        secondary_uvs: None,
        tangents: None,
        indices: &indices,
        draws,
    }))
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
            bone_table: None,
        }
    }

    fn solid() -> Solid {
        Solid {
            secondary_uvs: Vec::new(),
            tangents: Vec::new(),
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
            skin: None,
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
        let draws = solid_mesh(&solid(), &Shiny, Shading::Lit);
        let shadings: Vec<Shading> = draws.iter().map(|d| d.shading).collect();
        assert_eq!(shadings, [Shading::Lit, Shading::Sky, Shading::Lit]);
    }

    struct Effects;

    impl MaterialLookup for Effects {
        fn material(&self, _texture_hash: u32) -> Option<(TextureHandle, BlendMode)> {
            Some((TextureHandle::from_raw(17), BlendMode::AlphaBlend))
        }

        fn shading(&self, hash: u32) -> Option<Shading> {
            (hash == 0xBBBB).then_some(Shading::Lit)
        }

        fn effect_shading(&self, effect: u32) -> Option<Shading> {
            (effect == 22).then_some(Shading::Sky)
        }

        fn two_sided(&self, texture_hash: u32, effect: u32) -> bool {
            texture_hash != 42 || effect != 22
        }
    }

    #[test]
    fn effect_routing_is_per_group_and_preserves_texture_blend_and_material_precedence() {
        let mut solid = solid();
        solid.texture_hashes.push(42);
        for group in &mut solid.groups {
            group.texture_slots[0] = 0;
            group.effect_id = 22;
        }
        solid.groups[2].effect_id = 99;
        let draws = solid_mesh(&solid, &Effects, Shading::Prelit);
        assert_eq!(draws.iter().map(|d| d.shading).collect::<Vec<_>>(), [Shading::Sky, Shading::Lit, Shading::Prelit]);
        assert!(draws.iter().all(|d| d.texture == Some(TextureHandle::from_raw(17))));
        assert!(draws.iter().all(|d| d.blend == BlendMode::AlphaBlend));
        assert_eq!(draws.iter().map(|d| d.two_sided).collect::<Vec<_>>(), [false, false, true]);
    }

    #[test]
    fn groups_draw_both_faces_unless_the_lookup_knows_their_texture_is_single_sided() {
        let mut solid = solid();
        solid.texture_hashes.push(42);
        solid.groups[0].texture_slots[0] = 0;
        let draws = solid_mesh(&solid, &Shiny, Shading::Lit);
        assert!(draws.iter().all(|d| d.two_sided), "the default lookup and untextured groups keep both faces");
    }
}
