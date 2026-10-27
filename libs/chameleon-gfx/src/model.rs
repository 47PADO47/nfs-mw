use chameleon_bundle::Import;

use crate::bytes::{Block, import_at};
use crate::error::Result;

/// A model: the same object at decreasing levels of detail. Each level is a `Renderable`.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// Renderable resource ids, most detailed first.
    pub lods: Vec<u64>,
    /// Squared distance in metres at which each level takes over, as stored; the stored value after the last level is
    /// the distance it stops drawing.
    pub switch_distances: Vec<f32>,
}

impl Model {
    pub fn parse(main: &[u8], imports: &[Import]) -> Result<Self> {
        let block = Block::new(main, "model");
        let table = block.u32(0)?;
        let count = usize::from(block.u16(0x14)?);
        let mut lods = Vec::with_capacity(count);
        for n in 0..count {
            lods.push(import_at(imports, table + 4 * n as u32, "model")?);
        }
        let distances_at = block.u32(0x4)? as usize + 4;
        let switch_distances = (0..=count).map_while(|n| block.f32(distances_at + 4 * n).ok()).collect();
        Ok(Self { lods, switch_distances })
    }
}
