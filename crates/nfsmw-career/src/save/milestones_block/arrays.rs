//! The records of the milestone block's arrays, read and written in place at the offsets of [`BlockLayout`].
//!
//! A setter takes the whole array and keeps the header count: a slice of another length is an error and nothing is
//! written. The digest at `0x00` is not refreshed here; call [`MilestoneBlock::refresh_digest`] after the last change.

use super::layout::BlockLayout;
use super::{
    BIN_COUNTER_LEN, BinCounter, MILESTONE_LEN, MilestoneBlock, MilestoneEntry, SPEED_TRAP_LEN, STAT_NODE_LEN,
    SpeedTrapEntry, StatNode,
};
use crate::LayoutError;
use crate::bytes::u32_at;

/// Why an array could not be read or written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArrayError {
    /// The layout or a record does not fit in the block.
    #[error(transparent)]
    Layout(#[from] LayoutError),
    /// A setter got another number of records than the header count.
    #[error("{what}: the block holds {have}, got {given}")]
    Count {
        /// The array.
        what: &'static str,
        /// The count in the header.
        have: usize,
        /// The count that was given.
        given: usize,
    },
}

impl MilestoneBlock {
    /// The statistic nodes in array order (header count `0x20`).
    pub fn stats(&self) -> Result<Vec<StatNode>, ArrayError> {
        let layout = self.layout()?;
        read_records(self.bytes(), layout.stats, layout.stat_count, STAT_NODE_LEN, StatNode::from_bytes)
    }

    /// Writes the statistic nodes in place. `nodes` must hold as many nodes as the header count says.
    pub fn set_stats(&mut self, nodes: &[StatNode]) -> Result<(), ArrayError> {
        let layout = self.layout()?;
        check_count("statistic nodes", layout.stat_count, nodes.len())?;
        write_records(&mut self.raw[..], layout.stats, STAT_NODE_LEN, nodes, StatNode::write_into)
    }

    /// The milestone entries in array order (header count `0x24`).
    pub fn milestones(&self) -> Result<Vec<MilestoneEntry>, ArrayError> {
        let layout = self.layout()?;
        read_records(self.bytes(), layout.milestones, layout.milestone_count, MILESTONE_LEN, MilestoneEntry::from_bytes)
    }

    /// Writes the milestone entries in place. `entries` must hold as many entries as the header count says.
    pub fn set_milestones(&mut self, entries: &[MilestoneEntry]) -> Result<(), ArrayError> {
        let layout = self.layout()?;
        check_count("milestone entries", layout.milestone_count, entries.len())?;
        write_records(&mut self.raw[..], layout.milestones, MILESTONE_LEN, entries, MilestoneEntry::write_into)
    }

    /// The speed trap entries in array order (header count `0x28`).
    pub fn speed_traps(&self) -> Result<Vec<SpeedTrapEntry>, ArrayError> {
        let layout = self.layout()?;
        let count = layout.speed_trap_count;
        read_records(self.bytes(), layout.speed_traps, count, SPEED_TRAP_LEN, SpeedTrapEntry::from_bytes)
    }

    /// Writes the speed trap entries in place. `traps` must hold as many entries as the header count says.
    pub fn set_speed_traps(&mut self, traps: &[SpeedTrapEntry]) -> Result<(), ArrayError> {
        let layout = self.layout()?;
        check_count("speed trap entries", layout.speed_trap_count, traps.len())?;
        write_records(&mut self.raw[..], layout.speed_traps, SPEED_TRAP_LEN, traps, SpeedTrapEntry::write_into)
    }

    /// The bin counter records. The block starts with a u32 count, then the records.
    pub fn bin_counters(&self) -> Result<Vec<BinCounter>, ArrayError> {
        let layout = self.layout()?;
        let count = bin_count(self.bytes(), &layout)?;
        read_records(self.bytes(), layout.bin_counters + 4, count, BIN_COUNTER_LEN, BinCounter::from_bytes)
    }

    /// Writes the bin counter records in place. `counters` must hold as many records as the block's count says.
    pub fn set_bin_counters(&mut self, counters: &[BinCounter]) -> Result<(), ArrayError> {
        let layout = self.layout()?;
        let count = bin_count(self.bytes(), &layout)?;
        check_count("bin counters", count, counters.len())?;
        write_records(&mut self.raw[..], layout.bin_counters + 4, BIN_COUNTER_LEN, counters, BinCounter::write_into)
    }
}

/// The u32 count at the start of the bin counter block. The records after it must fit in the block's byte size.
fn bin_count(raw: &[u8], layout: &BlockLayout) -> Result<usize, ArrayError> {
    let have = layout.bin_counters_len;
    if have < 4 {
        return Err(LayoutError { what: "bin counter block", need: 4, got: have }.into());
    }
    let count = usize::try_from(u32_at(raw, layout.bin_counters)).unwrap_or(usize::MAX);
    let need = count.saturating_mul(BIN_COUNTER_LEN).saturating_add(4);
    if need > have {
        return Err(LayoutError { what: "bin counter block", need, got: have }.into());
    }
    Ok(count)
}

fn read_records<T>(
    raw: &[u8],
    at: usize,
    count: usize,
    each: usize,
    read: fn(&[u8]) -> Result<T, LayoutError>,
) -> Result<Vec<T>, ArrayError> {
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let start = at + i * each;
        out.push(read(&raw[start..start + each])?);
    }
    Ok(out)
}

fn write_records<T>(
    raw: &mut [u8],
    at: usize,
    each: usize,
    items: &[T],
    write: fn(&T, &mut [u8]) -> Result<(), LayoutError>,
) -> Result<(), ArrayError> {
    for (i, item) in items.iter().enumerate() {
        let start = at + i * each;
        write(item, &mut raw[start..start + each])?;
    }
    Ok(())
}

fn check_count(what: &'static str, have: usize, given: usize) -> Result<(), ArrayError> {
    if have != given {
        return Err(ArrayError::Count { what, have, given });
    }
    Ok(())
}
