//! Original per-car customization storage; see `docs/specs/garage-records.md`.

use super::{CUSTOMIZATION_RECORD_LEN, CUSTOMIZATION_SLOTS, Stable};
use crate::LayoutError;
use crate::bytes::{f32_at, put_f32, put_u16, put_u32, u16_at, u32_at};

pub const INSTALLED_PART_SLOTS: usize = 139;

/// A lossless record. Unknown fields stay in `bytes` when known fields change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomizationRecord {
    bytes: [u8; CUSTOMIZATION_RECORD_LEN],
}

impl Default for CustomizationRecord {
    fn default() -> Self {
        let mut record = Self { bytes: [0; CUSTOMIZATION_RECORD_LEN] };
        for slot in 0..INSTALLED_PART_SLOTS {
            record.set_part(slot, None);
        }
        record
    }
}

impl CustomizationRecord {
    pub fn parse(bytes: &[u8]) -> Result<Self, LayoutError> {
        LayoutError::check("customization record", bytes, CUSTOMIZATION_RECORD_LEN)?;
        let mut record = Self::default();
        record.bytes.copy_from_slice(&bytes[..CUSTOMIZATION_RECORD_LEN]);
        Ok(record)
    }

    pub fn as_bytes(&self) -> &[u8; CUSTOMIZATION_RECORD_LEN] {
        &self.bytes
    }

    /// A nonnegative database index; an absent or out-of-range slot has none.
    pub fn part(&self, slot: usize) -> Option<u16> {
        if slot >= INSTALLED_PART_SLOTS {
            return None;
        }
        let value = u16_at(&self.bytes, slot * 2);
        (value <= i16::MAX as u16).then_some(value)
    }

    /// Returns false rather than truncating an unrepresentable database index.
    pub fn set_part(&mut self, slot: usize, index: Option<u16>) -> bool {
        if slot >= INSTALLED_PART_SLOTS || index.is_some_and(|v| v > i16::MAX as u16) {
            return false;
        }
        put_u16(&mut self.bytes, slot * 2, index.unwrap_or(u16::MAX));
        true
    }

    pub fn performance(&self) -> [i32; 7] {
        std::array::from_fn(|i| u32_at(&self.bytes, 280 + 4 * i) as i32)
    }

    pub fn set_performance(&mut self, levels: [i32; 7], junkman: u32) {
        for (i, value) in levels.into_iter().enumerate() {
            put_u32(&mut self.bytes, 280 + 4 * i, value as u32);
        }
        put_u32(&mut self.bytes, 308, junkman);
    }

    pub fn junkman(&self) -> u32 {
        u32_at(&self.bytes, 308)
    }

    pub fn tuning(&self) -> [[f32; 7]; 3] {
        std::array::from_fn(|p| std::array::from_fn(|i| f32_at(&self.bytes, 312 + 28 * p + 4 * i)))
    }

    pub fn active_tuning(&self) -> i32 {
        u32_at(&self.bytes, 396) as i32
    }

    pub fn set_tuning(&mut self, profiles: [[f32; 7]; 3], active: i32) {
        for (p, profile) in profiles.into_iter().enumerate() {
            for (i, value) in profile.into_iter().enumerate() {
                put_f32(&mut self.bytes, 312 + 28 * p + 4 * i, value);
            }
        }
        put_u32(&mut self.bytes, 396, active as u32);
    }
}

impl Stable {
    pub fn customization_for_handle(&self, handle: u32) -> Option<CustomizationRecord> {
        let index = usize::from(self.car_by_handle(handle)?.customization_index);
        if index >= CUSTOMIZATION_SLOTS {
            return None;
        }
        let at = index * CUSTOMIZATION_RECORD_LEN;
        CustomizationRecord::parse(self.customization.get(at..at + CUSTOMIZATION_RECORD_LEN)?).ok()
    }

    /// Updates only the record already assigned to this car. Allocation belongs to the car lot.
    pub fn set_customization_for_handle(&mut self, handle: u32, record: &CustomizationRecord) -> bool {
        let Some(car) = self.car_by_handle(handle) else { return false };
        let index = usize::from(car.customization_index);
        if index >= CUSTOMIZATION_SLOTS {
            return false;
        }
        let at = index * CUSTOMIZATION_RECORD_LEN;
        let Some(bytes) = self.customization.get_mut(at..at + CUSTOMIZATION_RECORD_LEN) else { return false };
        bytes.copy_from_slice(record.as_bytes());
        true
    }
}

#[cfg(test)]
#[path = "customization_tests.rs"]
mod tests;
