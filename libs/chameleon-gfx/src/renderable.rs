use chameleon_bundle::Import;

use crate::bytes::{Block, import_at};
use crate::error::{Error, Result};

/// How a mesh's indices form triangles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primitive {
    TriangleList,
    /// A strip; the index with every bit set restarts it.
    TriangleStrip,
}

/// One draw of a renderable: a material, an index range and a vertex range in the graphics block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mesh {
    /// The material resource.
    pub material: u64,
    pub primitive: Primitive,
    pub index_count: u32,
    /// Bytes per index (2 or 4).
    pub index_stride: u32,
    pub index_offset: u32,
    pub vertex_offset: u32,
    pub vertex_size: u32,
}

/// A set of meshes drawn together, with its bounding sphere.
#[derive(Debug, Clone, PartialEq)]
pub struct Renderable {
    /// Centre and radius, metres.
    pub bounding_sphere: [f32; 4],
    pub meshes: Vec<Mesh>,
}

const MESH_SIZE: usize = 0x60;

impl Renderable {
    pub fn parse(main: &[u8], imports: &[Import]) -> Result<Self> {
        let block = Block::new(main, "renderable");
        let bounding_sphere = block.f32s::<4>(0)?;
        let (version, count, table) = (block.u16(0x10)?, usize::from(block.u16(0x12)?), block.u32(0x14)? as usize);
        if version != 12 {
            return Err(Error::Invalid { what: "renderable", detail: format!("version {version}, expected 12") });
        }
        let mut meshes = Vec::with_capacity(count);
        for n in 0..count {
            let mesh_at = block.u32(table + 4 * n)? as usize;
            block.slice(mesh_at, MESH_SIZE)?;
            meshes.push(parse_mesh(&block, imports, mesh_at)?);
        }
        Ok(Self { bounding_sphere, meshes })
    }
}

fn parse_mesh(block: &Block, imports: &[Import], at: usize) -> Result<Mesh> {
    let primitive = match block.u32(at + 0x10)? {
        4 => Primitive::TriangleList,
        5 => Primitive::TriangleStrip,
        other => return Err(Error::Invalid { what: "renderable", detail: format!("primitive type {other}") }),
    };
    let index_info = block.u32(at + 0x28)? as usize;
    let vertex_info = block.u32(at + 0x2C)? as usize;
    Ok(Mesh {
        material: import_at(imports, (at + 0x20) as u32, "renderable mesh")?,
        primitive,
        index_count: block.u32(at + 0x1C)?,
        index_stride: block.u32(index_info + 0x14)?,
        index_offset: block.u32(index_info + 0xC)?,
        vertex_offset: block.u32(vertex_info + 0xC)?,
        vertex_size: block.u32(vertex_info + 0x10)?,
    })
}

impl Mesh {
    /// The triangles of this mesh as indices into its vertex buffer, with strips expanded and degenerates dropped.
    pub fn triangles(&self, graphics: &[u8]) -> Result<Vec<[u32; 3]>> {
        let block = Block::new(graphics, "index buffer");
        let stride = self.index_stride as usize;
        if stride != 2 && stride != 4 {
            return Err(Error::Invalid { what: "index buffer", detail: format!("{stride}-byte indices") });
        }
        let bytes = block.slice(self.index_offset as usize, self.index_count as usize * stride)?;
        let restart = if stride == 2 { 0xFFFF } else { u32::MAX };
        let indices: Vec<u32> = bytes
            .chunks_exact(stride)
            .map(|chunk| match stride {
                2 => u32::from(u16::from_le_bytes([chunk[0], chunk[1]])),
                _ => u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]),
            })
            .collect();
        let mut triangles = Vec::new();
        match self.primitive {
            Primitive::TriangleList => {
                triangles.extend(indices.as_chunks::<3>().0.iter().copied());
            }
            Primitive::TriangleStrip => strip_to_triangles(&indices, restart, &mut triangles),
        }
        triangles.retain(|[a, b, c]| a != b && b != c && a != c);
        Ok(triangles)
    }

    /// The bytes of this mesh's vertex buffer.
    pub fn vertex_bytes<'a>(&self, graphics: &'a [u8]) -> Result<&'a [u8]> {
        Block::new(graphics, "vertex buffer").slice(self.vertex_offset as usize, self.vertex_size as usize)
    }
}

fn strip_to_triangles(indices: &[u32], restart: u32, out: &mut Vec<[u32; 3]>) {
    let mut run: Vec<u32> = Vec::new();
    for &index in indices {
        if index == restart {
            run.clear();
            continue;
        }
        run.push(index);
        if run.len() < 3 {
            continue;
        }
        let n = run.len();
        let (a, b, c) = (run[n - 3], run[n - 2], run[n - 1]);
        // Every second triangle of a strip runs the other way round.
        out.push(if n % 2 == 0 { [b, a, c] } else { [a, b, c] });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_alternate_winding_and_restart() {
        let mut out = Vec::new();
        strip_to_triangles(&[0, 1, 2, 3, 0xFFFF, 4, 5, 6], 0xFFFF, &mut out);
        assert_eq!(out, vec![[0, 1, 2], [2, 1, 3], [4, 5, 6]]);
    }
}
