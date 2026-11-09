//! Reading a tree model of the 2012 world: its levels of detail, each a set of meshes with the texture of its material.
//!
//! A tree level is two kinds of mesh, told apart by the technique of the material's shader (`docs/formats/chameleon-vegetation.md`):
//! **foliage** (`Tree_*`, and the `*_1Bit_*` techniques of the cross-card levels) is cards of leaves with a cut-out texture, **bark**
//! (`World_*`, `WorldPBR_*`) is the trunk and branches. The `WorldShadow_*` meshes are stand-ins for the shadow pass and are left
//! out. The positions stay in the 2012 engine's space (x left, y up, z forward, metres); only the vertices a triangle uses are kept.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use anyhow::{Context, Result};
use chameleon_bundle::{Pool, Resource, ResourceKind};
use chameleon_gfx::{Material, Model, Renderable, Shader, Texture, VertexDescriptor};

/// What a mesh of a tree is drawn as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Cards of leaves: a texture with a cut-out alpha, drawn from both sides.
    Foliage,
    /// A trunk or a branch: an opaque texture.
    Bark,
}

/// The role of a mesh whose material's shader has technique `technique`; `None` for a mesh that is not drawn (the shadow
/// stand-ins) or has no technique.
pub fn classify(technique: Option<&str>) -> Option<Role> {
    let technique = technique?;
    if technique.starts_with("WorldShadow") {
        return None;
    }
    if technique.starts_with("Tree_") || technique.contains("1Bit") {
        return Some(Role::Foliage);
    }
    Some(Role::Bark)
}

/// One mesh of a level: its vertices and triangles, and the id of its texture.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub role: Role,
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub triangles: Vec<[u32; 3]>,
    /// The id of the texture resource (the diffuse sampler's).
    pub texture: u64,
}

/// One level of detail.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Level {
    pub parts: Vec<Part>,
}

/// A tree: its levels, finest first, and the textures they use by resource id.
pub struct TreeModel {
    pub levels: Vec<Level>,
    pub textures: HashMap<u64, Texture>,
}

fn expect(pool: &Pool, id: u64, kind: ResourceKind) -> Result<Resource> {
    let resource = pool.resource(id).with_context(|| format!("resource {id:#x}"))?;
    anyhow::ensure!(
        resource.entry.kind == kind.id(),
        "resource {id:#x} is of type {:#x}, not {:#x}",
        resource.entry.kind,
        kind.id()
    );
    Ok(resource)
}

/// Read the model `model` out of `pool`. Fails when a resource it needs is not in the pool or does not decode.
pub fn read_model(pool: &Pool, model: u64) -> Result<TreeModel> {
    read_model_with(pool, model, classify)
}

/// The same with another rule for which meshes are drawn and how (the signs, `signs/`, have their own).
pub fn read_model_with(pool: &Pool, model: u64, classify: fn(Option<&str>) -> Option<Role>) -> Result<TreeModel> {
    let resource = expect(pool, model, ResourceKind::Model)?;
    let model = Model::parse(&resource.blocks[0], &resource.imports)?;
    let mut textures = HashMap::new();
    let mut levels = Vec::with_capacity(model.lods.len());
    for &lod in &model.lods {
        levels.push(read_level(pool, lod, &mut textures, classify)?);
    }
    Ok(TreeModel { levels, textures })
}

fn read_level(
    pool: &Pool,
    lod: u64,
    textures: &mut HashMap<u64, Texture>,
    classify: fn(Option<&str>) -> Option<Role>,
) -> Result<Level> {
    let resource = expect(pool, lod, ResourceKind::Renderable)?;
    let renderable = Renderable::parse(&resource.blocks[0], &resource.imports)?;
    let is_descriptor = |id: u64| pool.kind(id) == Some(ResourceKind::VertexDescriptor.id());
    let graphics = &resource.blocks[1];
    let mut parts = Vec::new();
    for mesh in &renderable.meshes {
        let material = expect(pool, mesh.material, ResourceKind::Material)?;
        let material = Material::parse(&material.blocks[0], &material.imports)?;
        let shader = expect(pool, material.shader, ResourceKind::Shader)?;
        let shader = Shader::parse(&shader.blocks[0], &shader.imports, is_descriptor)?;
        let Some(role) = classify(shader.technique()) else { continue };
        let Some(descriptor) = shader.vertex_descriptor else { continue };
        let slot = shader.sampler_slot("DiffuseTextureSampler").unwrap_or(0);
        let Some(&texture) = material.textures.get(slot) else { continue };
        if let Entry::Vacant(slot) = textures.entry(texture) {
            let picture = expect(pool, texture, ResourceKind::Texture)?;
            slot.insert(Texture::parse(&picture.blocks[0], &picture.blocks[1])?);
        }
        let descriptor = expect(pool, descriptor, ResourceKind::VertexDescriptor)?;
        let descriptor = VertexDescriptor::parse(&descriptor.blocks[0])?;
        let vertices = descriptor.decode(mesh.vertex_bytes(graphics)?)?;
        let triangles = mesh.triangles(graphics)?;
        parts.push(compact(role, texture, &vertices, &triangles));
    }
    Ok(Level { parts })
}

/// A part of the vertices `triangles` use, in the order they first appear, with the triangles renumbered. A triangle that names a
/// vertex the buffer does not have is dropped (the buffer's tail is padding).
fn compact(role: Role, texture: u64, vertices: &[chameleon_gfx::Vertex], triangles: &[[u32; 3]]) -> Part {
    let mut renumber: HashMap<u32, u32> = HashMap::new();
    let mut part = Part { role, positions: Vec::new(), uvs: Vec::new(), triangles: Vec::new(), texture };
    for triangle in triangles {
        if triangle.iter().any(|&index| index as usize >= vertices.len()) {
            continue;
        }
        let mut renumbered = [0; 3];
        for (slot, &index) in renumbered.iter_mut().zip(triangle) {
            *slot = *renumber.entry(index).or_insert_with(|| {
                let vertex = &vertices[index as usize];
                part.positions.push(vertex.position);
                part.uvs.push(vertex.uv);
                (part.positions.len() - 1) as u32
            });
        }
        part.triangles.push(renumbered);
    }
    part
}

#[cfg(test)]
mod tests {
    use super::*;
    use chameleon_gfx::Vertex;

    fn vertex(x: f32) -> Vertex {
        Vertex { position: [x, 0.0, 0.0], color: [255; 4], uv: [x, 0.0] }
    }

    #[test]
    fn foliage_bark_and_the_shadow_stand_ins_are_told_apart_by_the_technique() {
        assert_eq!(classify(Some("Tree_Translucent_1Bit_Doublesided_InstanceTint")), Some(Role::Foliage));
        assert_eq!(classify(Some("Diffuse_1Bit_Doublesided")), Some(Role::Foliage));
        assert_eq!(classify(Some("World_Diffuse_Specular_Singlesided")), Some(Role::Bark));
        assert_eq!(classify(Some("WorldPBR_Diffuse_Normal_Specular_Reflective_AO_Singlesided")), Some(Role::Bark));
        assert_eq!(classify(Some("WorldShadow_Opaque_Singlesided")), None);
        assert_eq!(classify(None), None);
    }

    #[test]
    fn a_part_keeps_only_the_vertices_its_triangles_use_in_first_use_order() {
        let vertices: Vec<Vertex> = (0..6).map(|n| vertex(n as f32)).collect();
        let part = compact(Role::Bark, 7, &vertices, &[[4, 2, 0], [2, 4, 5]]);
        assert_eq!(part.positions.iter().map(|p| p[0]).collect::<Vec<_>>(), [4.0, 2.0, 0.0, 5.0]);
        assert_eq!(part.triangles, [[0, 1, 2], [1, 0, 3]]);
        assert_eq!((part.texture, part.role), (7, Role::Bark));
    }

    #[test]
    fn a_triangle_past_the_end_of_the_buffer_is_dropped() {
        let vertices: Vec<Vertex> = (0..3).map(|n| vertex(n as f32)).collect();
        let part = compact(Role::Foliage, 1, &vertices, &[[0, 1, 2], [0, 1, 9]]);
        assert_eq!(part.triangles.len(), 1);
        assert_eq!(part.positions.len(), 3);
    }
}
