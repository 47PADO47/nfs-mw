//! The header of one speech bank.

use crate::bytes::{u8_at, u16_le};
use crate::error::{Error, Result};

/// Base alignment unit; each bank multiplies it by one plus its alignment byte.
pub const SAMPLE_UNIT: u64 = 0x100;

/// Bytes before the sample table: event (2), speaker (2), selector count (1), take count (1), repeat memory (1),
/// alignment multiplier (1), bank length in units (2), padding (2), first offset (2).
const FIXED: usize = 14;

/// What a bank header says about its takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankHeader {
    /// The phrase the takes say: an event number of the speech database.
    pub event: u16,
    /// The voice that says it (1 to 9), or `0xFFFF` for any.
    pub speaker: u16,
    /// Start of each take from the start of the bank, in bytes. The first is 0.
    pub starts: Vec<u64>,
    /// How many of the last takes said are kept out of the choice (the *repeat memory*): a take that is among
    /// them is only said again when no other take fits. 0 means takes may repeat at once.
    pub memory: u8,
    /// Selectors per take (0 to 3): bit indices that say which parameter values a take is for.
    pub selector_count: usize,
    /// The selectors of every take, `selector_count` bytes each, in take order.
    pub selectors: Vec<u8>,
    /// Bytes of the header after the last take's selectors: per-selector bit masks (the union of that selector over
    /// the takes), then the repeat memory, which the game fills while it plays.
    pub extra: Vec<u8>,
}

impl BankHeader {
    /// Parse a header. The sample table has one entry per take: `selector_count` selector bytes, then the start of
    /// the *next* take in [`SAMPLE_UNIT`]s, big-endian; the last take has its selectors only.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let event = u16_le(data, 0)?;
        let speaker = u16_le(data, 2)?;
        let selector_count = usize::from(u8_at(data, 4)?);
        let count = usize::from(u8_at(data, 5)?);
        let memory = u8_at(data, 6)?;
        if count == 0 {
            return Err(Error::BadHeader("a speech bank without takes"));
        }
        let stride = 2 + selector_count;
        let unit = SAMPLE_UNIT * (u64::from(u8_at(data, 7)?) + 1);
        let table_end = FIXED + (count - 1) * stride + selector_count;
        let Some(table) = data.get(FIXED..table_end) else {
            return Err(Error::Truncated { offset: FIXED as u64, needed: table_end - FIXED });
        };
        let mut starts = vec![0u64];
        let mut selectors = Vec::with_capacity(count * selector_count);
        for entry in table.chunks(stride) {
            selectors.extend_from_slice(&entry[..selector_count]);
            let Some(next) = entry.get(selector_count..) else { continue };
            if next.len() < 2 {
                continue;
            }
            starts.push(u64::from(u16::from_be_bytes([next[0], next[1]])) * unit);
        }
        if starts.windows(2).any(|w| w[0] >= w[1]) {
            return Err(Error::Corrupt("speech take starts do not increase"));
        }
        Ok(Self { event, speaker, starts, memory, selector_count, selectors, extra: data[table_end..].to_vec() })
    }

    /// How many takes the bank holds.
    pub fn takes(&self) -> usize {
        self.starts.len()
    }

    /// The selector bit indices of take `take` (empty for a bank without selectors or a take out of range).
    pub fn selectors_of(&self, take: usize) -> &[u8] {
        let at = take * self.selector_count;
        self.selectors.get(at..at + self.selector_count).unwrap_or(&[])
    }
}
