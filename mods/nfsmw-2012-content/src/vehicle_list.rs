//! The 2012 vehicle list (`VEHICLES/VEHICLELIST.BNDL`): which vehicles exist and which entries are liveries of another.
//!
//! The list is one resource of type `VehicleList`. Its main block starts with a 0x20-byte header (the count of entries at
//! +0x04, the offset of the first entry at +0x0C) and holds one 0x70-byte record per entry. Only the first two fields of
//! a record are read here: the vehicle id (+0x00) and the `LiveryID` (+0x04). A vehicle with liveries has one entry per
//! livery, all with the same vehicle id (`docs/formats/chameleon-customization.md`).

use chameleon_bundle::{Bundle, ResourceKind};

/// Bytes of one entry.
const STRIDE: usize = 0x70;
/// More entries than this is not a vehicle list (the install has 110).
const MAX_ENTRIES: usize = 4096;

/// One entry of the vehicle list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListEntry {
    pub vehicle: u32,
    /// The `LiveryID`: 0 or the vehicle id for the vehicle itself, another number for a livery.
    pub livery: u32,
}

impl ListEntry {
    /// The number in the name of the entry's bundle, `VEH_<n>_HI.BNDL`: the livery id when there is one, else the vehicle
    /// id **[verified]**: the 93 bundles of the install are exactly the entries that name a file.
    pub fn file(&self) -> u32 {
        match self.livery {
            0 => self.vehicle,
            livery => livery,
        }
    }

    /// Whether this entry is the vehicle itself, not one of its liveries.
    pub fn is_base(&self) -> bool {
        self.livery == 0 || self.livery == self.vehicle
    }
}

/// The entries of the vehicle list in the bundle `data`. `None` when it is not a vehicle list.
pub fn read(data: &[u8]) -> Option<Vec<ListEntry>> {
    let bundle = Bundle::parse(data).ok()?;
    let kind = ResourceKind::VehicleList.id();
    let index = bundle.entries().iter().position(|entry| entry.kind == kind)?;
    entries(&bundle.resource(index).ok()?.blocks[0])
}

/// The entries of the list resource's main block.
fn entries(block: &[u8]) -> Option<Vec<ListEntry>> {
    let word = |at: usize| -> Option<u32> {
        let bytes = block.get(at..at.checked_add(4)?)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    };
    let count = word(4)? as usize;
    let first = word(12)? as usize;
    let end = first.checked_add(count.checked_mul(STRIDE)?)?;
    if count > MAX_ENTRIES || end > block.len() {
        return None;
    }
    (0..count)
        .map(|n| {
            let at = first + n * STRIDE;
            Some(ListEntry { vehicle: word(at)?, livery: word(at + 4)? })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A list block with the given `(vehicle, livery)` entries.
    fn block(entries: &[(u32, u32)]) -> Vec<u8> {
        let mut data = vec![0u8; 0x20];
        data[4..8].copy_from_slice(&(entries.len() as u32).to_le_bytes());
        data[12..16].copy_from_slice(&0x20u32.to_le_bytes());
        for &(vehicle, livery) in entries {
            let mut record = vec![0u8; STRIDE];
            record[0..4].copy_from_slice(&vehicle.to_le_bytes());
            record[4..8].copy_from_slice(&livery.to_le_bytes());
            data.extend(record);
        }
        data
    }

    #[test]
    fn entries_are_read_in_order() {
        let found = entries(&block(&[(10, 0), (20, 20), (20, 99)])).unwrap();
        assert_eq!(found.len(), 3);
        assert_eq!(found[2], ListEntry { vehicle: 20, livery: 99 });
    }

    #[test]
    fn the_bundle_number_is_the_livery_id_or_the_vehicle_id() {
        assert_eq!(ListEntry { vehicle: 10, livery: 0 }.file(), 10);
        assert_eq!(ListEntry { vehicle: 20, livery: 20 }.file(), 20);
        assert_eq!(ListEntry { vehicle: 20, livery: 99 }.file(), 99);
    }

    #[test]
    fn only_an_entry_without_a_livery_id_or_with_its_own_is_the_vehicle() {
        assert!(ListEntry { vehicle: 10, livery: 0 }.is_base());
        assert!(ListEntry { vehicle: 20, livery: 20 }.is_base());
        assert!(!ListEntry { vehicle: 20, livery: 99 }.is_base());
    }

    #[test]
    fn a_block_that_is_cut_short_or_claims_too_much_is_refused() {
        let mut data = block(&[(10, 0), (20, 0)]);
        data.truncate(data.len() - 1);
        assert!(entries(&data).is_none());
        let mut huge = block(&[(10, 0)]);
        huge[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(entries(&huge).is_none());
        assert!(entries(&[0u8; 8]).is_none());
    }
}
