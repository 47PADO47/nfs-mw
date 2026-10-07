//! EA compression codecs found in EA Black Box games (Need for Speed
//! Underground 2, Most Wanted, Carbon, …): JDLZ, HUFF and the stored `RAWW`
//! wrapper.
//!
//! Every wrapper starts with a 16-byte header: a 4-byte magic, a version byte,
//! a header-size byte (0x10), the decompressed size and the compressed size.
//! Specs: `docs/formats/bchunk.md` §2–§4 (JDLZ, RAWW) and `docs/formats/huff.md`.
//!
//! This crate is game-agnostic and never touches the filesystem: it maps bytes
//! to bytes.

mod error;
mod header;
mod huff;
mod jdlz;
mod wrapper;

pub use error::{Error, Result};
pub use header::{HEADER_LEN, HUFF_MAGIC, Header, JDLZ_MAGIC, RAWW_MAGIC};
pub use huff::decompress as huff_decompress;
pub use jdlz::decompress as jdlz_decompress;
pub use wrapper::{Wrapper, unwrap};
