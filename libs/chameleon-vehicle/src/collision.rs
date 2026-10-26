//! The collision mesh of a car: the `PolySoupList` (type 0x60) its graphics spec imports.
//! Spec: `docs/formats/chameleon-collision.md`.
//!
//! A poly soup is a few hundred triangles in chunks of at most 255 vertices. Each vertex is three `u16` over a range of
//! 1000 m (`500 / 32768` m per step), measured from the origin on the axes the chunk marks as centred (then 500 m is
//! subtracted) and from zero on the others. Positions are in the engine's vehicle space (x left, y up, z forward,
//! metres, the origin on the ground), the same as the wheel hubs.

use chameleon_bundle::{Pool, ResourceKind};

use crate::error::{Error, Result};

/// Metres per step of a stored coordinate.
const STEP: f32 = 500.0 / 32768.0;
/// Subtracted from a coordinate of an axis the chunk marks as centred.
const CENTRE: f32 = 500.0;
/// Offset of the chunk count and of the table of chunk offsets in the list header.
const COUNT_AT: usize = 0x28;
const TABLE_AT: usize = 0x20;
/// Chunk header: triangle table, vertex table, 2 bytes of size, 4 axis flags, triangle count, vertex count.
const FLAGS_AT: usize = 10;
const TRIANGLE_COUNT_AT: usize = 14;
const VERTEX_COUNT_AT: usize = 15;
const VERTEX_LEN: usize = 6;
/// A triangle is 12 bytes: 4 of surface data, then the three vertex indices, then 5 bytes of edge links.
const TRIANGLE_LEN: usize = 12;
const INDICES_AT: usize = 4;
/// The most chunks a list may have.
const MAX_CHUNKS: usize = 256;

/// The triangles of a car's collision mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct PolySoup {
    /// The vertices some triangle uses, chunk after chunk (vehicle space, x left, y up, z forward).
    pub vertices: Vec<[f32; 3]>,
    /// Indices into [`PolySoup::vertices`].
    pub triangles: Vec<[u32; 3]>,
}

impl PolySoup {
    /// Reads the main block of a `PolySoupList`. `None` for a list without chunks (one car ships so).
    ///
    /// A vertex no triangle uses is dropped: one car stores a vertex 500 m away from the body that nothing refers to.
    pub fn parse(main: &[u8]) -> Result<Option<Self>> {
        let chunks = u32_at(main, COUNT_AT)? as usize;
        if chunks == 0 {
            return Ok(None);
        }
        if chunks > MAX_CHUNKS {
            return Err(Error::Spec(format!("poly soup with {chunks} chunks")));
        }
        let table = u32_at(main, TABLE_AT)? as usize;
        let mut soup = Self { vertices: Vec::new(), triangles: Vec::new() };
        for n in 0..chunks {
            let chunk = u32_at(main, table + 4 * n)? as usize;
            soup.add_chunk(main, chunk)?;
        }
        Ok(Some(soup))
    }

    fn add_chunk(&mut self, main: &[u8], chunk: usize) -> Result<()> {
        let triangle_table = u32_at(main, chunk)? as usize;
        let vertex_table = u32_at(main, chunk + 4)? as usize;
        let flags = bytes(main, chunk + FLAGS_AT, 4)?;
        let (triangles, vertices) = (byte(main, chunk + TRIANGLE_COUNT_AT)?, byte(main, chunk + VERTEX_COUNT_AT)?);
        let mut triangle_indices = Vec::with_capacity(usize::from(triangles));
        for t in 0..usize::from(triangles) {
            let at = triangle_table + t * TRIANGLE_LEN + INDICES_AT;
            let indices = bytes(main, at, 3)?;
            if indices.iter().any(|&i| i >= vertices) {
                return Err(Error::Spec(format!("poly soup triangle {t} indexes past {vertices} vertices")));
            }
            triangle_indices.push([indices[0], indices[1], indices[2]]);
        }
        let mut slot: Vec<Option<u32>> = vec![None; usize::from(vertices)];
        for indices in triangle_indices {
            let mut triangle = [0u32; 3];
            for (corner, &index) in indices.iter().enumerate() {
                let index = usize::from(index);
                triangle[corner] = match slot[index] {
                    Some(known) => known,
                    None => {
                        let at = vertex_table + index * VERTEX_LEN;
                        let mut position = [0.0f32; 3];
                        for (axis, value) in position.iter_mut().enumerate() {
                            let centred = flags[axis] != 0;
                            *value =
                                f32::from(u16_at(main, at + 2 * axis)?) * STEP - if centred { CENTRE } else { 0.0 };
                        }
                        let new = self.vertices.len() as u32;
                        self.vertices.push(position);
                        slot[index] = Some(new);
                        new
                    }
                };
            }
            self.triangles.push(triangle);
        }
        Ok(())
    }

    /// The lowest and highest coordinate of the vertices on each axis.
    pub fn extents(&self) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        for v in &self.vertices {
            for a in 0..3 {
                min[a] = min[a].min(v[a]);
                max[a] = max[a].max(v[a]);
            }
        }
        (min, max)
    }
}

/// The collision mesh of the car whose graphics spec is `spec`, resolved through `pool`: the `PolySoupList` the spec
/// imports. `None` when the spec imports none or the list is empty.
pub fn load_poly_soup(pool: &Pool, spec: u64) -> Result<Option<PolySoup>> {
    let spec = pool.resource(spec)?;
    let kind = ResourceKind::PolySoupList.id();
    let Some(import) = spec.imports.iter().find(|import| pool.kind(import.id) == Some(kind)) else { return Ok(None) };
    let list = pool.resource(import.id)?;
    PolySoup::parse(&list.blocks[0])
}

fn bytes(block: &[u8], at: usize, len: usize) -> Result<&[u8]> {
    at.checked_add(len)
        .and_then(|end| block.get(at..end))
        .ok_or_else(|| Error::Spec(format!("poly soup block ends before 0x{:X}", at.saturating_add(len))))
}

fn byte(block: &[u8], at: usize) -> Result<u8> {
    Ok(bytes(block, at, 1)?[0])
}

fn u16_at(block: &[u8], at: usize) -> Result<u16> {
    let b = bytes(block, at, 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(block: &[u8], at: usize) -> Result<u32> {
    let b = bytes(block, at, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

#[cfg(test)]
mod tests;
