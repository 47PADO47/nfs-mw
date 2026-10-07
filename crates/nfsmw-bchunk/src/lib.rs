//! The bChunk container used by almost every NFS: Most Wanted data file
//! (`.BUN`, `.BIN`, `.LZC`). Spec: `docs/formats/bchunk.md`.
//!
//! A chunk is `u32 id, u32 size, u8 payload[size]` (little-endian). Bit 31 of the
//! id marks a container whose payload is more chunks. Parsing is zero-copy over a
//! byte slice; this crate never touches the filesystem and never decompresses
//! (whole-file wrappers are stripped by `nfsmw-compress` first).

pub mod ids;

/// Bit 31 of a chunk id: the payload is a sequence of child chunks.
pub const CONTAINER_BIT: u32 = 0x8000_0000;
/// Size of a chunk header.
pub const HEADER_LEN: usize = 8;
/// A bare JDLZ blob standing where a chunk would be (`b"JDLZ"` as a little-endian u32).
/// See `docs/formats/bchunk.md` §4.
pub const BARE_JDLZ_ID: u32 = u32::from_le_bytes(*b"JDLZ");
/// Byte used to left-pad aligned payloads.
pub const PAD_BYTE: u8 = 0x11;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("{0} trailing byte(s) at 0x{1:X} are too short for a chunk header")]
    Trailing(usize, usize),
    #[error("chunk 0x{id:08X} at 0x{offset:X} claims {size} bytes but its parent ends at 0x{parent_end:X}")]
    Overrun { id: u32, offset: usize, size: usize, parent_end: usize },
}

pub type Result<T> = std::result::Result<T, Error>;

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
        find_in(self.children(), id)
    }

    /// The direct child with `id`, if any.
    pub fn child(&self, id: u32) -> Option<Chunk<'a>> {
        self.children().filter_map(|c| c.ok()).find(|c| c.id == id)
    }
}

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
    fn new_at(data: &'a [u8], base: usize) -> Self {
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

fn find_in<'a>(iter: Chunks<'a>, id: u32) -> Option<Chunk<'a>> {
    for c in iter.filter_map(|c| c.ok()) {
        if c.id == id {
            return Some(c);
        }
        if c.is_container()
            && let Some(found) = find_in(c.children(), id)
        {
            return Some(found);
        }
    }
    None
}

/// Depth-first search of a whole buffer for the first chunk with `id`.
pub fn find(data: &[u8], id: u32) -> Option<Chunk<'_>> {
    find_in(chunks(data), id)
}

/// Every chunk with `id` in a buffer, depth-first, including ones nested in containers.
pub fn find_all(data: &[u8], id: u32) -> Vec<Chunk<'_>> {
    fn walk<'a>(iter: Chunks<'a>, id: u32, out: &mut Vec<Chunk<'a>>) {
        for c in iter.filter_map(|c| c.ok()) {
            if c.id == id {
                out.push(c);
            }
            if c.is_container() {
                walk(c.children(), id, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(chunks(data), id, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
        let mut v = id.to_le_bytes().to_vec();
        v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        v.extend_from_slice(payload);
        v
    }

    #[test]
    fn nested_container() {
        let inner = [chunk(0, b""), chunk(0x0013_4011, b"ABCD"), chunk(0x0013_4012, b"12345678")].concat();
        let data = chunk(0x8013_4000, &inner);
        let top: Vec<_> = chunks(&data).collect::<Result<_>>().unwrap();
        assert_eq!(top.len(), 1);
        assert!(top[0].is_container());
        let kids: Vec<_> = top[0].children().collect::<Result<_>>().unwrap();
        assert_eq!(kids.iter().map(|c| c.id).collect::<Vec<_>>(), [0, 0x0013_4011, 0x0013_4012]);
        assert!(kids[0].is_padding());
        assert_eq!(kids[1].payload, b"ABCD");
        assert_eq!(kids[1].offset, 16);
        assert_eq!(find(&data, 0x0013_4012).unwrap().payload, b"12345678");
    }

    #[test]
    fn overrun_is_error() {
        let mut data = chunk(0x0003_4600, b"abcd");
        data[4] = 99;
        assert!(matches!(chunks(&data).next(), Some(Err(Error::Overrun { .. }))));
    }

    #[test]
    fn trailing_bytes_are_error() {
        let mut data = chunk(1, b"");
        data.extend_from_slice(&[1, 2, 3]);
        let items: Vec<_> = chunks(&data).collect();
        assert!(items[0].is_ok());
        assert!(matches!(items[1], Err(Error::Trailing(3, 8))));
    }

    #[test]
    fn aligned_payload_skips_padding_by_address() {
        // Payload starts at 8; aligned to 0x10 it starts at 16, so 8 pad bytes are skipped
        // even though the real data also begins with 0x11.
        let mut payload = vec![PAD_BYTE; 8];
        payload.extend_from_slice(&[0x11, 0x22]);
        let data = chunk(0x0013_4011, &payload);
        let c = chunks(&data).next().unwrap().unwrap();
        assert_eq!(c.aligned_payload(0x10), &[0x11, 0x22]);
    }

    #[test]
    fn bare_jdlz_blob_is_one_item() {
        let mut blob = b"JDLZ".to_vec();
        blob.extend_from_slice(&[2, 0x10, 0, 0]);
        blob.extend_from_slice(&1u32.to_le_bytes());
        blob.extend_from_slice(&19u32.to_le_bytes());
        blob.extend_from_slice(&[0, 0, b'z']);
        let data = [blob.clone(), chunk(5, b"")].concat();
        let items: Vec<_> = chunks(&data).collect::<Result<_>>().unwrap();
        assert!(items[0].is_bare_jdlz());
        assert_eq!(items[0].payload, &blob[..]);
        assert_eq!(items[1].id, 5);
    }
}
