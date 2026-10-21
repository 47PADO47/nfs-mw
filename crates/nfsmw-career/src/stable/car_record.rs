//! `FECarRecord`: one of the 200 slots of the owned-car table (`docs/formats/career-state.md` section 4,
//! `docs/formats/career-save.md` section 4.1). Twenty bytes.

use crate::bytes::{array_at, put_u32, u32_at};

/// Size of one record in the save.
pub const CAR_RECORD_LEN: usize = 0x14;
/// The handle of a free slot.
pub const FREE_HANDLE: u32 = 0xFFFF_FFFF;
/// The "none" value of the two index bytes.
pub const NO_INDEX: u8 = 0xFF;

/// A car of the stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarRecord {
    /// The car handle; [`FREE_HANDLE`] marks a free slot. A small slot number for ordinary cars, a name hash
    /// for the preset rides.
    pub handle: u32,
    /// `+0x04`: the car front-end collection key (price, unlock rank, class).
    pub frontend_key: u32,
    /// `+0x08`: the vehicle collection key (the physics car).
    pub vehicle_key: u32,
    /// `+0x0C`: low 16 bits are the state, high 16 bits the class mask.
    pub flags: u32,
    /// `+0x10`: index of the customization record, `0xFF` = none.
    pub customization_index: u8,
    /// `+0x11`: index of the career record, `0xFF` = none.
    pub career_index: u8,
    /// `+0x12..+0x14`: not described by the specs, kept as they are.
    pub tail: [u8; 2],
}

impl CarRecord {
    /// The low-half flag for a quick race custom car, the cars of the My Cars garage (`career-car-lot.md` section 7).
    pub const FLAG_QUICK_RACE: u32 = 0x4;
    /// The low-half flag for an owned career car (tab "Career"; `career-state.md` calls it the owned bit).
    pub const FLAG_OWNED: u32 = 0x2;
    /// The class mask of the high half; the owned cars carry all four bits.
    pub const CLASS_MASK: u32 = 0xF_0000;

    /// An empty slot: `handle = 0xFFFFFFFF`, keys 0, both index bytes `0xFF`.
    pub const fn free() -> CarRecord {
        CarRecord {
            handle: FREE_HANDLE,
            frontend_key: 0,
            vehicle_key: 0,
            flags: 0,
            customization_index: NO_INDEX,
            career_index: NO_INDEX,
            tail: [0; 2],
        }
    }

    /// Reads a record from at least [`CAR_RECORD_LEN`] bytes.
    pub(crate) fn parse(b: &[u8]) -> CarRecord {
        CarRecord {
            handle: u32_at(b, 0),
            frontend_key: u32_at(b, 4),
            vehicle_key: u32_at(b, 8),
            flags: u32_at(b, 0xC),
            customization_index: b[0x10],
            career_index: b[0x11],
            tail: array_at(b, 0x12),
        }
    }

    /// Writes the record into [`CAR_RECORD_LEN`] bytes.
    pub(crate) fn write(&self, b: &mut [u8]) {
        put_u32(b, 0, self.handle);
        put_u32(b, 4, self.frontend_key);
        put_u32(b, 8, self.vehicle_key);
        put_u32(b, 0xC, self.flags);
        b[0x10] = self.customization_index;
        b[0x11] = self.career_index;
        b[0x12..0x14].copy_from_slice(&self.tail);
    }

    /// True for a free slot.
    pub const fn is_free(&self) -> bool {
        self.handle == FREE_HANDLE
    }

    /// True when the owned bit is set.
    pub const fn is_owned(&self) -> bool {
        self.flags & CarRecord::FLAG_OWNED != 0
    }

    /// True for a quick race custom car: not a free slot, with the quick race flag.
    pub const fn is_quick_race(&self) -> bool {
        !self.is_free() && self.flags & CarRecord::FLAG_QUICK_RACE != 0
    }

    /// True when any bit of the class mask is set.
    pub const fn has_class_mask(&self) -> bool {
        self.flags & CarRecord::CLASS_MASK != 0
    }

    /// The index of the car career record, `None` for `0xFF`.
    pub const fn career_record_index(&self) -> Option<usize> {
        match self.career_index {
            NO_INDEX => None,
            i => Some(i as usize),
        }
    }

    /// True when the record counts in the stable totals: a handle, a class mask, the owned bit and a career
    /// record index (`career-economy.md` 1.2). Whether that index points at a used record is checked by the
    /// stable. The specs say "a record counts only if both are set" about the mask and the owned bit; the mask
    /// is read as "any bit of `0xF0000`" (the weaker reading, unconfirmed).
    pub const fn counts(&self) -> bool {
        !self.is_free() && self.has_class_mask() && self.is_owned() && self.career_index != NO_INDEX
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(handle: u32, career: u8) -> CarRecord {
        CarRecord { handle, flags: 0xF_0002, career_index: career, ..CarRecord::free() }
    }

    #[test]
    fn a_quick_race_record_has_the_quick_race_flag_and_a_handle() {
        let custom = CarRecord { handle: 3, flags: 0x4, ..CarRecord::free() };
        assert!(custom.is_quick_race());
        assert!(!owned(3, 0).is_quick_race(), "the owned bit alone is a career car");
        assert!(!CarRecord { flags: 0x4, ..CarRecord::free() }.is_quick_race(), "a free slot is not a car");
    }

    #[test]
    fn a_record_round_trips_and_keeps_its_unknown_bytes() {
        let bytes: Vec<u8> = (1..=0x14).collect();
        let r = CarRecord::parse(&bytes);
        assert_eq!(r.handle, 0x0403_0201);
        assert_eq!(r.flags, 0x100F_0E0D);
        assert_eq!(r.customization_index, 0x11);
        assert_eq!(r.career_index, 0x12);
        assert_eq!(r.tail, [0x13, 0x14]);
        let mut out = vec![0u8; CAR_RECORD_LEN];
        r.write(&mut out);
        assert_eq!(out, bytes);
    }

    #[test]
    fn a_free_slot_is_all_ones_where_the_spec_says() {
        let mut out = vec![0u8; CAR_RECORD_LEN];
        CarRecord::free().write(&mut out);
        assert_eq!(&out[..4], &[0xFF; 4]);
        assert_eq!(out[0x10], 0xFF);
        assert_eq!(out[0x11], 0xFF);
        assert!(CarRecord::free().is_free());
        assert!(!CarRecord::free().counts());
    }

    #[test]
    fn a_record_counts_only_with_handle_class_owned_bit_and_index() {
        assert!(owned(0x12345678, 0).counts());
        assert!(!CarRecord { flags: 0x2, ..owned(7, 0) }.counts(), "no class mask");
        assert!(!CarRecord { flags: 0xF_0000, ..owned(7, 0) }.counts(), "not owned");
        assert!(!owned(7, NO_INDEX).counts(), "no career record");
        assert!(!CarRecord { handle: FREE_HANDLE, ..owned(7, 0) }.counts(), "free slot");
        assert_eq!(owned(7, 3).career_record_index(), Some(3));
        assert_eq!(owned(7, NO_INDEX).career_record_index(), None);
    }
}
