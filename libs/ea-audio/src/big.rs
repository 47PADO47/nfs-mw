//! `.big` containers: many `SCHl` streams back to back, each starting on a 0x100 boundary, with non-stream tables
//! in between. This module finds the streams; decode them with [`crate::schl`].

use crate::error::Result;
use crate::pcm::Pcm;
use crate::schl::{StreamReader, TAG_HEADER, extent};
use crate::source::ReadAt;

/// Alignment of stream starts inside a `.big`.
pub const ALIGNMENT: u64 = 0x100;

/// Where one stream sits in the container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BigEntry {
    pub offset: u64,
    /// Length in bytes up to the end of its `SCEl` block.
    pub len: u64,
}

impl BigEntry {
    /// Decode this stream.
    pub fn decode<S: ReadAt + ?Sized>(&self, src: &S) -> Result<Pcm> {
        StreamReader::open(src, self.offset)?.read_all()
    }
}

fn is_header_at<S: ReadAt + ?Sized>(src: &S, offset: u64) -> bool {
    let mut tag = [0u8; 4];
    src.read_at(offset, &mut tag) == 4 && tag == TAG_HEADER
}

/// Find every stream. Only block prefixes are read, so scanning a 190 MB file touches a few hundred KB.
pub fn scan<S: ReadAt + ?Sized>(src: &S) -> Result<Vec<BigEntry>> {
    let total = src.len();
    let mut entries = Vec::new();
    let mut pos = 0u64;
    while pos < total {
        if !is_header_at(src, pos) {
            pos = (pos / ALIGNMENT + 1) * ALIGNMENT;
            continue;
        }
        let len = extent(src, pos)?;
        entries.push(BigEntry { offset: pos, len });
        pos = (pos + len).div_ceil(ALIGNMENT) * ALIGNMENT;
    }
    Ok(entries)
}
