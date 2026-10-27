use crate::error::{Error, Result};

/// Bytes of the fixed header; the resource table follows at [`Header::entries_offset`].
pub const HEADER_SIZE: usize = 0x28;
/// Where the header's second part (default resource, stream names) ends in files that have a table at 0x70.
pub const EXTENDED_HEADER_SIZE: usize = 0x70;
/// Bytes of one resource table entry in a version 5 bundle.
pub const ENTRY_SIZE: usize = 0x48;
/// Memory blocks a resource can have (main memory, graphics memory and two more the PC files do not use).
pub const BLOCKS: usize = 4;
/// Streams a multi-stream bundle can name.
pub const STREAMS: usize = 4;
/// Bytes of one stream name in the header (NUL padded).
pub const STREAM_NAME_BYTES: usize = 15;

/// Bundle flag: every block is a zlib stream.
pub const FLAG_COMPRESSED: u32 = 0x1;
/// Bundle flag: a `ResourceStringTable` (XML, optionally zlib) follows the data at [`Header::debug_offset`].
pub const FLAG_HAS_DEBUG_DATA: u32 = 0x2;
/// Bundle flag: the loader must resolve the imports on the loading thread (names from the community tools).
pub const FLAG_NON_ASYNC_FIXUP: u32 = 0x4;
/// Bundle flag: resources belong to several named streams, see [`Header::stream_name`].
pub const FLAG_MULTISTREAM: u32 = 0x8;
/// Bundle flag: the bundle is a patch over another one (no 2012 file has it).
pub const FLAG_DELTA: u32 = 0x10;
/// Bundle flag: [`Header::default_resource`] is set.
pub const FLAG_HAS_DEFAULT_RESOURCE: u32 = 0x20;

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
    /// The id of the resource the loader substitutes for a missing one; 0 unless [`FLAG_HAS_DEFAULT_RESOURCE`].
    pub default_resource: u64,
    /// The stream of the default resource; -1 when there is none.
    pub default_stream: i32,
    stream_names: [[u8; STREAM_NAME_BYTES]; STREAMS],
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
        let mut header = Self {
            version,
            platform,
            debug_offset: u32_at(data, 8),
            resource_count: u32_at(data, 0xC),
            entries_offset: u32_at(data, 0x10),
            block_offsets,
            flags: u32_at(data, 0x24),
            default_resource: 0,
            default_stream: -1,
            stream_names: [[0; STREAM_NAME_BYTES]; STREAMS],
        };
        // Files whose table starts at 0x70 keep the default resource and the stream names between the two.
        if header.entries_offset as usize >= EXTENDED_HEADER_SIZE && data.len() >= EXTENDED_HEADER_SIZE {
            header.default_resource = u64_at(data, 0x28);
            header.default_stream = u32_at(data, 0x30) as i32;
            for (stream, name) in header.stream_names.iter_mut().enumerate() {
                let start = 0x34 + STREAM_NAME_BYTES * stream;
                name.copy_from_slice(&data[start..start + STREAM_NAME_BYTES]);
            }
        }
        Ok(header)
    }

    pub fn compressed(&self) -> bool {
        self.flags & FLAG_COMPRESSED != 0
    }

    /// Whether every bit of `mask` is set in the bundle flags.
    pub fn has_flags(&self, mask: u32) -> bool {
        self.flags & mask == mask
    }

    /// The name of stream `index` (`GRAPHICS`, `SOUND`, `PHYSICS`, `UI`, ...), `None` for an unused slot.
    pub fn stream_name(&self, index: usize) -> Option<&str> {
        let raw = self.stream_names.get(index)?;
        let end = raw.iter().position(|&byte| byte == 0).unwrap_or(raw.len());
        match std::str::from_utf8(&raw[..end]) {
            Ok("") | Err(_) => None,
            Ok(name) => Some(name),
        }
    }
}
