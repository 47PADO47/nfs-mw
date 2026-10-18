//! The milestone block of a new career: the header, the arrays and the digest of a block with no progress
//! (`docs/formats/career-save.md` 3.1a, `docs/specs/career-milestones.md` 7).
//!
//! The arrays are written with the setters of `arrays.rs`, at the offsets the header counts give, so the block is
//! laid out by the same code that reads it back. The object records are none, the slots are zero, the flag bits are
//! clear.
//!
//! Unconfirmed: the statistic node keys and the milestone type keys are not in the data, so the callers pass `0`
//! for them. The save's values for those keys are not known.

use super::{
    ArrayError, BLOCK_LEN, BinCounter, GAME_MAGIC, MIN_VERSION, MilestoneBlock, MilestoneEntry, SpeedTrapEntry,
    StatNode, stat_flags,
};
use crate::bytes::put_u32;
use crate::milestones::speedtrap::{SpeedTrap, SpeedTrapDef};
use crate::milestones::{MilestoneDef, MilestoneState, STAT_COUNT, StatType};

const VERSION_AT: usize = 0x14;
const OBJECT_COUNT_AT: usize = 0x18;
const SLOT_COUNT_AT: usize = 0x1C;
const STAT_COUNT_AT: usize = 0x20;
const MILESTONE_COUNT_AT: usize = 0x24;
const TRAP_COUNT_AT: usize = 0x28;
const FLAG_BITS_AT: usize = 0x30;
const BIN_BLOCK_LEN_AT: usize = 0x34;
const KEY_COUNT_AT: usize = 0x38;

/// The slot count the saves hold (`career-save.md` 3.1a, header `0x1C`).
const SLOT_COUNT: u32 = 8;
/// The flag bit count the saves hold (64 bytes).
const FLAG_BITS: u32 = 0x200;
/// The bin counter block size the saves hold: a u32 count and 20 records of 8 bytes.
const BIN_BLOCK_LEN: u32 = 164;

impl MilestoneBlock {
    /// A block for a new career: no object records, 8 empty slots, every statistic node "not set" (`-1.0`), the
    /// milestones and traps as given (the caller locks them with [`MilestoneEntry::new_locked`] and
    /// [`SpeedTrapEntry::new_locked`]), the flag bits clear and the bin counters as given. The header counts follow the
    /// slices. The digest is refreshed.
    ///
    /// An error when the arrays do not fit in the block, or when there are more bin counters than the 164-byte block
    /// holds (20).
    pub fn new_career(
        milestones: &[MilestoneEntry],
        traps: &[SpeedTrapEntry],
        bins: &[BinCounter],
    ) -> Result<MilestoneBlock, ArrayError> {
        let mut block = MilestoneBlock { raw: Box::new([0; BLOCK_LEN]) };
        write_header(&mut block, milestones.len(), traps.len());
        let layout = block.layout()?;
        let stats = stat_nodes();
        block.set_stats(&stats)?;
        block.set_milestones(milestones)?;
        block.set_speed_traps(traps)?;
        put_u32(&mut block.raw[..], layout.bin_counters, bins.len() as u32);
        block.set_bin_counters(bins)?;
        block.refresh_digest();
        Ok(block)
    }
}

/// The magic, the version and the header counts. Object records and bounty-jump keys are none.
fn write_header(block: &mut MilestoneBlock, milestones: usize, traps: usize) {
    let raw = &mut block.raw[..];
    put_u32(raw, 0x10, GAME_MAGIC);
    put_u32(raw, VERSION_AT, MIN_VERSION);
    put_u32(raw, OBJECT_COUNT_AT, 0);
    put_u32(raw, SLOT_COUNT_AT, SLOT_COUNT);
    put_u32(raw, STAT_COUNT_AT, STAT_COUNT as u32);
    put_u32(raw, MILESTONE_COUNT_AT, milestones as u32);
    put_u32(raw, TRAP_COUNT_AT, traps as u32);
    put_u32(raw, 0x2C, 0);
    put_u32(raw, FLAG_BITS_AT, FLAG_BITS);
    put_u32(raw, BIN_BLOCK_LEN_AT, BIN_BLOCK_LEN);
    put_u32(raw, KEY_COUNT_AT, 0);
}

/// One node per statistic, in `StatType::ALL` order: key `0` (unconfirmed), no current or best value, the flags the
/// statistic implies.
fn stat_nodes() -> Vec<StatNode> {
    StatType::ALL.iter().map(|&stat| StatNode { key: 0, current: None, best: None, flags: stat_flags(stat) }).collect()
}

impl MilestoneEntry {
    /// The entry of milestone `def` in a new career: locked, nothing banked. `type_key` is the statistic node key
    /// (unconfirmed; the caller passes `0` when it is not known).
    pub fn new_locked(def: MilestoneDef, type_key: u32) -> MilestoneEntry {
        MilestoneEntry {
            type_key,
            key: def.key,
            state_byte: MilestoneState::Locked.byte(),
            flags: u8::from(def.stat.bigger_is_better()),
            bin: u16::from(def.bin),
            threshold: def.threshold,
            value: 0.0,
        }
    }
}

impl SpeedTrapEntry {
    /// The entry of speed trap `def` in a new career: flags `0` (locked, not armed, not done), nothing recorded.
    pub fn new_locked(def: SpeedTrapDef) -> SpeedTrapEntry {
        let trap = SpeedTrap::new(def);
        SpeedTrapEntry {
            flags: trap.flags(),
            bin: u16::from(trap.bin()),
            key: trap.key(),
            unknown_0x08: 0,
            threshold_mps: trap.threshold_mps(),
            recorded_mps: trap.recorded_mps(),
        }
    }
}

impl BinCounter {
    /// The counter of `bin` in a new career: nothing completed, no events.
    pub fn new_career(bin: u16) -> BinCounter {
        BinCounter { bin, length: 4, completed: 0, events: 0 }
    }
}

#[cfg(test)]
mod tests;
