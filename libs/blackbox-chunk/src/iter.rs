use crate::{BARE_JDLZ_ID, Chunk, Error, HEADER_LEN, Result};

/// Iterator over a sequence of sibling chunks.
#[derive(Debug, Clone)]
pub struct Chunks<'a> {
    data: &'a [u8],
    /// Offset of `data[0]` in the root buffer.
    base: usize,
    pos: usize,
    failed: bool,
}

impl<'a> Chunks<'a> {
    pub(crate) fn new_at(data: &'a [u8], base: usize) -> Self {
        Self { data, base, pos: 0, failed: false }
    }
}

/// Top-level chunks of a buffer.
pub fn chunks(data: &[u8]) -> Chunks<'_> {
    Chunks::new_at(data, 0)
}

impl<'a> Iterator for Chunks<'a> {
    type Item = Result<Chunk<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed || self.pos >= self.data.len() {
            return None;
        }
        let rest = &self.data[self.pos..];
        let offset = self.base + self.pos;
        if rest.len() < HEADER_LEN {
            self.failed = true;
            return Some(Err(Error::Trailing(rest.len(), offset)));
        }
        let id = u32::from_le_bytes(rest[0..4].try_into().unwrap());
        let size = u32::from_le_bytes(rest[4..8].try_into().unwrap()) as usize;

        if id == BARE_JDLZ_ID && rest.len() >= 16 {
            // The 16-byte JDLZ header stands in for the chunk header; the blob's
            // extent is its compressed size (header included).
            let len = u32::from_le_bytes(rest[12..16].try_into().unwrap()) as usize;
            if len < 16 || len > rest.len() {
                self.failed = true;
                return Some(Err(Error::Overrun { id, offset, size: len, parent_end: self.base + self.data.len() }));
            }
            self.pos += len;
            return Some(Ok(Chunk { id, offset, payload: &rest[..len] }));
        }

        if size > rest.len() - HEADER_LEN {
            self.failed = true;
            return Some(Err(Error::Overrun { id, offset, size, parent_end: self.base + self.data.len() }));
        }
        self.pos += HEADER_LEN + size;
        Some(Ok(Chunk { id, offset, payload: &rest[HEADER_LEN..HEADER_LEN + size] }))
    }
}
