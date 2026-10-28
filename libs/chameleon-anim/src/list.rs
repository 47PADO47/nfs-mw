//! An animation list (resource type 0xB0): all the clips of a character in one resource.

use crate::bytes::Block;
use crate::clip::Clip;
use crate::error::Result;

const COUNT_AT: usize = 2;
const TOTAL_AT: usize = 4;
/// The offset of the table of clip offsets.
const OFFSETS_AT: usize = 8;

/// The clips of an animation list, in file order.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationList {
    pub clips: Vec<Clip>,
}

impl AnimationList {
    /// Parse the main block of an animation list resource.
    pub fn parse(main: &[u8]) -> Result<Self> {
        let block = Block::new(main, "animation list");
        let count = usize::from(block.u16(COUNT_AT)?);
        let total = (block.u32(TOTAL_AT)? as usize).min(block.len());
        let offsets = block.u32(OFFSETS_AT)? as usize;
        let starts: Vec<usize> =
            (0..count).map(|n| block.u32(offsets + 4 * n).map(|at| at as usize)).collect::<Result<_>>()?;
        let mut clips = Vec::with_capacity(count);
        for (n, &start) in starts.iter().enumerate() {
            let end = starts.get(n + 1).copied().unwrap_or(total);
            clips.push(Clip::parse(block.slice(start, end.saturating_sub(start))?)?);
        }
        Ok(Self { clips })
    }

    /// The clip whose id (low word) is `id`.
    pub fn by_id(&self, id: u32) -> Option<&Clip> {
        self.clips.iter().find(|clip| clip.id == id)
    }
}
