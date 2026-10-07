//! The chunk list of a `.vlt` blob (`DepN`, `StrN`, `DatN`, `ExpN`, `PtrN`, ...) and the `StrE`
//! string chunk at the start of a `.bin` blob.
//!
//! A chunk is `{u32 id, u32 size}` followed by its payload; the size includes the 8-byte header.
//! The id is a FourCC stored so that it reads as text when the `u32` is printed big-endian
//! (the bytes in the file are `NpeD` for `DepN`).

use crate::bytes::{cstr_list, u32_at};

const fn fourcc(id: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*id)
}

pub const DEP_N: u32 = fourcc(b"DepN");
pub const EXP_N: u32 = fourcc(b"ExpN");
pub const PTR_N: u32 = fourcc(b"PtrN");
pub const STR_E: u32 = fourcc(b"StrE");
const HEADER_LEN: usize = 8;

#[derive(Debug, Clone, Copy)]
pub struct Chunk<'a> {
    pub id: u32,
    pub payload: &'a [u8],
}

/// Walks the chunks of `data`. Stops at the end or at zero padding; a chunk whose size is
/// smaller than its header or runs past the end is an error.
pub fn chunks(data: &[u8]) -> impl Iterator<Item = Result<Chunk<'_>, String>> {
    let mut pos = 0;
    std::iter::from_fn(move || {
        let id = u32_at(data, pos)?;
        let size = u32_at(data, pos + 4)? as usize;
        if id == 0 && size == 0 {
            return None;
        }
        let start = pos;
        let payload = (size >= HEADER_LEN).then(|| data.get(start + HEADER_LEN..start + size)).flatten();
        pos = if payload.is_some() { start + size } else { data.len() };
        Some(payload.map(|payload| Chunk { id, payload }).ok_or_else(|| {
            let name = String::from_utf8_lossy(&id.to_be_bytes()).into_owned();
            format!("chunk `{name}` at vlt+0x{start:X} has a bad size 0x{size:X}")
        }))
    })
}

/// A blob a vault's pointers can be relative to (`DepN`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    /// Hash of [`Self::name`].
    pub key: u32,
    /// For example `db.vlt` and `db.bin`.
    pub name: String,
}

/// `DepN`: `u32 count`, `u32 hash[count]`, `u32 name offset[count]`, then the NUL-terminated
/// names (offsets are relative to the first name).
pub fn read_dependencies(payload: &[u8]) -> Result<Vec<Dependency>, String> {
    let bad = || "DepN chunk is truncated".to_owned();
    let count = u32_at(payload, 0).ok_or_else(bad)? as usize;
    let names_at =
        count.checked_mul(8).and_then(|n| n.checked_add(4)).filter(|&n| n <= payload.len()).ok_or_else(bad)?;
    let names = &payload[names_at..];
    (0..count)
        .map(|i| {
            let key = u32_at(payload, 4 + i * 4).ok_or_else(bad)?;
            let offset = u32_at(payload, 4 + (count + i) * 4).ok_or_else(bad)? as usize;
            let name = crate::bytes::cstr_at(names, offset).ok_or_else(bad)?;
            Ok(Dependency { key, name: String::from_utf8_lossy(name).into_owned() })
        })
        .collect()
}

/// The strings of the `StrE` chunk that starts a `.bin` blob (none if it doesn't start with one).
pub fn read_strings(bin: &[u8]) -> Vec<String> {
    match chunks(bin).next() {
        Some(Ok(chunk)) if chunk.id == STR_E => {
            cstr_list(chunk.payload).map(|s| String::from_utf8_lossy(s).into_owned()).collect()
        }
        _ => Vec::new(),
    }
}
