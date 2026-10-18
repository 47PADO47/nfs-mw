//! Start offsets of the arrays of the milestone block (`docs/formats/career-save.md` section 3.1a).
//!
//! Each start follows from the header counts and the object record sizes, and nothing else is read. Every part is
//! checked against the 0x4000 block, so a count that does not fit is an error, not a panic.

use super::{BLOCK_LEN, MILESTONE_LEN, MilestoneBlock, RECORDS_AT, SPEED_TRAP_LEN, STAT_NODE_LEN};
use crate::LayoutError;
use crate::bytes::u32_at;

const OBJECT_COUNT_AT: usize = 0x18;
const SLOT_COUNT_AT: usize = 0x1C;
const STAT_COUNT_AT: usize = 0x20;
const MILESTONE_COUNT_AT: usize = 0x24;
const TRAP_COUNT_AT: usize = 0x28;
const FLAG_BITS_AT: usize = 0x30;
const BIN_BLOCK_LEN_AT: usize = 0x34;
const KEY_COUNT_AT: usize = 0x38;

const OBJECT_HEADER_LEN: usize = 8;
const SLOT_LEN: usize = 0x20;
const BOUNTY_KEY_LEN: usize = 4;
const REFERENCE_LEN: usize = 4;
const REFERENCE_STEP: usize = 0x10;

/// The start of each array of the block, and the counts that size them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockLayout {
    /// Start of the slot block (8 slots of 0x20 bytes).
    pub slots: usize,
    /// Start of the statistic nodes.
    pub stats: usize,
    /// Number of statistic nodes (header `0x20`).
    pub stat_count: usize,
    /// Start of the milestone entries.
    pub milestones: usize,
    /// Number of milestone entries (header `0x24`).
    pub milestone_count: usize,
    /// Start of the speed trap entries.
    pub speed_traps: usize,
    /// Number of speed trap entries (header `0x28`).
    pub speed_trap_count: usize,
    /// Start of the flag bitfield.
    pub flag_bits: usize,
    /// Byte length of the flag bitfield (header `0x30` bits, divided by 8).
    pub flag_bits_len: usize,
    /// Start of the bin counter block; its u32 count comes first.
    pub bin_counters: usize,
    /// Byte length of the bin counter block (header `0x34`).
    pub bin_counters_len: usize,
    /// Start of the bounty-jump keys.
    pub bounty_keys: usize,
    /// Number of bounty-jump keys (header `0x38`).
    pub bounty_key_count: usize,
    /// Start of the first of the two references; the second is 0x10 further on.
    pub references: usize,
}

impl MilestoneBlock {
    /// The start of each array, from the header counts and the object records. An error when a count or an object
    /// record does not fit in the 0x4000 block.
    pub fn layout(&self) -> Result<BlockLayout, LayoutError> {
        read_layout(self.bytes())
    }
}

fn read_layout(raw: &[u8]) -> Result<BlockLayout, LayoutError> {
    let slots = align16(objects_end(raw, word(raw, OBJECT_COUNT_AT))?);
    let slot_count = word(raw, SLOT_COUNT_AT);
    let stats = span(slots, slot_count, SLOT_LEN)?;
    let stat_count = word(raw, STAT_COUNT_AT);
    let milestones = span(stats, stat_count, STAT_NODE_LEN)?;
    let milestone_count = word(raw, MILESTONE_COUNT_AT);
    let speed_traps = align16(span(milestones, milestone_count, MILESTONE_LEN)?);
    let speed_trap_count = word(raw, TRAP_COUNT_AT);
    let flag_bits = align16(span(speed_traps, speed_trap_count, SPEED_TRAP_LEN)?);
    let flag_bits_len = word(raw, FLAG_BITS_AT) / 8;
    let bin_counters = span(flag_bits, flag_bits_len, 1)?;
    let bin_counters_len = word(raw, BIN_BLOCK_LEN_AT);
    let bounty_keys = align16(span(bin_counters, bin_counters_len, 1)?);
    let bounty_key_count = word(raw, KEY_COUNT_AT);
    let references = align16(span(bounty_keys, bounty_key_count, BOUNTY_KEY_LEN)?);
    within(references + REFERENCE_STEP + REFERENCE_LEN)?;
    Ok(BlockLayout {
        slots,
        stats,
        stat_count,
        milestones,
        milestone_count,
        speed_traps,
        speed_trap_count,
        flag_bits,
        flag_bits_len,
        bin_counters,
        bin_counters_len,
        bounty_keys,
        bounty_key_count,
        references,
    })
}

/// The end of the object records: walks `count` records from `0x80`. Each is `{u32 key, u32 size, size bytes}` and the
/// next one starts at `(size + 0x17) & !0xF` bytes from its own start.
fn objects_end(raw: &[u8], count: usize) -> Result<usize, LayoutError> {
    let mut at = RECORDS_AT;
    for _ in 0..count {
        within(at.saturating_add(OBJECT_HEADER_LEN))?;
        let size = word(raw, at + 4);
        within(at.saturating_add(OBJECT_HEADER_LEN).saturating_add(size))?;
        at += size.saturating_add(0x17) & !0xF;
    }
    within(at)
}

/// The header word at `at`, as a count. The header is always inside the block.
fn word(raw: &[u8], at: usize) -> usize {
    usize::try_from(u32_at(raw, at)).unwrap_or(usize::MAX)
}

/// The end of `count` items of `each` bytes that start at `start`, checked against the block.
fn span(start: usize, count: usize, each: usize) -> Result<usize, LayoutError> {
    let len = count.saturating_mul(each);
    within(start.saturating_add(len))
}

/// `end` when it is inside the block, otherwise an error that names the size needed.
fn within(end: usize) -> Result<usize, LayoutError> {
    if end > BLOCK_LEN {
        return Err(LayoutError { what: "milestone block layout", need: end, got: BLOCK_LEN });
    }
    Ok(end)
}

const fn align16(x: usize) -> usize {
    (x + 15) & !15
}
