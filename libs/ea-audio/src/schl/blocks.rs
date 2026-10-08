//! Walking the block structure of a stream: `char tag[4], u32 size (LE, header included), payload`.

use crate::error::{Error, Result};

/// Header block.
pub const TAG_HEADER: [u8; 4] = *b"SCHl";
/// Block count.
pub const TAG_COUNT: [u8; 4] = *b"SCCl";
/// Audio data.
pub const TAG_DATA: [u8; 4] = *b"SCDl";
/// End of stream.
pub const TAG_END: [u8; 4] = *b"SCEl";

/// Size of the tag + size prefix of every block.
pub const BLOCK_HEADER: usize = 8;

/// One block with its payload (the bytes after the 8-byte prefix).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block<'a> {
    pub tag: [u8; 4],
    pub payload: &'a [u8],
}

/// Decode the 8-byte prefix of a block into its tag and its total size.
pub fn parse_prefix(prefix: &[u8; BLOCK_HEADER]) -> Result<([u8; 4], usize)> {
    let tag = [prefix[0], prefix[1], prefix[2], prefix[3]];
    let size = u32::from_le_bytes([prefix[4], prefix[5], prefix[6], prefix[7]]) as usize;
    if tag == [0; 4] || tag == [0xFF; 4] {
        return Err(Error::Corrupt("stream ends without an SCEl block"));
    }
    if size < BLOCK_HEADER {
        return Err(Error::Corrupt("block smaller than its header"));
    }
    Ok((tag, size))
}

/// Iterator over the blocks of a byte slice, ending after `SCEl` (which is yielded) or at the first error.
pub struct Blocks<'a> {
    data: &'a [u8],
    pos: usize,
    done: bool,
}

impl<'a> Blocks<'a> {
    /// Walk `data`, which starts at a block boundary.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0, done: false }
    }

    /// Bytes consumed so far.
    pub fn position(&self) -> usize {
        self.pos
    }

    fn next_block(&mut self) -> Result<Block<'a>> {
        let rest = self.data.get(self.pos..).unwrap_or(&[]);
        let prefix: &[u8; BLOCK_HEADER] = rest
            .get(..BLOCK_HEADER)
            .and_then(|p| p.try_into().ok())
            .ok_or(Error::Truncated { offset: self.pos as u64, needed: BLOCK_HEADER })?;
        let (tag, size) = parse_prefix(prefix)?;
        let payload = rest.get(BLOCK_HEADER..size).ok_or(Error::Truncated { offset: self.pos as u64, needed: size })?;
        self.pos += size;
        Ok(Block { tag, payload })
    }
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Result<Block<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let block = self.next_block();
        self.done = block.as_ref().map_or(true, |b| b.tag == TAG_END);
        Some(block)
    }
}
