use crate::header::{BLOCKS, ENTRY_SIZE, u16_at, u32_at, u64_at};

/// One row of the resource table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The resource id: a 64-bit key whose low bits are a name hash and whose top byte is a namespace (0x01 mostly).
    pub id: u64,
    /// Each block's size when inflated, with the alignment code in the top four bits (see [`Entry::block_size`]).
    pub size_and_alignment: [u32; BLOCKS],
    /// Each block's size on disk (the zlib stream length when the bundle is compressed).
    pub disk_size: [u32; BLOCKS],
    /// Each block's offset from the bundle's matching block offset.
    pub disk_offset: [u32; BLOCKS],
    /// Where the import table sits in block 0.
    pub import_offset: u32,
    /// The resource type id (see [`crate::ResourceKind`]).
    pub kind: u32,
    pub import_count: u16,
    pub flags: u16,
    pub stream_index: u32,
}

impl Entry {
    pub(crate) fn parse(table: &[u8], index: usize) -> Self {
        let base = index * ENTRY_SIZE;
        let read4 = |at: usize| std::array::from_fn(|block| u32_at(table, base + at + 4 * block));
        Self {
            id: u64_at(table, base),
            size_and_alignment: read4(0x8),
            disk_size: read4(0x18),
            disk_offset: read4(0x28),
            import_offset: u32_at(table, base + 0x38),
            kind: u32_at(table, base + 0x3C),
            import_count: u16_at(table, base + 0x40),
            flags: u16_at(table, base + 0x42),
            stream_index: u32_at(table, base + 0x44),
        }
    }

    /// Bytes of block `block` once inflated.
    pub fn block_size(&self, block: usize) -> usize {
        (self.size_and_alignment[block] & 0x0FFF_FFFF) as usize
    }

    /// The alignment the loader gives block `block` in memory, in bytes.
    pub fn block_alignment(&self, block: usize) -> usize {
        1 << (self.size_and_alignment[block] >> 28)
    }
}

/// A pointer the loader patches in a resource's main block: at `offset` it stores the address of resource `id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Import {
    pub id: u64,
    /// Offset in block 0 of the 32-bit slot that receives the pointer.
    pub offset: u32,
}
