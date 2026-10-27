//! Which hinged part each triangle of the body belongs to. Spec: `docs/formats/chameleon-hinged-parts.md`.
//!
//! The 2012 body is skinned: a vertex of the layouts that carry it has four bone indices (usage 13, four bytes) and four
//! weights (usage 14, four bytes). A hinged part is the piece of the body skinned to its hinge bone or to a bone below it
//! in the skeleton (the lamps of the bonnet, the glass and handle of a door), unless a deeper hinge claims the bone (the
//! mirror on a door).

use chameleon_bundle::{Pool, ResourceKind};
use chameleon_gfx::{Model, Renderable, usage};

use crate::error::Result;
use crate::hinged::HingedPart;
use crate::skeleton::Skeleton;
use crate::spec::GraphicsSpec;
use crate::vehicle::{Surface, Surfaces, expect};

/// For each level of detail of the body, each mesh of [`crate::Vehicle::body`] and each of its triangles: the index of
/// the part in `parts` that the triangle belongs to, or `None` for the rest of the body.
pub type BodyParts = Vec<Vec<Vec<Option<usize>>>>;

/// Four bone indices with their weights.
type Skin = [(u8, u8); 4];

/// Classify the triangles of the body of the vehicle whose graphics spec is `spec`. The meshes are those [`crate::load`]
/// keeps, in its order, so the result lines up with [`crate::Vehicle::body`].
pub fn body_parts(pool: &Pool, spec: u64, skeleton: &Skeleton, parts: &[HingedPart]) -> Result<BodyParts> {
    let spec = expect(pool, spec, ResourceKind::VehicleGraphicsSpec)?;
    let spec = GraphicsSpec::parse(&spec.blocks[0], &spec.imports)?;
    let model = expect(pool, spec.body_model, ResourceKind::Model)?;
    let model = Model::parse(&model.blocks[0], &model.imports)?;
    let hinges: Vec<usize> = parts.iter().map(|part| part.bone).collect();
    let owner: Vec<Option<usize>> = (0..skeleton.bones.len())
        .map(|bone| skeleton.nearest(bone, &hinges).and_then(|hinge| hinges.iter().position(|&h| h == hinge)))
        .collect();
    let surfaces = Surfaces::default();
    let mut levels = Vec::with_capacity(model.lods.len());
    for &lod in &model.lods {
        let renderable = expect(pool, lod, ResourceKind::Renderable)?;
        let parsed = Renderable::parse(&renderable.blocks[0], &renderable.imports)?;
        let mut meshes = Vec::with_capacity(parsed.meshes.len());
        for mesh in &parsed.meshes {
            let Some(surface) = surfaces.get(pool, mesh.material)? else { continue };
            let bytes = mesh.vertex_bytes(&renderable.blocks[1])?;
            let triangles = mesh.triangles(&renderable.blocks[1])?;
            let descriptor = &surface.0;
            let stride = descriptor.stride();
            let find = |wanted: u8| descriptor.elements.iter().find(|e| e.usage() == wanted && e.index() == 0);
            let (Some(bones), Some(weights)) = (find(usage::BONES), find(usage::WEIGHTS)) else {
                let classes = vec![None; triangles.len()];
                meshes.extend(split_like_lamps(descriptor, bytes, &surface, triangles, classes));
                continue;
            };
            let vertices = bytes.len() / stride.max(1);
            let skin = |vertex: usize| -> Option<Skin> {
                if vertex >= vertices {
                    return None;
                }
                let base = vertex * stride;
                Some(std::array::from_fn(|k| {
                    (bytes[base + bones.offset as usize + k], bytes[base + weights.offset as usize + k])
                }))
            };
            let classes: Vec<Option<usize>> =
                triangles.iter().map(|tri| triangle_part(tri, &skin, &owner, parts.len())).collect();
            meshes.extend(split_like_lamps(descriptor, bytes, &surface, triangles, classes));
        }
        levels.push(meshes);
    }
    Ok(levels)
}

/// The classes of one mesh as the meshes `load` makes of it: a lens (`Refraction` glass) is one mesh per connected piece and
/// circuit and the lamp interiors (`EmissiveFourChannel`) one per circuit and side besides the rest (`lamp_pieces.rs`),
/// any other mesh is one.
fn split_like_lamps(
    descriptor: &chameleon_gfx::VertexDescriptor,
    bytes: &[u8],
    surface: &Surface,
    triangles: Vec<[u32; 3]>,
    classes: Vec<Option<usize>>,
) -> Vec<Vec<Option<usize>>> {
    let Ok(vertices) = descriptor.decode(bytes) else { return vec![classes] };
    let mesh = crate::PartMesh { vertices, triangles, appearance: surface.1.clone() };
    match crate::lamp_pieces::lamp_groups(&mesh, surface.2.as_ref()) {
        Some(groups) => groups.iter().map(|group| group.triangles.iter().map(|&at| classes[at]).collect()).collect(),
        None => vec![classes],
    }
}

/// The part with the largest summed weight over the three corners, `None` when the body (every bone that no part owns)
/// outweighs every part.
fn triangle_part(
    tri: &[u32; 3],
    skin: &impl Fn(usize) -> Option<Skin>,
    owner: &[Option<usize>],
    count: usize,
) -> Option<usize> {
    let mut per_part = vec![0u32; count];
    let mut body = 0u32;
    for &vertex in tri {
        for (bone, weight) in skin(vertex as usize)? {
            match owner.get(usize::from(bone)).copied().flatten() {
                Some(part) => per_part[part] += u32::from(weight),
                None => body += u32::from(weight),
            }
        }
    }
    let (best, weight) = per_part.iter().copied().enumerate().max_by_key(|&(_, w)| w)?;
    (weight > body).then_some(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_triangle_goes_to_the_part_with_most_weight() {
        let owner = [None, Some(0), Some(1)];
        let skin = |vertex: usize| -> Option<Skin> {
            Some(match vertex {
                0 => [(1, 255), (0, 0), (0, 0), (0, 0)],
                1 => [(1, 200), (0, 55), (0, 0), (0, 0)],
                2 => [(2, 100), (1, 100), (0, 55), (0, 0)],
                _ => [(0, 255), (0, 0), (0, 0), (0, 0)],
            })
        };
        assert_eq!(triangle_part(&[0, 1, 2], &skin, &owner, 2), Some(0));
        assert_eq!(triangle_part(&[3, 3, 3], &skin, &owner, 2), None);
        assert_eq!(triangle_part(&[0, 3, 3], &skin, &owner, 2), None, "the body outweighs one corner");
    }
}
