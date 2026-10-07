//! The 16-byte header shared by all wrappers.

use crate::{Error, Result};

/// Size of the common wrapper header.
pub const HEADER_LEN: usize = 16;

/// `b"JDLZ"` read as a little-endian `u32`.
pub const JDLZ_MAGIC: u32 = u32::from_le_bytes(*b"JDLZ");
/// `b"HUFF"` read as a little-endian `u32`.
pub const HUFF_MAGIC: u32 = u32::from_le_bytes(*b"HUFF");
/// `b"RAWW"` read as a little-endian `u32`.
pub const RAWW_MAGIC: u32 = u32::from_le_bytes(*b"RAWW");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub magic: u32,
    /// Byte 4: format version (2 for JDLZ, 1 for HUFF).
    pub version: u8,
    /// Byte 5: always 0x10. The decomp's `LZHeader` calls it `HeaderSize`.
    pub flags: u8,
    pub decompressed_size: u32,
    /// The u32 at 0x0C. JDLZ counts the 16-byte header; HUFF does **not**
    /// (see `docs/formats/huff.md`).
    pub compressed_size: u32,
}

impl Header {
    pub fn parse(data: &[u8], kind: &'static str) -> Result<Self> {
        if data.len() < HEADER_LEN {
            return Err(Error::Truncated(kind));
        }
        let u32_at = |o: usize| u32::from_le_bytes(data[o..o + 4].try_into().unwrap());
        Ok(Self {
            magic: u32_at(0),
            version: data[4],
            flags: data[5],
            decompressed_size: u32_at(8),
            compressed_size: u32_at(12),
        })
    }
}
