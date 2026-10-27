use crate::header::{BLOCKS, ENTRY_SIZE, u16_at, u32_at, u64_at};
use crate::id::ResourceId;

/// One row of the resource table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The resource id: a 64-bit key, see [`Entry::resource_id`] for its parts.
    pub id: u64,
    /// Each block's size when inflated, with the alignment code in the top four bits (see [`Entry::block_size`]).
    pub size_and_alignment: [u32; BLOCKS],
    /// Each block's size on disk (the zlib stream length when the bundle is compressed; in a plain bundle the top four
    /// bits repeat the alignment code).
    pub disk_size: [u32; BLOCKS],
    /// Each block's offset from the bundle's matching block offset.
    pub disk_offset: [u32; BLOCKS],
    /// Where the import table sits in block 0.
    pub import_offset: u32,
    /// The resource type id (see [`crate::ResourceKind`]).
    pub kind: u32,
    pub import_count: u16,
    /// Always 0 in the 2012 files.
    pub flags: u8,
    /// The stream the resource belongs to: an index into the header's stream names ([`crate::Header::stream_name`]).
    pub stream_index: u8,
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
            flags: table[base + 0x42],
            stream_index: table[base + 0x43],
        }
    }

    /// The id taken apart into key, sub-type, index and class.
    pub const fn resource_id(&self) -> ResourceId {
        ResourceId(self.id)
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

/// How the loader fills an import slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImportKind {
    /// The slot receives the address of the resource.
    Pointer,
    /// The slot receives a handle to the resource (bit 31 of the stored offset is set). Genesys objects use these for
    /// the references to their type resources.
    ResourceHandle,
}

/// Bit 31 of an import's stored offset: the slot takes a resource handle instead of a pointer.
pub const HANDLE_FLAG: u32 = 0x8000_0000;

/// A slot the loader patches in a resource's main block: at the slot it stores the address (or handle) of resource
/// `id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Import {
    pub id: u64,
    /// The offset as stored: the slot's offset in block 0, with [`HANDLE_FLAG`] set for a handle import.
    pub offset: u32,
}

impl Import {
    /// Offset in block 0 of the 32-bit slot that receives the pointer or handle.
    pub const fn slot(&self) -> u32 {
        self.offset & !HANDLE_FLAG
    }

    pub const fn kind(&self) -> ImportKind {
        match self.offset & HANDLE_FLAG {
            0 => ImportKind::Pointer,
            _ => ImportKind::ResourceHandle,
        }
    }
}
