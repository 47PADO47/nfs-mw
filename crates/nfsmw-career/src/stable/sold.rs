//! The sold-history totals at the end of the stable: the numbers of cars that are no longer in the garage
//! (`docs/formats/career-state.md` section 4, tail at stable `+0x8CA0`).

use super::career_record::CareerRecord;
use crate::bytes::{counters_at, put_counters, put_u16, put_u32, u16_at, u32_at};

/// Size of the tail in the save.
pub const SOLD_HISTORY_LEN: usize = 0x28;

/// Bounty, evaded and busted counts and both infraction sets of every car that was sold or removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SoldHistory {
    /// `+0x8CA0`: bounty of sold cars.
    pub bounty: u32,
    /// `+0x8CA4`: pursuits evaded in sold cars.
    pub evaded: u16,
    /// `+0x8CA6`: times busted in sold cars.
    pub busted: u16,
    /// `+0x8CA8`: infractions unserved, sold cars.
    pub unserved: [u16; 8],
    /// `+0x8CB8`: infractions served, sold cars.
    pub served: [u16; 8],
}

impl SoldHistory {
    /// Reads the tail from at least [`SOLD_HISTORY_LEN`] bytes.
    pub(crate) fn parse(b: &[u8]) -> SoldHistory {
        SoldHistory {
            bounty: u32_at(b, 0),
            evaded: u16_at(b, 4),
            busted: u16_at(b, 6),
            unserved: counters_at(b, 8),
            served: counters_at(b, 0x18),
        }
    }

    /// Writes the tail into [`SOLD_HISTORY_LEN`] bytes.
    pub(crate) fn write(&self, b: &mut [u8]) {
        put_u32(b, 0, self.bounty);
        put_u16(b, 4, self.evaded);
        put_u16(b, 6, self.busted);
        put_counters(b, 8, &self.unserved);
        put_counters(b, 0x18, &self.served);
    }

    /// Moves the numbers of a car that leaves the garage into the history: its bounty, evaded and busted
    /// counts and both infraction sets (`career-economy-flows.md` section 7, "Sell a car"). Sums saturate.
    pub fn absorb(&mut self, record: &CareerRecord) {
        self.bounty = self.bounty.saturating_add(record.bounty);
        self.evaded = self.evaded.saturating_add(record.evaded);
        self.busted = self.busted.saturating_add(record.busted);
        for (sum, add) in self.unserved.iter_mut().zip(record.unserved) {
            *sum = sum.saturating_add(add);
        }
        for (sum, add) in self.served.iter_mut().zip(record.served) {
            *sum = sum.saturating_add(add);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tail_round_trips_at_the_documented_offsets() {
        let mut b = vec![0u8; SOLD_HISTORY_LEN];
        b[..4].copy_from_slice(&43_116_691u32.to_le_bytes());
        b[4..6].copy_from_slice(&111u16.to_le_bytes());
        b[8 + 2] = 3; // unserved[1]
        b[0x18 + 14] = 9; // served[7]
        let h = SoldHistory::parse(&b);
        assert_eq!((h.bounty, h.evaded, h.busted), (43_116_691, 111, 0));
        assert_eq!((h.unserved[1], h.served[7]), (3, 9));
        let mut out = vec![0u8; SOLD_HISTORY_LEN];
        h.write(&mut out);
        assert_eq!(out, b);
    }

    #[test]
    fn a_sold_car_moves_into_the_history() {
        let mut car = CareerRecord::new(0);
        car.bounty = 1000;
        car.evaded = 4;
        car.busted = 2;
        car.unserved[0] = 1;
        car.served[3] = 6;
        let mut h = SoldHistory { bounty: u32::MAX - 500, ..SoldHistory::default() };
        h.absorb(&car);
        h.absorb(&car);
        assert_eq!(h.bounty, u32::MAX, "saturates");
        assert_eq!((h.evaded, h.busted, h.unserved[0], h.served[3]), (8, 4, 2, 12));
    }
}
