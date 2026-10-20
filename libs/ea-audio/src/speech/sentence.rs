//! One record of the event database: the recipe of a sentence.
//!
//! A *sentence* is a list of alternative *blocks* (versions of the wording). A block is a list of *entries*, said one
//! after another; an entry names a phrase (a bank of takes), says whose voice to use, and carries *tests* that choose
//! among the phrase's takes by the request's parameters. Layout: `docs/formats/speech-evt.md`.

use crate::bytes::{slice, u8_at, u16_le, u32_le};
use crate::error::{Error, Result};

/// Weight of a block by its first byte: the low five bits times 1, 4, 16 or 64 by the top three bits.
const WEIGHT_SCALE: [u32; 4] = [1, 4, 16, 64];
/// Bytes before the entry-offset list of a block.
const BLOCK_HEADER: usize = 8;
/// Bytes of an entry before its tests, and of one test.
const ENTRY_HEADER: usize = 8;
const TEST: usize = 8;
/// Where a record keeps its block count, and its table of block positions (`u16`, in units of 4 bytes).
const BLOCK_COUNT_AT: usize = 6;
const BLOCK_TABLE_AT: usize = 12;

/// Whose recording of a phrase an entry wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voice {
    /// Any bank of the phrase (phrases recorded once, for every speaker).
    Any,
    /// The bank whose speaker equals the value of this parameter number.
    Speaker(u8),
}

/// A condition on one take of an entry's phrase. The nth test constrains the nth selector of a take: the take's
/// selector bit must be in `mask`, and set in the value of parameter `param` (0: no parameter, the mask only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Test {
    pub mask: u32,
    pub param: u8,
}

/// One phrase of a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub phrase: u16,
    pub voice: Voice,
    pub tests: Vec<Test>,
}

/// One version of a sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// How likely the block is to be tried first among the versions (0: only after the others, in file order).
    pub weight: u32,
    /// The chance in percent that the block is considered at all.
    pub chance: u8,
    pub entries: Vec<Entry>,
}

/// The recipe of one event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence {
    pub event: u16,
    pub blocks: Vec<Block>,
}

impl Sentence {
    /// Parse one record (the bytes from its event number up to the next record, or the end of the file).
    pub fn parse(record: &[u8]) -> Result<Self> {
        let event = u16_le(record, 0)?;
        let count = usize::from(u8_at(record, BLOCK_COUNT_AT)?);
        let mut blocks = Vec::with_capacity(count);
        for i in 0..count {
            let at = usize::from(u16_le(record, BLOCK_TABLE_AT + 2 * i)?) * 4;
            blocks.push(Block::parse(record, at)?);
        }
        Ok(Self { event, blocks })
    }
}

impl Block {
    fn parse(record: &[u8], at: usize) -> Result<Self> {
        let weight_byte = u8_at(record, at)?;
        let chance = u8_at(record, at + 1)?;
        let entry_count = usize::from(u8_at(record, at + 2)? >> 2);
        let offsets = slice(record, at + BLOCK_HEADER, entry_count)?;
        let mut entries = Vec::with_capacity(entry_count);
        for &offset in offsets {
            entries.push(Entry::parse(record, at + usize::from(offset) * 4)?);
        }
        let weight = u32::from(weight_byte & 0x1F) * WEIGHT_SCALE[usize::from(weight_byte >> 5) & 3];
        Ok(Self { weight, chance, entries })
    }
}

impl Entry {
    fn parse(record: &[u8], at: usize) -> Result<Self> {
        let phrase = u16_le(record, at)?;
        let param = u8_at(record, at + 2)?;
        let voice = match u8_at(record, at + 3)? {
            0 => Voice::Any,
            1 => Voice::Speaker(param),
            _ => return Err(Error::Unsupported("a phrase chosen by a speaker group")),
        };
        let count = u32_le(record, at + 4)? as usize;
        let list = slice(record, at + ENTRY_HEADER, count.checked_mul(TEST).ok_or(Error::Corrupt("test count"))?)?;
        let mut tests = Vec::with_capacity(count);
        for test in list.as_chunks::<TEST>().0 {
            let param = test[4];
            if param > 0x7F {
                return Err(Error::Unsupported("a test on a rule variable"));
            }
            tests.push(Test { mask: u32_le(test, 0)?, param });
        }
        Ok(Self { phrase, voice, tests })
    }
}
