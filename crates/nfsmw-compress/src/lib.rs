//! Compression wrappers used by NFS: Most Wanted data files.
//!
//! Every wrapper starts with a 16-byte header: a 4-byte magic, 4 bytes of
//! version/flags, the decompressed size and the compressed size (header
//! included). See `docs/formats/bchunk.md` §2–§4 and `docs/formats/huff.md`.
//!
//! This crate never touches the filesystem: it maps bytes to bytes.

mod huff;
mod jdlz;

pub use huff::decompress as huff_decompress;
pub use jdlz::decompress as jdlz_decompress;

use std::borrow::Cow;

/// Size of the common wrapper header.
pub const HEADER_LEN: usize = 16;

/// `b"JDLZ"` read as a little-endian `u32`.
pub const JDLZ_MAGIC: u32 = u32::from_le_bytes(*b"JDLZ");
/// `b"HUFF"` read as a little-endian `u32`.
pub const HUFF_MAGIC: u32 = u32::from_le_bytes(*b"HUFF");
/// `b"RAWW"` read as a little-endian `u32`.
pub const RAWW_MAGIC: u32 = u32::from_le_bytes(*b"RAWW");

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("input too short for a {0} header")]
    Truncated(&'static str),
    #[error("bad {kind} header: {detail}")]
    BadHeader { kind: &'static str, detail: String },
    #[error("corrupt {kind} stream at input 0x{input:X} / output 0x{output:X}: {detail}")]
    Corrupt { kind: &'static str, input: usize, output: usize, detail: &'static str },
}

pub type Result<T> = std::result::Result<T, Error>;

/// Which wrapper (if any) a buffer starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrapper {
    Jdlz,
    Huff,
    Raww,
    None,
}

impl Wrapper {
    pub fn detect(data: &[u8]) -> Self {
        match data.first_chunk::<4>().map(|m| u32::from_le_bytes(*m)) {
            Some(JDLZ_MAGIC) => Self::Jdlz,
            Some(HUFF_MAGIC) => Self::Huff,
            Some(RAWW_MAGIC) => Self::Raww,
            _ => Self::None,
        }
    }
}

/// Header fields shared by all wrappers.
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

/// Decompress any supported wrapper. Returns the input unchanged (borrowed) when there is none.
pub fn unwrap(data: &[u8]) -> Result<Cow<'_, [u8]>> {
    match Wrapper::detect(data) {
        Wrapper::Jdlz => jdlz_decompress(data).map(Cow::Owned),
        Wrapper::Huff => huff_decompress(data).map(Cow::Owned),
        Wrapper::Raww => {
            let h = Header::parse(data, "RAWW")?;
            let end = HEADER_LEN + h.decompressed_size as usize;
            data.get(HEADER_LEN..end).map(Cow::Borrowed).ok_or(Error::Truncated("RAWW"))
        }
        Wrapper::None => Ok(Cow::Borrowed(data)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_and_unwrap_raww() {
        let mut blob = b"RAWW".to_vec();
        blob.extend_from_slice(&[1, 0x10, 0, 0]);
        blob.extend_from_slice(&3u32.to_le_bytes());
        blob.extend_from_slice(&19u32.to_le_bytes());
        blob.extend_from_slice(b"abc");
        assert_eq!(Wrapper::detect(&blob), Wrapper::Raww);
        assert_eq!(&*unwrap(&blob).unwrap(), b"abc");
    }

    #[test]
    fn no_wrapper_is_borrowed() {
        let data = [0u8; 12];
        assert!(matches!(unwrap(&data).unwrap(), Cow::Borrowed(_)));
    }
}
