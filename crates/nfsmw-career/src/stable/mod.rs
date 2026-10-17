//! The car stable (`FEPlayerCarDB`): 200 car records, 75 customization records, 25 career records and the
//! sold-history tail, 36,040 bytes (`docs/formats/career-state.md` section 4, `docs/formats/career-save.md`
//! section 4).
//!
//! The customization records are not decoded yet and travel as raw bytes. The totals ([`Stable::total_bounty`]
//! and friends) and the tickers live in [`totals`] and [`impound`].

mod car_record;
mod career_record;
pub mod impound;
mod sold;
mod totals;

pub use car_record::{CAR_RECORD_LEN, CarRecord, FREE_HANDLE, NO_INDEX};
pub use career_record::{CAREER_RECORD_LEN, CareerRecord, DEFAULT_STRIKE_LIMIT, ImpoundState, UNUSED_INDEX};
pub use sold::{SOLD_HISTORY_LEN, SoldHistory};
pub use totals::InfractionSet;

use crate::LayoutError;

/// Car record slots.
pub const CAR_SLOTS: usize = 200;
/// Customization record slots.
pub const CUSTOMIZATION_SLOTS: usize = 75;
/// Size of one customization record.
pub const CUSTOMIZATION_RECORD_LEN: usize = 0x198;
/// Career record slots.
pub const CAREER_SLOTS: usize = 25;
/// Size of the whole stable in the save.
pub const STABLE_LEN: usize = 0x8CC8;

const CUSTOMIZATION_AT: usize = CAR_SLOTS * CAR_RECORD_LEN;
const CUSTOMIZATION_LEN: usize = CUSTOMIZATION_SLOTS * CUSTOMIZATION_RECORD_LEN;
const CAREER_AT: usize = CUSTOMIZATION_AT + CUSTOMIZATION_LEN;
const SOLD_AT: usize = CAREER_AT + CAREER_SLOTS * CAREER_RECORD_LEN;

/// The player car stable.
#[derive(Debug, Clone, PartialEq)]
pub struct Stable {
    /// The 200 car record slots.
    pub cars: Vec<CarRecord>,
    /// The 75 customization records, raw (`75 * 0x198` bytes).
    pub customization: Vec<u8>,
    /// The 25 career record slots.
    pub careers: Vec<CareerRecord>,
    /// The sold-history tail.
    pub sold: SoldHistory,
}

impl Stable {
    /// A stable with no cars: 200 free car slots, 25 unused career records, zeroed customization and history.
    pub fn empty() -> Stable {
        Stable {
            cars: vec![CarRecord::free(); CAR_SLOTS],
            customization: vec![0; CUSTOMIZATION_LEN],
            careers: vec![CareerRecord::unused(); CAREER_SLOTS],
            sold: SoldHistory::default(),
        }
    }

    /// Reads the stable from [`STABLE_LEN`] bytes (extra bytes are ignored).
    pub fn parse(bytes: &[u8]) -> Result<Stable, LayoutError> {
        LayoutError::check("car stable", bytes, STABLE_LEN)?;
        Ok(Stable {
            cars: (0..CAR_SLOTS).map(|i| CarRecord::parse(&bytes[i * CAR_RECORD_LEN..])).collect(),
            customization: bytes[CUSTOMIZATION_AT..CAREER_AT].to_vec(),
            careers: (0..CAREER_SLOTS)
                .map(|i| CareerRecord::parse(&bytes[CAREER_AT + i * CAREER_RECORD_LEN..]))
                .collect(),
            sold: SoldHistory::parse(&bytes[SOLD_AT..]),
        })
    }

    /// The stable as [`STABLE_LEN`] bytes. Slots missing from the vectors are written as free or unused, and
    /// extra ones are dropped, so a hand-built stable cannot produce a block of the wrong size.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = vec![0u8; STABLE_LEN];
        for i in 0..CAR_SLOTS {
            let record = self.cars.get(i).copied().unwrap_or(CarRecord::free());
            record.write(&mut out[i * CAR_RECORD_LEN..]);
        }
        let n = self.customization.len().min(CUSTOMIZATION_LEN);
        out[CUSTOMIZATION_AT..CUSTOMIZATION_AT + n].copy_from_slice(&self.customization[..n]);
        for i in 0..CAREER_SLOTS {
            let record = self.careers.get(i).copied().unwrap_or(CareerRecord::unused());
            record.write(&mut out[CAREER_AT + i * CAREER_RECORD_LEN..]);
        }
        self.sold.write(&mut out[SOLD_AT..]);
        out
    }

    /// The car record with this handle (free slots never match).
    pub fn car_by_handle(&self, handle: u32) -> Option<&CarRecord> {
        if handle == FREE_HANDLE {
            return None;
        }
        self.cars.iter().find(|car| car.handle == handle)
    }

    /// The index of the career record of the car with this handle: the car must exist and point at a used
    /// career record.
    pub fn career_index_for_handle(&self, handle: u32) -> Option<usize> {
        let index = self.car_by_handle(handle)?.career_record_index()?;
        match self.careers.get(index) {
            Some(record) if !record.is_unused() => Some(index),
            _ => None,
        }
    }

    /// The career record of the car with this handle.
    pub fn career_for_handle(&self, handle: u32) -> Option<&CareerRecord> {
        self.careers.get(self.career_index_for_handle(handle)?)
    }

    /// The career record of the car with this handle, mutable.
    pub fn career_for_handle_mut(&mut self, handle: u32) -> Option<&mut CareerRecord> {
        let index = self.career_index_for_handle(handle)?;
        self.careers.get_mut(index)
    }
}

#[cfg(test)]
mod tests;
