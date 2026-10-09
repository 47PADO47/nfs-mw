//! The `.idx` file: the table of banks and the headers behind it.

use crate::bytes::{slice, u32_le};
use crate::error::{Error, Result};
use crate::pcm::Pcm;
use crate::schl::StreamReader;
use crate::source::ReadAt;
use crate::speech::bank::BankHeader;

/// Where the bank count sits: a type count, then room for 19 more `u32` (bank counts per type).
const COUNT_AT: usize = 0x54;
/// Where the table starts: after the count.
const TABLE_AT: usize = 0x58;
/// Bytes per table entry: key, header size, header position, position of the takes in the `.big`.
const ENTRY: usize = 16;

/// One bank: its place in the `.big` and what its header says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeechBank {
    /// The bank's type: the high byte of its key (1 in the game's speech index).
    pub kind: u8,
    /// The bank's number within its type: the low 24 bits of its key. The numbers count up from 0 in the file.
    pub number: u32,
    /// Where the first take starts in the `.big`.
    pub base: u64,
    pub header: BankHeader,
}

impl SpeechBank {
    /// Where take `take` starts in the `.big`.
    pub fn take_offset(&self, take: usize) -> Option<u64> {
        Some(self.base + self.header.starts.get(take)?)
    }

    /// Decode take `take` from the `.big`.
    pub fn decode<S: ReadAt + ?Sized>(&self, big: &S, take: usize) -> Result<Pcm> {
        let offset = self.take_offset(take).ok_or(Error::NoSuchEntry(take))?;
        StreamReader::open(big, offset)?.read_all()
    }
}

/// Every bank of a speech `.big`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpeechIndex {
    banks: Vec<SpeechBank>,
}

impl SpeechIndex {
    /// Parse an `.idx` file.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let count = u32_le(data, COUNT_AT)? as usize;
        let table_len = count.checked_mul(ENTRY).ok_or(Error::BadHeader("too many speech banks"))?;
        let table = slice(data, TABLE_AT, table_len)?;
        let mut banks = Vec::with_capacity(count);
        for entry in table.as_chunks::<ENTRY>().0 {
            let key = u32_le(entry, 0)?;
            let (size, at, base) = (u32_le(entry, 4)? as usize, u32_le(entry, 8)? as usize, u32_le(entry, 12)?);
            let header = BankHeader::parse(slice(data, at, size)?)?;
            banks.push(SpeechBank {
                kind: (key >> 24) as u8,
                number: key & 0x00FF_FFFF,
                base: u64::from(base),
                header,
            });
        }
        Ok(Self { banks })
    }

    pub fn banks(&self) -> &[SpeechBank] {
        &self.banks
    }

    /// The banks that say phrase `event`, one per speaker (more when the phrase has variants).
    pub fn banks_for(&self, event: u16) -> impl Iterator<Item = &SpeechBank> {
        self.banks.iter().filter(move |b| b.header.event == event)
    }

    /// How many takes all the banks hold together.
    pub fn take_count(&self) -> usize {
        self.banks.iter().map(|b| b.header.takes()).sum()
    }
}
