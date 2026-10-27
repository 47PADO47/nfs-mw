//! A tiny bChunk builder: nested chunks written into one buffer, with alignment padding.
//!
//! Private to the writer so it cannot clash with a general `ChunkWriter` in `blackbox-chunk`.

use blackbox_chunk::{HEADER_LEN, PAD_BYTE};

/// Chunk id of a padding chunk.
const PADDING_ID: u32 = 0;

#[derive(Default)]
pub(super) struct ChunkBuf {
    pub bytes: Vec<u8>,
}

impl ChunkBuf {
    /// Open a chunk; returns the handle to pass to [`end`](Self::end). The size is patched there.
    pub fn begin(&mut self, id: u32) -> usize {
        let start = self.bytes.len();
        self.bytes.extend_from_slice(&id.to_le_bytes());
        self.bytes.extend_from_slice(&0u32.to_le_bytes());
        start
    }

    /// Close the chunk opened at `start`: its payload is everything written since.
    pub fn end(&mut self, start: usize) {
        let size = (self.bytes.len() - start - HEADER_LEN) as u32;
        self.bytes[start + 4..start + 8].copy_from_slice(&size.to_le_bytes());
    }

    /// A chunk with `payload` and no children.
    pub fn leaf(&mut self, id: u32, payload: &[u8]) {
        let start = self.begin(id);
        self.bytes.extend_from_slice(payload);
        self.end(start);
    }

    /// Pad so the next chunk header starts at a multiple of `align`: a zero-filled padding chunk (id 0), as the
    /// game's files have. A gap too small for a chunk header grows by `align`.
    pub fn align_header(&mut self, align: usize) {
        let mut gap = self.bytes.len().next_multiple_of(align) - self.bytes.len();
        if gap == 0 {
            return;
        }
        if gap < HEADER_LEN {
            gap += align;
        }
        let start = self.begin(PADDING_ID);
        self.bytes.resize(start + gap, 0);
        self.end(start);
    }

    /// Fill with `0x11` up to a multiple of `align`: the left padding of a payload that must start aligned.
    pub fn align_payload(&mut self, align: usize) {
        let target = self.bytes.len().next_multiple_of(align);
        self.bytes.resize(target, PAD_BYTE);
    }

    /// Fill with zeros up to a multiple of `align` (gaps between blobs inside a payload).
    pub fn align_zero(&mut self, align: usize) {
        let target = self.bytes.len().next_multiple_of(align);
        self.bytes.resize(target, 0);
    }
}
