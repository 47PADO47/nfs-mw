use crate::{BARE_JDLZ_ID, CONTAINER_BIT, Chunks, HEADER_LEN, search};

/// One chunk, borrowed from the buffer it was parsed from.
///
/// Offsets are relative to the start of that buffer, which is assumed to be
/// aligned the way the engine loads it (files and inflated blobs both start at an
/// aligned address).
#[derive(Debug, Clone, Copy)]
pub struct Chunk<'a> {
    pub id: u32,
    /// Offset of the chunk header in the root buffer.
    pub offset: usize,
    /// The payload (header excluded). For a bare JDLZ blob this is the whole blob,
    /// JDLZ header included.
    pub payload: &'a [u8],
}

impl<'a> Chunk<'a> {
    pub fn is_container(&self) -> bool {
        self.id & CONTAINER_BIT != 0 && !self.is_bare_jdlz()
    }

    pub fn is_bare_jdlz(&self) -> bool {
        self.id == BARE_JDLZ_ID
    }

    pub fn is_padding(&self) -> bool {
        self.id == 0
    }

    /// Offset of the payload in the root buffer.
    pub fn payload_offset(&self) -> usize {
        if self.is_bare_jdlz() { self.offset } else { self.offset + HEADER_LEN }
    }

    /// Offset just past this chunk in the root buffer.
    pub fn end(&self) -> usize {
        self.payload_offset() + self.payload.len()
    }

    /// The payload with its `0x11` alignment padding skipped, the way the engine's
    /// `bChunk::GetAlignedData(alignment)` does: round the payload's address up to
    /// `align`. Never strip `0x11` bytes instead: real data can start with them.
    pub fn aligned_payload(&self, align: usize) -> &'a [u8] {
        let start = self.payload_offset();
        let skip = start.next_multiple_of(align) - start;
        self.payload.get(skip..).unwrap_or(&[])
    }

    /// Child chunks of a container (empty iterator for a leaf).
    pub fn children(&self) -> Chunks<'a> {
        if self.is_container() {
            Chunks::new_at(self.payload, self.payload_offset())
        } else {
            Chunks::new_at(&[], self.end())
        }
    }

    /// Depth-first search below this chunk for the first chunk with `id`.
    pub fn find(&self, id: u32) -> Option<Chunk<'a>> {
        search::find_in(self.children(), id)
    }

    /// The direct child with `id`, if any.
    pub fn child(&self, id: u32) -> Option<Chunk<'a>> {
        self.children().filter_map(|c| c.ok()).find(|c| c.id == id)
    }
}
