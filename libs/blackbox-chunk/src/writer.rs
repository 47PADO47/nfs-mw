//! Building chunk streams: the write side of the format. Spec: `docs/formats/bchunk.md`.

use crate::{CONTAINER_BIT, Error, HEADER_LEN, PAD_BYTE, Result};

/// Builds a chunk stream in memory, applying the engine's alignment rules.
///
/// Alignment is measured from the start of the writer's own buffer, which therefore has to land on an address that is
/// a multiple of the largest alignment used (0x80 for vertex buffers) wherever it ends up in a file. A stream built
/// here can be appended to a bigger writer with [`ChunkWriter::append`] when it goes at such an address.
#[derive(Debug, Default)]
pub struct ChunkWriter {
    buf: Vec<u8>,
    /// Id of the first chunk whose size did not fit the 32-bit size field.
    oversize: Option<u32>,
}

impl ChunkWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Where the next chunk header will be written.
    pub fn position(&self) -> usize {
        self.buf.len()
    }

    /// A leaf chunk. `id` must not have the container bit set.
    pub fn leaf(&mut self, id: u32, payload: &[u8]) {
        self.header(id, payload.len());
        self.buf.extend_from_slice(payload);
    }

    /// A leaf chunk whose payload starts at a multiple of `align`: `0x11` bytes are put in front of it, the way the
    /// engine's `GetAlignedData(align)` expects. `Chunk::aligned_payload(align)` skips them again.
    pub fn leaf_aligned(&mut self, id: u32, align: usize, payload: &[u8]) {
        let at = self.buf.len() + HEADER_LEN;
        let fill = at.next_multiple_of(align) - at;
        self.header(id, fill + payload.len());
        self.buf.resize(self.buf.len() + fill, PAD_BYTE);
        self.buf.extend_from_slice(payload);
    }

    /// A container chunk (the container bit is set on `id` if it is not) holding what `body` writes.
    pub fn container(&mut self, id: u32, body: impl FnOnce(&mut Self)) {
        let header_at = self.buf.len();
        self.header(id | CONTAINER_BIT, 0);
        body(self);
        let size = self.buf.len() - header_at - HEADER_LEN;
        self.set_size(header_at, id, size);
    }

    /// A padding chunk (id 0, zero payload) so that the next chunk header starts at a multiple of `align`. Does
    /// nothing when it already does. A gap too small for a chunk header is widened by a whole `align`.
    pub fn pad_to(&mut self, align: usize) {
        let mut gap = self.buf.len().next_multiple_of(align) - self.buf.len();
        if gap == 0 {
            return;
        }
        if gap < HEADER_LEN {
            gap += align;
        }
        self.header(0, gap - HEADER_LEN);
        self.buf.resize(self.buf.len() + gap - HEADER_LEN, 0);
    }

    /// Bytes that are already a chunk stream (built by another writer), copied in as they are.
    pub fn append(&mut self, stream: &[u8]) {
        self.buf.extend_from_slice(stream);
    }

    /// The stream. Fails if a chunk was too big for its 32-bit size field.
    pub fn finish(self) -> Result<Vec<u8>> {
        match self.oversize {
            Some(id) => Err(Error::TooLarge(id)),
            None => Ok(self.buf),
        }
    }

    fn header(&mut self, id: u32, size: usize) {
        self.buf.extend_from_slice(&id.to_le_bytes());
        self.buf.extend_from_slice(&[0; 4]);
        let header_at = self.buf.len() - HEADER_LEN;
        self.set_size(header_at, id, size);
    }

    fn set_size(&mut self, header_at: usize, id: u32, size: usize) {
        let Ok(size) = u32::try_from(size) else {
            self.oversize.get_or_insert(id);
            return;
        };
        self.buf[header_at + 4..header_at + HEADER_LEN].copy_from_slice(&size.to_le_bytes());
    }
}
