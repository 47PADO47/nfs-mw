//! The `VPAK` pack: a directory of vaults, each a `.vlt` + `.bin` blob pair.
//!
//! ```text
//! 0x00 char[4] "VPAK"
//! 0x04 u32     vault count
//! 0x08 u32     string-block offset (vault names, NUL-terminated)
//! 0x0C u32     string-block size
//! 0x10         count x { u32 name offset (in the string block), u32 bin size, u32 vlt size,
//!                        u32 bin offset, u32 vlt offset }
//! ```
//!
//! Offsets are relative to the start of the pack.

use crate::bytes::{cstr_at, slice, u32_at};
use crate::{Error, Result};

pub const VPAK_MAGIC: [u8; 4] = *b"VPAK";
const HEADER_LEN: usize = 0x10;
const ENTRY_LEN: usize = 20;

/// One vault's blobs, borrowed from the pack.
#[derive(Debug, Clone, Copy)]
pub struct PackEntry<'a> {
    pub name: &'a str,
    pub vlt: &'a [u8],
    pub bin: &'a [u8],
}

pub fn is_pack(data: &[u8]) -> bool {
    data.starts_with(&VPAK_MAGIC)
}

/// Splits a `VPAK` pack into its vaults.
pub fn read_pack(data: &[u8]) -> Result<Vec<PackEntry<'_>>> {
    if !is_pack(data) {
        return Err(Error::NotAttrib);
    }
    let header = |o: usize| u32_at(data, o).ok_or_else(|| Error::Pack("truncated header".into()));
    let count = header(4)? as usize;
    let strings = slice(data, header(8)? as usize, header(12)? as usize)
        .ok_or_else(|| Error::Pack("string block out of bounds".into()))?;
    if count > (data.len() - HEADER_LEN) / ENTRY_LEN {
        return Err(Error::Pack(format!("{count} vaults do not fit in {} bytes", data.len())));
    }
    (0..count)
        .map(|i| {
            let at = |field: usize| u32_at(data, HEADER_LEN + i * ENTRY_LEN + field * 4).unwrap_or(0) as usize;
            let (name_offset, bin_size, vlt_size, bin_offset, vlt_offset) = (at(0), at(1), at(2), at(3), at(4));
            let name = cstr_at(strings, name_offset)
                .and_then(|n| std::str::from_utf8(n).ok())
                .ok_or_else(|| Error::Pack(format!("vault {i}: bad name offset 0x{name_offset:X}")))?;
            let blob = |offset, size, kind| {
                slice(data, offset, size).ok_or_else(|| {
                    Error::Pack(format!("vault `{name}`: {kind} blob 0x{offset:X}+0x{size:X} out of bounds"))
                })
            };
            Ok(PackEntry { name, vlt: blob(vlt_offset, vlt_size, "vlt")?, bin: blob(bin_offset, bin_size, "bin")? })
        })
        .collect()
}
