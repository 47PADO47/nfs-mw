//! The header of one speech bank.

use crate::bytes::{u8_at, u16_le};
use crate::error::{Error, Result};

/// Sample positions in a bank header count in units of this many bytes (the `.big` aligns streams to it).
pub const SAMPLE_UNIT: u64 = 0x100;

/// Bytes before the sample table: event (2), speaker (2), entry flags (1), sample count (1) and eight bytes whose
/// first four are the count again and the bank's length in units of [`SAMPLE_UNIT`] (the rest are zero).
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
    /// Bytes of the header after the sample table; meaning unknown (flags of the takes, mostly `0x70`).
    pub extra: Vec<u8>,
}

impl BankHeader {
    /// Parse a header. The sample table has one entry per take after the first: `2 + flags` bytes, the last two
    /// being the start in [`SAMPLE_UNIT`]s, big-endian. The bytes before them are per-take flags of unknown meaning.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let event = u16_le(data, 0)?;
        let speaker = u16_le(data, 2)?;
        let flags = usize::from(u8_at(data, 4)?);
        let count = usize::from(u8_at(data, 5)?);
        if count == 0 {
            return Err(Error::BadHeader("a speech bank without takes"));
        }
        let stride = 2 + flags;
        let table_end = FIXED + (count - 1) * stride;
        let Some(table) = data.get(FIXED..table_end) else {
            return Err(Error::Truncated { offset: FIXED as u64, needed: table_end - FIXED });
        };
        let mut starts = vec![0u64];
        for entry in table.chunks_exact(stride) {
            let units = u16::from_be_bytes([entry[stride - 2], entry[stride - 1]]);
            starts.push(u64::from(units) * SAMPLE_UNIT);
        }
        if starts.windows(2).any(|w| w[0] >= w[1]) {
            return Err(Error::Corrupt("speech take starts do not increase"));
        }
        Ok(Self { event, speaker, starts, extra: data[table_end..].to_vec() })
    }

    /// How many takes the bank holds.
    pub fn takes(&self) -> usize {
        self.starts.len()
    }
}
