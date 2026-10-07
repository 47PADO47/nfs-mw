//! The bChunk container used by EA Black Box games (Need for Speed Underground 2,
//! Most Wanted, Carbon, …) for almost every data file (`.BUN`, `.BIN`, `.LZC`).
//! Spec: `docs/formats/bchunk.md`.
//!
//! A chunk is `u32 id, u32 size, u8 payload[size]`. Bit 31 of the id marks a
//! container whose payload is more chunks. Parsing is zero-copy over a byte
//! slice; this crate never touches the filesystem and never decompresses
//! (whole-file wrappers are stripped by `ea-compress` first).
//!
//! Only little-endian (PC) data is supported so far; console builds are big-endian.

mod chunk;
mod error;
pub mod ids;
mod iter;
mod search;

#[cfg(test)]
mod tests;

pub use chunk::Chunk;
pub use error::{Error, Result};
pub use iter::{Chunks, chunks};
pub use search::{find, find_all};

/// Bit 31 of a chunk id: the payload is a sequence of child chunks.
pub const CONTAINER_BIT: u32 = 0x8000_0000;
/// Size of a chunk header.
pub const HEADER_LEN: usize = 8;
/// A bare JDLZ blob standing where a chunk would be (`b"JDLZ"` as a little-endian u32).
/// See `docs/formats/bchunk.md` §4.
pub const BARE_JDLZ_ID: u32 = u32::from_le_bytes(*b"JDLZ");
/// Byte used to left-pad aligned payloads.
pub const PAD_BYTE: u8 = 0x11;
