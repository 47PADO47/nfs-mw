//! The 16 KiB milestone block at the start of the profile data, known parts only
//! (`docs/specs/career-milestones.md` section 7; `docs/formats/career-save.md` sections 3.1 and 3.2).
//!
//! The block is the same 0x4000 bytes as `settings::CareerSettings::gameplay_blob` (file `0x34`, profile `0x10`). Its
//! MD5 at `0x00` covers `0x10..0x4000`, the magic `Game` is at `0x10` and the version (a load needs at least 8) at
//! `0x14`.
//!
//! Decoded here, at the offsets the docs give:
//! - the header words and the digest ([`MilestoneBlock`]);
//! - the 16-byte statistic node ([`StatNode`]), the 20-byte milestone entry ([`MilestoneEntry`]), the 20-byte speed
//!   trap entry ([`SpeedTrapEntry`]) and the 8-byte bin counter record ([`BinCounter`]).
//!
//! The array offsets come from the header counts and the object sizes ([`MilestoneBlock::layout`], in `layout.rs`,
//! `career-save.md` 3.1a). The records are read and written in place by the accessors in `arrays.rs`.
//! [`MilestoneBlock`] keeps all 16 KiB raw; every write touches only the bytes of the record it is given, so a block
//! that is read and written back is byte for byte the same.
//!
//! Conversions to the in-memory types of [`crate::milestones`]: the save's `-1.0` is `None` in memory
//! ([`opt_from_save`], [`opt_to_save`]), and the milestone entry's value `0` is "not banked" (`None`). The save does
//! not hold a milestone's statistic or bounty, so a milestone is rebuilt from its data definition, matched by key.

mod arrays;
mod layout;
#[cfg(test)]
mod tests;

pub use arrays::ArrayError;
pub use layout::BlockLayout;

use crate::LayoutError;
use crate::bytes::{array_at, f32_at, put_f32, put_u16, put_u32, u16_at, u32_at};
use crate::milestones::speedtrap::{self, SpeedTrap, SpeedTrapDef};
use crate::milestones::{Milestone, MilestoneDef, MilestoneSet, MilestoneState, StatType, StatValue};
use save_store::digest::md5;

/// Size of the block (the gameplay-state blob, file `0x34..0x4033`).
pub const BLOCK_LEN: usize = 0x4000;
/// The magic `Game` as a little-endian u32, at `0x10`.
pub const GAME_MAGIC: u32 = 0x656D_6147;
/// The lowest version a load accepts, at `0x14`.
pub const MIN_VERSION: u32 = 8;
/// The "not set" value the save uses for an unset statistic value.
pub const NOT_SET: f32 = -1.0;
/// Offset of the first record area (script state records, then the arrays).
pub const RECORDS_AT: usize = 0x80;
/// Size of a statistic node.
pub const STAT_NODE_LEN: usize = 16;
/// Size of a milestone entry.
pub const MILESTONE_LEN: usize = 20;
/// Size of a speed trap entry.
pub const SPEED_TRAP_LEN: usize = 20;
/// Size of a bin counter record.
pub const BIN_COUNTER_LEN: usize = 8;

const DIGEST_AT: usize = 0x00;
const DIGEST_LEN: usize = 16;
const DIGEST_FROM: usize = 0x10;
const MAGIC_AT: usize = 0x10;
const VERSION_AT: usize = 0x14;

/// The save's "not set" value as an in-memory `None`.
pub fn opt_from_save(v: f32) -> Option<f32> {
    if v == NOT_SET {
        return None;
    }
    Some(v)
}

/// An in-memory `None` as the save's "not set" value.
pub fn opt_to_save(v: Option<f32>) -> f32 {
    v.unwrap_or(NOT_SET)
}

/// The flags of a statistic node: bit 0 bigger is better, bit 1 per pursuit (`docs/specs/career-milestones.md` 2.2).
pub fn stat_flags(stat: StatType) -> u32 {
    u32::from(stat.bigger_is_better()) | (u32::from(stat.per_pursuit()) << 1)
}

/// The 16 KiB block. The bytes are kept as they were read; the typed accessors read them in place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MilestoneBlock {
    raw: Box<[u8; BLOCK_LEN]>,
}

impl MilestoneBlock {
    /// Takes the block from its bytes. Only the length is checked here; use [`MilestoneBlock::is_loadable`] for the
    /// digest, the magic and the version the load checks.
    pub fn from_bytes(block: &[u8]) -> Result<MilestoneBlock, LayoutError> {
        LayoutError::check("milestone block", block, BLOCK_LEN)?;
        Ok(MilestoneBlock { raw: Box::new(array_at(block, 0)) })
    }

    /// The block as bytes, exactly as it was read or last changed.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.raw.to_vec()
    }

    /// Writes the block into `out`, which must hold at least [`BLOCK_LEN`] bytes.
    pub fn write_into(&self, out: &mut [u8]) -> Result<(), LayoutError> {
        LayoutError::check("milestone block", out, BLOCK_LEN)?;
        out[..BLOCK_LEN].copy_from_slice(&self.raw[..]);
        Ok(())
    }

    /// The whole block, read only.
    pub fn bytes(&self) -> &[u8] {
        &self.raw[..]
    }

    /// The magic at `0x10` (`Game` is `0x656D6147`).
    pub fn game_magic(&self) -> u32 {
        u32_at(&self.raw[..], MAGIC_AT)
    }

    /// The version at `0x14`.
    pub fn version(&self) -> u32 {
        u32_at(&self.raw[..], VERSION_AT)
    }

    /// True when the MD5 at `0x00` matches bytes `0x10..0x4000`.
    pub fn digest_ok(&self) -> bool {
        self.raw[..DIGEST_LEN] == md5(&self.raw[DIGEST_FROM..])
    }

    /// Recomputes the MD5 at `0x00`. Call it after changing any byte inside the digest range.
    pub fn refresh_digest(&mut self) {
        let digest = md5(&self.raw[DIGEST_FROM..]);
        self.raw[DIGEST_AT..DIGEST_AT + DIGEST_LEN].copy_from_slice(&digest);
    }

    /// True when the load would accept the block: digest, magic and version (`docs/specs/career-milestones.md` 7).
    pub fn is_loadable(&self) -> bool {
        if !self.digest_ok() {
            return false;
        }
        if self.game_magic() != GAME_MAGIC {
            return false;
        }
        self.version() >= MIN_VERSION
    }
}

/// A statistic node, 16 bytes: `0x00` key, `0x04` current value, `0x08` best value (`-1.0` = not set), `0x0C` flags.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatNode {
    /// The collection key (hash of the statistic's name).
    pub key: u32,
    /// The current value; `None` is the save's `-1.0`.
    pub current: Option<f32>,
    /// The best value; `None` is the save's `-1.0`.
    pub best: Option<f32>,
    /// Bit 0 bigger is better, bit 1 per pursuit; other bits kept as stored.
    pub flags: u32,
}

impl StatNode {
    /// Reads a node from the 16 bytes at the start of `b`.
    pub fn from_bytes(b: &[u8]) -> Result<StatNode, LayoutError> {
        LayoutError::check("statistic node", b, STAT_NODE_LEN)?;
        Ok(StatNode {
            key: u32_at(b, 0x00),
            current: opt_from_save(f32_at(b, 0x04)),
            best: opt_from_save(f32_at(b, 0x08)),
            flags: u32_at(b, 0x0C),
        })
    }

    /// Writes the node into the 16 bytes at the start of `b`.
    pub fn write_into(&self, b: &mut [u8]) -> Result<(), LayoutError> {
        LayoutError::check("statistic node", b, STAT_NODE_LEN)?;
        put_u32(b, 0x00, self.key);
        put_f32(b, 0x04, opt_to_save(self.current));
        put_f32(b, 0x08, opt_to_save(self.best));
        put_u32(b, 0x0C, self.flags);
        Ok(())
    }

    /// A node for `stat` with the in-memory values. The flags are the ones the statistic implies.
    pub fn from_value(key: u32, stat: StatType, value: StatValue) -> StatNode {
        StatNode { key, current: value.current, best: value.best, flags: stat_flags(stat) }
    }

    /// The in-memory values of the node.
    pub fn value(&self) -> StatValue {
        StatValue { current: self.current, best: self.best }
    }
}

/// A milestone entry, 20 bytes: `0x00` type key, `0x04` leaf key, `0x08` state (1 locked, 2 active, 3 at stake,
/// 4 complete), `0x09` flags (bit 0 bigger is better, bit 1 forced), `0x0A` bin, `0x0C` threshold, `0x10` value
/// recorded at completion (`0` before).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MilestoneEntry {
    /// Hash of the milestone's `MilestoneName`. The in-memory milestone does not hold it, so the caller keeps it.
    pub type_key: u32,
    /// The collection key of the milestone leaf.
    pub key: u32,
    /// The state byte as stored. Use [`MilestoneEntry::state`] for the decoded state.
    pub state_byte: u8,
    /// Bit 0 bigger is better, bit 1 forced; other bits kept as stored.
    pub flags: u8,
    /// The Blacklist bin.
    pub bin: u16,
    /// The threshold (field `0x8445AF47`).
    pub threshold: f32,
    /// The value recorded at completion; `0` before. Use [`MilestoneEntry::banked_value`] for the `None` form.
    pub value: f32,
}

impl MilestoneEntry {
    /// Reads an entry from the 20 bytes at the start of `b`.
    pub fn from_bytes(b: &[u8]) -> Result<MilestoneEntry, LayoutError> {
        LayoutError::check("milestone entry", b, MILESTONE_LEN)?;
        Ok(MilestoneEntry {
            type_key: u32_at(b, 0x00),
            key: u32_at(b, 0x04),
            state_byte: b[0x08],
            flags: b[0x09],
            bin: u16_at(b, 0x0A),
            threshold: f32_at(b, 0x0C),
            value: f32_at(b, 0x10),
        })
    }

    /// Writes the entry into the 20 bytes at the start of `b`.
    pub fn write_into(&self, b: &mut [u8]) -> Result<(), LayoutError> {
        LayoutError::check("milestone entry", b, MILESTONE_LEN)?;
        put_u32(b, 0x00, self.type_key);
        put_u32(b, 0x04, self.key);
        b[0x08] = self.state_byte;
        b[0x09] = self.flags;
        put_u16(b, 0x0A, self.bin);
        put_f32(b, 0x0C, self.threshold);
        put_f32(b, 0x10, self.value);
        Ok(())
    }

    /// The decoded state; `None` for a byte other than 1 to 4 (the byte is still kept).
    pub fn state(&self) -> Option<MilestoneState> {
        MilestoneState::from_byte(self.state_byte)
    }

    /// The value as the in-memory form: `None` while nothing is banked (the saved `0`).
    pub fn banked_value(&self) -> Option<f32> {
        if self.value == 0.0 {
            return None;
        }
        Some(self.value)
    }

    /// Copies the in-memory milestone into the entry. Bit 0 of the flags follows the statistic, bit 1 (forced) and the
    /// other bits are kept. A milestone with no value gets `0`.
    pub fn update_from_milestone(&mut self, m: &Milestone, type_key: u32) {
        self.type_key = type_key;
        self.key = m.def.key;
        self.state_byte = m.state.byte();
        self.flags = (self.flags & !1) | u8::from(m.def.stat.bigger_is_better());
        self.bin = u16::from(m.def.bin);
        self.threshold = m.def.threshold;
        self.value = m.value.unwrap_or(0.0);
    }

    /// Rebuilds the in-memory milestone from its data definition `def`. `None` when the key or the bin differ from the
    /// entry, or the state byte is not 1 to 4. The threshold comes from `def`, not from the entry. A complete milestone
    /// keeps its value even when it is `0`.
    pub fn to_milestone(&self, def: MilestoneDef) -> Option<Milestone> {
        let state = self.state()?;
        if def.key != self.key || u16::from(def.bin) != self.bin {
            return None;
        }
        let value = match state {
            MilestoneState::Complete => Some(self.value),
            _ => self.banked_value(),
        };
        Some(Milestone { def, state, value })
    }
}

/// A speed trap entry, 20 bytes: `0x00` flags (bit 0 unlocked, bit 1 armed, bit 2 done), `0x02` bin, `0x04` collection
/// key, `0x08` a number read from field `0x52DC742C` (use not found; kept as stored), `0x0C` threshold in m/s,
/// `0x10` speed in m/s recorded at completion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedTrapEntry {
    /// The flag bits as stored.
    pub flags: u16,
    /// The bin index.
    pub bin: u16,
    /// The collection key of the trap.
    pub key: u32,
    /// The number at `0x08`, kept as stored.
    pub unknown_0x08: u32,
    /// The threshold in m/s.
    pub threshold_mps: f32,
    /// The speed in m/s recorded at completion; `0` before.
    pub recorded_mps: f32,
}

impl SpeedTrapEntry {
    /// Reads an entry from the 20 bytes at the start of `b`.
    pub fn from_bytes(b: &[u8]) -> Result<SpeedTrapEntry, LayoutError> {
        LayoutError::check("speed trap entry", b, SPEED_TRAP_LEN)?;
        Ok(SpeedTrapEntry {
            flags: u16_at(b, 0x00),
            bin: u16_at(b, 0x02),
            key: u32_at(b, 0x04),
            unknown_0x08: u32_at(b, 0x08),
            threshold_mps: f32_at(b, 0x0C),
            recorded_mps: f32_at(b, 0x10),
        })
    }

    /// Writes the entry into the 20 bytes at the start of `b`.
    pub fn write_into(&self, b: &mut [u8]) -> Result<(), LayoutError> {
        LayoutError::check("speed trap entry", b, SPEED_TRAP_LEN)?;
        put_u16(b, 0x00, self.flags);
        put_u16(b, 0x02, self.bin);
        put_u32(b, 0x04, self.key);
        put_u32(b, 0x08, self.unknown_0x08);
        put_f32(b, 0x0C, self.threshold_mps);
        put_f32(b, 0x10, self.recorded_mps);
        Ok(())
    }

    /// Copies the in-memory trap into the entry. `unknown_0x08` is kept.
    pub fn update_from_trap(&mut self, trap: &SpeedTrap) {
        self.flags = trap.flags();
        self.bin = u16::from(trap.bin());
        self.key = trap.key();
        self.threshold_mps = trap.threshold_mps();
        self.recorded_mps = trap.recorded_mps();
    }

    /// The static definition of the trap with this key, bin and trap number `index` (1 to 3), from the data table.
    /// `None` when the bin is not 1 to 15 or the table has no such trap. The flags and the recorded speed are not part
    /// of it: [`SpeedTrap`] has no constructor that takes them (see the report on `MilestoneBlock`).
    pub fn def(&self, index: u8) -> Option<SpeedTrapDef> {
        let bin = u8::try_from(self.bin).ok()?;
        speedtrap::def_for(bin, index, self.key)
    }
}

/// A bin counter record, 8 bytes: `0x00` bin index, `0x02` length (4), then the u32 value. Its low u16 (`0x04`) is the
/// completed milestones plus completed speed traps of the bin, its high u16 (`0x06`) the completed events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinCounter {
    /// The bin index.
    pub bin: u16,
    /// The length field (4 in the saves read so far).
    pub length: u16,
    /// Completed milestones plus completed speed traps of the bin.
    pub completed: u16,
    /// Completed events of the bin (the high half of the value).
    pub events: u16,
}

impl BinCounter {
    /// Reads a record from the 8 bytes at the start of `b`.
    pub fn from_bytes(b: &[u8]) -> Result<BinCounter, LayoutError> {
        LayoutError::check("bin counter", b, BIN_COUNTER_LEN)?;
        Ok(BinCounter {
            bin: u16_at(b, 0x00),
            length: u16_at(b, 0x02),
            completed: u16_at(b, 0x04),
            events: u16_at(b, 0x06),
        })
    }

    /// Writes the record into the 8 bytes at the start of `b`.
    pub fn write_into(&self, b: &mut [u8]) -> Result<(), LayoutError> {
        LayoutError::check("bin counter", b, BIN_COUNTER_LEN)?;
        put_u16(b, 0x00, self.bin);
        put_u16(b, 0x02, self.length);
        put_u16(b, 0x04, self.completed);
        put_u16(b, 0x06, self.events);
        Ok(())
    }

    /// Sets `completed` to the complete milestones of this record's bin plus the done speed traps of that bin
    /// (`docs/specs/career-milestones.md` 3.8). Leaves `events` alone. A bin that is not 1 to 255 is left as it is.
    pub fn set_progress(&mut self, milestones: &MilestoneSet, traps: &[SpeedTrap]) {
        let bin = match u8::try_from(self.bin) {
            Ok(bin) => bin,
            Err(_) => return,
        };
        let milestone_count = u16::try_from(milestones.complete_in_bin(bin)).unwrap_or(u16::MAX);
        self.completed = milestone_count.saturating_add(speedtrap::done_count(traps, bin));
    }
}
