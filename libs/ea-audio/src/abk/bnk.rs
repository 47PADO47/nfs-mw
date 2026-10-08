//! The `BNKl` sound list embedded in a bank: a table of offsets to `PT` headers; the sound data follows.

use crate::bytes::{u8_at, u16_le, u32_le};
use crate::error::{Error, Result};
use crate::schl::header::{self, StreamHeader};

/// One sound of a `BNKl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankSound {
    /// Index of the entry in the `BNKl` table (what sample tables refer to).
    pub index: usize,
    pub header: StreamHeader,
}

/// Parse the sound list of the `BNKl` at `base`. Dummy entries (offset 0) are skipped.
pub fn parse_sounds(data: &[u8], base: usize) -> Result<Vec<BankSound>> {
    if data.get(base..base + 4) != Some(b"BNKl") {
        return Err(Error::BadMagic { expected: "BNKl" });
    }
    let table = match u8_at(data, base + 4)? {
        2 => 0x0C,
        4 | 5 => 0x14,
        _ => return Err(Error::Unsupported("BNKl version")),
    };
    let count = u16_le(data, base + 6)? as usize;
    let mut sounds = Vec::new();
    for index in 0..count {
        let entry = base + table + 4 * index;
        let relative = u32_le(data, entry)? as usize;
        if relative == 0 {
            continue;
        }
        let at = entry.checked_add(relative).filter(|&a| a < data.len());
        let at = at.ok_or(Error::Corrupt("BNKl entry points outside the bank"))?;
        sounds.push(BankSound { index, header: header::parse(&data[at..])? });
    }
    Ok(sounds)
}
