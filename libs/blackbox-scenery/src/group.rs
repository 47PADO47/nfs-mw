//! `SceneryGroups` (chunk `0x34109` of the track bundle): scenery instances that are switched on and off together,
//! such as the race gates. Spec: `docs/specs/career-regions.md` section 1.2.

use std::collections::HashMap;

use blackbox_chunk::ids;

use crate::{Error, Result};

/// Offsets inside a group record.
const HASH_AT: usize = 0x08;
const NUMBER_AT: usize = 0x0C;
const COUNT_AT: usize = 0x0E;
const FLAG_AT: usize = 0x10;
const DRIVE_THROUGH_AT: usize = 0x11;
const INDEXES_AT: usize = 0x14;

/// One scenery group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneryGroup {
    /// `bStringHash` of the group's name, for example `BARRIER_LOCKED_10`.
    pub hash: u32,
    /// The number the collision instances store. Group 0 means no group.
    pub number: u16,
    /// The byte at `+0x10`: 1 for most groups, 0 for three of them.
    pub flag: u8,
    /// The byte at `+0x11`: set for the race drive-through groups, clear for the gates.
    pub drive_through: bool,
    /// Global scenery override indexes of the group's instances (`ScenerySection::override_hooks` joins them to
    /// instances).
    pub override_indexes: Vec<u16>,
}

/// Bytes a record with `count` override indexes takes up, padded to 4.
pub(crate) const fn record_size(count: usize) -> usize {
    (2 * count + 0x17) & !3
}

/// Every group of a `SceneryGroups` payload (the chunk's bytes, header excluded). Records follow each other.
pub fn parse_groups(payload: &[u8]) -> Result<Vec<SceneryGroup>> {
    let mut groups = Vec::new();
    let mut at = 0;
    while at < payload.len() {
        let (group, size) = parse_group(&payload[at..]).map_err(|detail| Error::Groups { offset: at, detail })?;
        groups.push(group);
        at += size;
    }
    Ok(groups)
}

/// The group at the start of `record`, and the bytes it takes up.
fn parse_group(record: &[u8]) -> std::result::Result<(SceneryGroup, usize), String> {
    let Some(header) = record.get(..INDEXES_AT) else {
        return Err(format!("{} bytes left, a group header needs {INDEXES_AT}", record.len()));
    };
    let count = usize::from(u16::from_le_bytes([header[COUNT_AT], header[COUNT_AT + 1]]));
    let size = record_size(count);
    let Some(record) = record.get(..size) else {
        return Err(format!("{} bytes left, a group with {count} indexes needs {size}", record.len()));
    };
    let indexes = &record[INDEXES_AT..INDEXES_AT + 2 * count];
    let group = SceneryGroup {
        hash: u32::from_le_bytes([record[HASH_AT], record[HASH_AT + 1], record[HASH_AT + 2], record[HASH_AT + 3]]),
        number: u16::from_le_bytes([record[NUMBER_AT], record[NUMBER_AT + 1]]),
        flag: record[FLAG_AT],
        drive_through: record[DRIVE_THROUGH_AT] != 0,
        override_indexes: indexes.as_chunks::<2>().0.iter().map(|b| u16::from_le_bytes(*b)).collect(),
    };
    Ok((group, size))
}

/// Every scenery group in a track bundle (`TRACKS/L2RA.BUN`). A bundle without the chunk has none.
pub fn read_scenery_groups(bundle: &[u8]) -> Result<Vec<SceneryGroup>> {
    let Some(chunk) = blackbox_chunk::find(bundle, ids::SCENERY_GROUPS) else {
        return Ok(Vec::new());
    };
    parse_groups(chunk.payload)
}

/// The group number of each global override index that a group lists. Groups with number 0 are left out. The
/// shipped bundle lists every override index in one group only.
pub fn override_group_numbers(groups: &[SceneryGroup]) -> HashMap<u16, u16> {
    groups
        .iter()
        .filter(|g| g.number != 0)
        .flat_map(|g| g.override_indexes.iter().map(move |&index| (index, g.number)))
        .collect()
}
