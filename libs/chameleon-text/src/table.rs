use crate::error::{Error, Result};

/// Bytes of the table header: language index, string count, offset of the ids, offset of the string records.
const HEADER_SIZE: usize = 16;
/// Bytes of one string record: offset of the text, length of the text in UTF-16 units including its NUL.
const RECORD_SIZE: usize = 8;
/// The resource's main block is padded to this many bytes.
const BLOCK_ALIGNMENT: usize = 16;

/// One string of the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringEntry {
    pub id: u32,
    pub text: String,
}

/// The strings of one language, sorted by id. Several entries can share an id (the 2012 tables have duplicates);
/// lookups return the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringTable {
    language: u32,
    entries: Vec<StringEntry>,
}

fn u32_at(block: &[u8], offset: usize) -> usize {
    u32::from_le_bytes([block[offset], block[offset + 1], block[offset + 2], block[offset + 3]]) as usize
}

fn slice<'a>(block: &'a [u8], what: &'static str, start: usize, len: usize) -> Result<&'a [u8]> {
    let end = start.saturating_add(len);
    block.get(start..end).ok_or(Error::Truncated { what, start, end, len: block.len() })
}

impl StringTable {
    /// A table from `entries`, put in id order (entries with the same id keep their order).
    pub fn new(language: u32, mut entries: Vec<StringEntry>) -> Self {
        entries.sort_by_key(|entry| entry.id);
        Self { language, entries }
    }

    /// Read the main block of a `LocalisedText` resource.
    pub fn parse(block: &[u8]) -> Result<Self> {
        let header = slice(block, "header", 0, HEADER_SIZE)?;
        let (language, count) = (u32_at(header, 0), u32_at(header, 4));
        let (ids_at, records_at) = (u32_at(header, 8), u32_at(header, 12));
        let ids = slice(block, "ids", ids_at, count.saturating_mul(4))?;
        let records = slice(block, "records", records_at, count.saturating_mul(RECORD_SIZE))?;
        let mut entries = Vec::with_capacity(count);
        for index in 0..count {
            let (text_at, units) = (u32_at(records, RECORD_SIZE * index), u32_at(records, RECORD_SIZE * index + 4));
            let raw = slice(block, "text", text_at, units.saturating_mul(2))?;
            let mut units: Vec<u16> = raw.as_chunks::<2>().0.iter().map(|&pair| u16::from_le_bytes(pair)).collect();
            if units.pop() != Some(0) {
                return Err(Error::Unterminated { index });
            }
            let text = String::from_utf16(&units).map_err(|_| Error::BadText { index })?;
            entries.push(StringEntry { id: u32_at(ids, 4 * index) as u32, text });
        }
        Ok(Self::new(language as u32, entries))
    }

    /// The block bytes: header, ids, records, then the texts one after another, padded to 16 bytes. For a table read
    /// from a game file this reproduces the file's block exactly.
    pub fn to_bytes(&self) -> Vec<u8> {
        let count = self.entries.len();
        let ids_at = HEADER_SIZE;
        let records_at = ids_at + 4 * count;
        let mut text_at = records_at + RECORD_SIZE * count;
        let mut out = Vec::new();
        for value in [self.language, count as u32, ids_at as u32, records_at as u32] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        for entry in &self.entries {
            out.extend_from_slice(&entry.id.to_le_bytes());
        }
        for entry in &self.entries {
            let units = entry.text.encode_utf16().count() + 1;
            out.extend_from_slice(&(text_at as u32).to_le_bytes());
            out.extend_from_slice(&(units as u32).to_le_bytes());
            text_at += units * 2;
        }
        for entry in &self.entries {
            for unit in entry.text.encode_utf16().chain([0]) {
                out.extend_from_slice(&unit.to_le_bytes());
            }
        }
        out.resize(out.len().next_multiple_of(BLOCK_ALIGNMENT), 0);
        out
    }

    /// The language index (1 to 14, the number in the bundle's file name).
    pub fn language(&self) -> u32 {
        self.language
    }

    pub fn entries(&self) -> &[StringEntry] {
        &self.entries
    }

    /// The text of the first entry with id `id`.
    pub fn get(&self, id: u32) -> Option<&str> {
        self.all(id).first().map(|entry| entry.text.as_str())
    }

    /// Every entry with id `id`, in order.
    pub fn all(&self, id: u32) -> &[StringEntry] {
        let start = self.entries.partition_point(|entry| entry.id < id);
        let len = self.entries[start..].partition_point(|entry| entry.id == id);
        &self.entries[start..start + len]
    }

    /// Replace the text of the first entry with id `id`, or add the entry. Returns the old text.
    pub fn set(&mut self, id: u32, text: impl Into<String>) -> Option<String> {
        let start = self.entries.partition_point(|entry| entry.id < id);
        let text = text.into();
        match self.entries.get_mut(start).filter(|entry| entry.id == id) {
            Some(entry) => Some(std::mem::replace(&mut entry.text, text)),
            None => {
                self.entries.insert(start, StringEntry { id, text });
                None
            }
        }
    }
}
