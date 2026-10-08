//! Language string tables of EA Black Box games. Spec: `docs/formats/text.md`.
//!
//! The chunk `0x00039000` holds, after a 0x20-byte header, a histogram table (`i32 count`, `u16[3072]`), a
//! table of `{u32 label hash, u32 offset}` records sorted by hash, and the packed strings. Packed strings are
//! NUL-terminated bytes: below 0x80 a character, otherwise an index into the histogram (a small value there
//! is a prefix that selects a second-level entry with the next byte).

use blackbox_chunk::find;
use thiserror::Error;

/// The chunk id of a language table.
pub const LANGUAGE_CHUNK: u32 = 0x0003_9000;

/// The key of a label: the Black Box string hash of its text (case sensitive).
pub fn label_hash(label: &str) -> u32 {
    blackbox_hash::bstring_hash(label)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("the file has no language chunk (0x00039000)")]
    NoLanguageChunk,
    #[error("the language table is truncated or inconsistent: {0}")]
    Malformed(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;

const HISTOGRAM_ENTRIES: usize = 3072;

/// A parsed table. Strings are unpacked on demand.
#[derive(Clone, Debug)]
pub struct StringTable {
    histogram: Vec<u16>,
    /// `(label hash, byte offset into `strings`)`, sorted by hash.
    records: Vec<(u32, u32)>,
    strings: Vec<u8>,
}

fn le32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

impl StringTable {
    /// Finds the language chunk in a whole `.bin` file and parses it.
    pub fn from_file(file: &[u8]) -> Result<Self> {
        let chunk = find(file, LANGUAGE_CHUNK).ok_or(Error::NoLanguageChunk)?;
        Self::parse(chunk.payload)
    }

    /// Parses the payload of a `0x00039000` chunk.
    pub fn parse(payload: &[u8]) -> Result<Self> {
        let bad = Error::Malformed("header");
        let hist_pos = le32(payload, 0).ok_or(Error::Malformed("header"))? as usize;
        let count = le32(payload, 4).ok_or(Error::Malformed("header"))? as usize;
        let rec_pos = le32(payload, 8).ok_or(Error::Malformed("header"))? as usize;
        let str_pos = le32(payload, 12).ok_or(Error::Malformed("header"))? as usize;
        if str_pos > payload.len() || count > payload.len() / 8 {
            return Err(bad);
        }
        let hist_bytes =
            payload.get(hist_pos + 4..hist_pos + 4 + HISTOGRAM_ENTRIES * 2).ok_or(Error::Malformed("histogram"))?;
        let histogram = hist_bytes.as_chunks::<2>().0.iter().map(|b| u16::from_le_bytes(*b)).collect();
        let rec_bytes = payload.get(rec_pos..rec_pos + count * 8).ok_or(Error::Malformed("records"))?;
        let records = rec_bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|r| (u32::from_le_bytes([r[0], r[1], r[2], r[3]]), u32::from_le_bytes([r[4], r[5], r[6], r[7]])))
            .collect();
        Ok(Self { histogram, records, strings: payload[str_pos..].to_vec() })
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The text for a label hash, unpacked.
    pub fn get(&self, label: u32) -> Option<String> {
        let i = self.records.binary_search_by_key(&label, |r| r.0).ok()?;
        Some(self.unpack(self.records[i].1 as usize))
    }

    /// Every label hash in the table.
    pub fn labels(&self) -> impl Iterator<Item = u32> + '_ {
        self.records.iter().map(|r| r.0)
    }

    fn unpack(&self, offset: usize) -> String {
        let bytes = self.strings.get(offset..).unwrap_or_default();
        let mut units: Vec<u16> = Vec::new();
        let mut i = 0;
        while let Some(&b) = bytes.get(i) {
            i += 1;
            if b == 0 {
                break;
            }
            if b < 0x80 {
                units.push(b as u16);
                continue;
            }
            let first = self.histogram.get(b as usize).copied().unwrap_or(0);
            let value = if (1..0x80).contains(&first) {
                // A prefix: the next byte picks the entry in the second-level block.
                let next = bytes.get(i).copied().unwrap_or(0);
                i += 1;
                let at = first as usize * 0x80 + next as usize;
                self.histogram.get(at.wrapping_sub(0x80)).copied().unwrap_or(0)
            } else {
                first
            };
            units.push(if value == 0 { b'_' as u16 } else { value });
        }
        String::from_utf16_lossy(&units)
    }
}

#[cfg(test)]
mod tests;
