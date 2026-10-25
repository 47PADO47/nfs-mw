use crate::error::{Error, Result};

/// Bytes of the fixed header; the resource table follows at [`Header::entries_offset`].
pub const HEADER_SIZE: usize = 0x28;
/// Bytes of one resource table entry in a version 5 bundle.
pub const ENTRY_SIZE: usize = 0x48;
/// Memory blocks a resource can have (main memory, graphics memory and two more the PC files do not use).
pub const BLOCKS: usize = 4;

/// Bundle flag: every block is a zlib stream.
pub const FLAG_COMPRESSED: u32 = 0x1;
/// Bundle flag: a `ResourceStringTable` (XML, optionally zlib) follows the data at [`Header::debug_offset`].
pub const FLAG_HAS_DEBUG_DATA: u32 = 0x2;

/// The fixed part of a `bnd2` file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub version: u16,
    /// 1 on PC.
    pub platform: u16,
    /// Where the debug data starts; the file size when there is none.
    pub debug_offset: u32,
    pub resource_count: u32,
    pub entries_offset: u32,
    /// Where each block's data starts; a resource's disk offsets are relative to these.
    pub block_offsets: [u32; BLOCKS],
    pub flags: u32,
}

pub(crate) fn u16_at(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

pub(crate) fn u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
}

pub(crate) fn u64_at(data: &[u8], offset: usize) -> u64 {
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    u64::from_le_bytes(bytes)
}

impl Header {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < HEADER_SIZE {
            return Err(Error::Truncated { what: "header", start: 0, end: HEADER_SIZE, len: data.len() });
        }
        let magic = [data[0], data[1], data[2], data[3]];
        if &magic != b"bnd2" {
            return Err(Error::BadMagic { magic });
        }
        let (version, platform) = (u16_at(data, 4), u16_at(data, 6));
        if version != 5 || platform != 1 {
            return Err(Error::Unsupported { version, platform });
        }
        let mut block_offsets = [0; BLOCKS];
        for (block, slot) in block_offsets.iter_mut().enumerate() {
            *slot = u32_at(data, 0x14 + 4 * block);
        }
        Ok(Self {
            version,
            platform,
            debug_offset: u32_at(data, 8),
            resource_count: u32_at(data, 0xC),
            entries_offset: u32_at(data, 0x10),
            block_offsets,
            flags: u32_at(data, 0x24),
        })
    }

    pub fn compressed(&self) -> bool {
        self.flags & FLAG_COMPRESSED != 0
    }
}
