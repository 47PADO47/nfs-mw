//! A renderable of the 2012 world as triangles: float positions and the first pair of texture coordinates out of each mesh's
//! vertex buffer, strips turned into triangle lists, and big meshes cut into parts the game's 16-bit solids can hold
//! (`docs/formats/chameleon-world.md#models-meshes-and-materials`).

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chameleon_bundle::Resource;
use chameleon_gfx::{Element, Renderable, usage};

use crate::install::ShaderInfo;

/// Most vertices and most triangles of one part: a solid's indices are 16-bit and its triangle count is too.
pub const MAX_PART: usize = 60_000;
/// The vertex element formats of the world's meshes (DXGI numbers).
const FLOAT3: u32 = 6;
const HALF2: u32 = 34;
const FLOAT2: u32 = 16;

/// One mesh of a renderable, in engine space.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshData {
    /// The material resource the mesh is drawn with.
    pub material: u64,
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub triangles: Vec<[u32; 3]>,
}

/// A renderable: its bounding sphere (centre and radius, in the renderable's own space) and its meshes. A mesh whose vertex
/// layout is unknown or unreadable is left out.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderableData {
    pub sphere: [f32; 4],
    pub meshes: Vec<MeshData>,
}

/// The half-precision float `bits` as a float.
pub fn half_to_f32(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let (exponent, fraction) = (i32::from((bits >> 10) & 0x1F), f32::from(bits & 0x3FF));
    match exponent {
        0 => sign * fraction * 2f32.powi(-24),
        0x1F if fraction == 0.0 => sign * f32::INFINITY,
        0x1F => f32::NAN,
        _ => sign * (1.0 + fraction / 1024.0) * 2f32.powi(exponent - 15),
    }
}

fn f32_at(bytes: &[u8], at: usize) -> Option<f32> {
    Some(f32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

/// The positions and the first texture coordinates of a vertex buffer, one of each per vertex.
pub type Vertices = (Vec<[f32; 3]>, Vec<[f32; 2]>);

/// The positions and first texture coordinates of the vertices of a buffer with layout `elements`. `None` when the layout has
/// no float position (the world's meshes all have one) or the buffer is not a whole number of vertices.
pub fn vertices(bytes: &[u8], elements: &[Element]) -> Option<Vertices> {
    let find = |wanted: u8| elements.iter().find(|element| element.usage() == wanted && element.index() == 0);
    let position = find(usage::POSITION).filter(|element| element.format == FLOAT3)?;
    let stride = position.stride as usize;
    if stride == 0 {
        return None;
    }
    let uv = find(usage::TEXCOORD);
    let count = bytes.len() / stride;
    let mut positions = Vec::with_capacity(count);
    let mut uvs = Vec::with_capacity(count);
    for n in 0..count {
        let base = n * stride;
        let at = base + position.offset as usize;
        positions.push([f32_at(bytes, at)?, f32_at(bytes, at + 4)?, f32_at(bytes, at + 8)?]);
        let coords = match uv {
            Some(element) if element.format == HALF2 => {
                let at = base + element.offset as usize;
                [half_to_f32(u16_at(bytes, at)?), half_to_f32(u16_at(bytes, at + 2)?)]
            }
            Some(element) if element.format == FLOAT2 => {
                let at = base + element.offset as usize;
                [f32_at(bytes, at)?, f32_at(bytes, at + 4)?]
            }
            _ => [0.0, 0.0],
        };
        uvs.push(if coords.iter().all(|c| c.is_finite()) { coords } else { [0.0, 0.0] });
    }
    Some((positions, uvs))
}

/// Reads a renderable. `layout_of` gives the shader of a material (its vertex layout is what is read here); a mesh whose
/// material has none is left out.
pub fn read_renderable(
    resource: &Resource,
    mut layout_of: impl FnMut(u64) -> Option<Arc<ShaderInfo>>,
) -> Result<RenderableData> {
    let renderable = Renderable::parse(&resource.blocks[0], &resource.imports)?;
    let graphics = &resource.blocks[1];
    let mut meshes = Vec::with_capacity(renderable.meshes.len());
    for mesh in &renderable.meshes {
        let Some(shader) = layout_of(mesh.material) else { continue };
        let (Ok(bytes), Ok(triangles)) = (mesh.vertex_bytes(graphics), mesh.triangles(graphics)) else { continue };
        let Some((positions, uvs)) = vertices(bytes, &shader.elements) else { continue };
        let count = positions.len() as u32;
        let triangles: Vec<[u32; 3]> = triangles.into_iter().filter(|t| t.iter().all(|&i| i < count)).collect();
        if triangles.is_empty() {
            continue;
        }
        meshes.push(MeshData { material: mesh.material, positions, uvs, triangles });
    }
    Ok(RenderableData { sphere: renderable.bounding_sphere, meshes })
}

/// A piece of a mesh a solid can hold: its own vertices and 16-bit indices.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u16>,
}

/// The mesh as parts of at most [`MAX_PART`] vertices and triangles each, in the order of its triangles. A mesh that fits is one
/// part with its own vertices; a bigger one is cut where a part fills, each part copying the vertices its triangles use.
pub fn parts(mesh: &MeshData) -> Vec<Part> {
    if mesh.positions.len() <= MAX_PART && mesh.triangles.len() <= MAX_PART {
        let indices = mesh.triangles.iter().flatten().map(|&index| index as u16).collect();
        return vec![Part { positions: mesh.positions.clone(), uvs: mesh.uvs.clone(), indices }];
    }
    let mut out: Vec<Part> = Vec::new();
    let mut remap: HashMap<u32, u16> = HashMap::new();
    for triangle in &mesh.triangles {
        let full = out.last().is_none_or(|part| {
            let new = triangle.iter().filter(|index| !remap.contains_key(index)).count();
            part.positions.len() + new > MAX_PART || part.indices.len() / 3 >= MAX_PART
        });
        if full {
            out.push(Part { positions: Vec::new(), uvs: Vec::new(), indices: Vec::new() });
            remap.clear();
        }
        let part = out.last_mut().expect("a part was just pushed");
        for &index in triangle {
            let local = *remap.entry(index).or_insert_with(|| {
                part.positions.push(mesh.positions[index as usize]);
                part.uvs.push(mesh.uvs[index as usize]);
                (part.positions.len() - 1) as u16
            });
            part.indices.push(local);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(semantic: u32, format: u32, offset: u32, stride: u32) -> Element {
        Element { semantic, format, offset, slot: 0, stride }
    }

    #[test]
    fn halves_read_as_the_numbers_they_hold() {
        assert_eq!(half_to_f32(0x3C00), 1.0);
        assert_eq!(half_to_f32(0xC000), -2.0);
        assert_eq!(half_to_f32(0x3800), 0.5);
        assert_eq!(half_to_f32(0x0000), 0.0);
        assert_eq!(half_to_f32(0x0001), 2f32.powi(-24));
        assert!(half_to_f32(0x7C00).is_infinite());
        assert!(half_to_f32(0x7E00).is_nan());
    }

    #[test]
    fn a_world_vertex_gives_its_float_position_and_half_uv() {
        // Stride 16: position at 0, uv at 12 as two halves.
        let elements = [element(1, FLOAT3, 0, 16), element(5, HALF2, 12, 16)];
        let mut bytes = Vec::new();
        for (p, uv) in [([1.0f32, 2.0, 3.0], [0x3C00u16, 0x4000]), ([-1.5, 0.0, 9.0], [0x0000, 0x3800])] {
            for v in p {
                bytes.extend_from_slice(&v.to_le_bytes());
            }
            for h in uv {
                bytes.extend_from_slice(&h.to_le_bytes());
            }
        }
        let (positions, uvs) = vertices(&bytes, &elements).unwrap();
        assert_eq!(positions, [[1.0, 2.0, 3.0], [-1.5, 0.0, 9.0]]);
        assert_eq!(uvs, [[1.0, 2.0], [0.0, 0.5]]);
    }

    #[test]
    fn a_layout_with_no_float_position_is_refused_and_no_uv_gives_zeros() {
        assert!(vertices(&[0; 16], &[element(1, 36, 0, 8)]).is_none());
        let (_, uvs) = vertices(&[0; 24], &[element(1, FLOAT3, 0, 12)]).unwrap();
        assert_eq!(uvs, [[0.0, 0.0], [0.0, 0.0]]);
    }

    fn grid(n: usize) -> MeshData {
        // n by n quads, two triangles each, sharing vertices.
        let side = n + 1;
        let positions = (0..side * side).map(|i| [(i % side) as f32, (i / side) as f32, 0.0]).collect();
        let mut triangles = Vec::new();
        for y in 0..n {
            for x in 0..n {
                let a = (y * side + x) as u32;
                let (b, c, d) = (a + 1, a + side as u32, a + side as u32 + 1);
                triangles.extend([[a, b, d], [a, d, c]]);
            }
        }
        MeshData { material: 1, uvs: vec![[0.0, 0.0]; side * side], positions, triangles }
    }

    #[test]
    fn a_mesh_that_fits_is_one_part_with_its_own_vertices() {
        let parts = parts(&grid(4));
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].positions.len(), 25);
        assert_eq!(parts[0].indices.len(), 4 * 4 * 2 * 3);
    }

    #[test]
    fn a_mesh_that_is_too_big_is_cut_and_every_triangle_is_kept() {
        // 200 by 200 quads: 40,401 vertices and 80,000 triangles, more triangles than a part holds.
        let mesh = grid(200);
        let parts = parts(&mesh);
        assert!(parts.len() >= 2, "{} parts", parts.len());
        assert!(parts.iter().all(|p| p.positions.len() <= MAX_PART && p.indices.len() / 3 <= MAX_PART));
        let triangles: usize = parts.iter().map(|p| p.indices.len() / 3).sum();
        assert_eq!(triangles, mesh.triangles.len());
        for part in &parts {
            assert!(part.indices.iter().all(|&i| usize::from(i) < part.positions.len()));
        }
    }
}
