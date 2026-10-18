//! The event database `copspeech.evt`: which phrases make up a sentence.
//!
//! A record is one event: its number (a `u16`), then fixed fields, then blocks that start with `39 64`. Inside a block,
//! 4-byte entries `<u16 event> 01 01` name the phrases the sentence says, each followed by parameter tests that choose
//! among variants. The tests are not read here, so [`Sentences::phrases`] gives every `01 01` phrase of the record in
//! file order. Layout: `docs/formats/audio.md`.

use std::collections::HashMap;

use crate::bytes::{slice, u16_le, u32_le};
use crate::error::Result;

/// Where the record count sits, then the record positions (each `u16`, in units of 4 bytes) from `TABLE_AT`.
const COUNT_AT: usize = 0x10;
const TABLE_AT: usize = 0x18;
/// The marker that opens a block of entries.
const BLOCK: [u8; 2] = [0x39, 0x64];
/// The flag bytes of an entry that names a phrase.
const SAID: [u8; 2] = [0x01, 0x01];

/// The phrases of every sentence in a `copspeech.evt`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sentences {
    phrases: HashMap<u16, Vec<u16>>,
}

impl Sentences {
    /// Parse a `copspeech.evt`.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let count = u32_le(data, COUNT_AT)? as usize;
        let mut starts = Vec::with_capacity(count);
        for i in 0..count {
            starts.push(u16_le(data, TABLE_AT + 2 * i)? as usize * 4);
        }
        let mut phrases = HashMap::new();
        for &start in &starts {
            let end = starts.iter().filter(|&&s| s > start).min().copied().unwrap_or(data.len());
            let record = slice(data, start, end.saturating_sub(start))?;
            let event = u16_le(record, 0)?;
            phrases.insert(event, phrases_of(record));
        }
        Ok(Self { phrases })
    }

    /// The phrases the sentence `event` says, in file order (empty when `event` is not a sentence here).
    pub fn phrases(&self, event: u16) -> &[u16] {
        self.phrases.get(&event).map_or(&[], Vec::as_slice)
    }
}

/// The `01 01` phrases of one record: the 4-byte entries after each block marker, read on 4-byte steps.
fn phrases_of(record: &[u8]) -> Vec<u16> {
    let mut out = Vec::new();
    let mut in_block = false;
    for at in (4..record.len().saturating_sub(3)).step_by(4) {
        if record[at..at + 2] == BLOCK {
            in_block = true;
            continue;
        }
        if in_block && record[at + 2..at + 4] == SAID {
            out.push(u16::from_le_bytes([record[at], record[at + 1]]));
        }
    }
    out
}
