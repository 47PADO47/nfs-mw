//! `FECareerRecord`: the per-car career numbers (`docs/formats/career-state.md` section 4,
//! `docs/specs/career-economy.md` section 1.3). Fifty-six bytes. The impound rules on it are in
//! [`super::impound`].

use crate::bytes::{array_at, counters_at, f32_at, put_counters, put_f32, put_u16, put_u32, u16_at, u32_at};

/// Size of one record in the save.
pub const CAREER_RECORD_LEN: usize = 0x38;
/// The own-index byte of an unused record.
pub const UNUSED_INDEX: u8 = 0xFF;
/// The strike limit of a new record.
pub const DEFAULT_STRIKE_LIMIT: i8 = 3;

/// The impound state byte (record `+4`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpoundState {
    /// 0: the car is free to drive.
    Free,
    /// 2: impounded, the strike limit was reached.
    LimitReached,
    /// 3: impounded, the cash could not pay the fine and was confiscated.
    CashConfiscated,
    /// 4: the countdown is over; a release fee or marker frees the car.
    Releasable,
    /// Any other byte (not produced by the game).
    Other(u8),
}

impl ImpoundState {
    /// The byte stored in the record.
    pub const fn byte(self) -> u8 {
        match self {
            ImpoundState::Free => 0,
            ImpoundState::LimitReached => 2,
            ImpoundState::CashConfiscated => 3,
            ImpoundState::Releasable => 4,
            ImpoundState::Other(b) => b,
        }
    }

    /// Decodes the state byte.
    pub const fn from_byte(b: u8) -> ImpoundState {
        match b {
            0 => ImpoundState::Free,
            2 => ImpoundState::LimitReached,
            3 => ImpoundState::CashConfiscated,
            4 => ImpoundState::Releasable,
            other => ImpoundState::Other(other),
        }
    }
}

/// The career numbers of one owned car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CareerRecord {
    /// `+0x00`: the record own index, `0xFF` = unused.
    pub index: u8,
    /// `+0x01`: not described by the specs, kept as it is.
    pub unknown_01: u8,
    /// `+0x02`: the impound strike limit ("max busted"), 3 at the start, at most 5.
    pub strike_limit: i8,
    /// `+0x03`: strikes so far, never below 0 when the rules touch it.
    pub strikes: i8,
    /// `+0x04`: the impound state byte; see [`CareerRecord::impound_state`].
    pub state: u8,
    /// `+0x05`: the release countdown in events, 5 when impounded.
    pub countdown: u8,
    /// `+0x06`: the strike decay counter (evades while free).
    pub decay: u8,
    /// `+0x07`: not described by the specs, kept as it is.
    pub unknown_07: u8,
    /// `+0x08..+0x0C`: cleared by the initializer, not described, kept as they are.
    pub unknown_08: [u8; 4],
    /// `+0x0C`: the car heat.
    pub heat: f32,
    /// `+0x10`: the bounty earned in this car.
    pub bounty: u32,
    /// `+0x14`: pursuits evaded.
    pub evaded: u16,
    /// `+0x16`: times busted.
    pub busted: u16,
    /// `+0x18`: infractions not yet paid, per kind index.
    pub unserved: [u16; 8],
    /// `+0x28`: infractions paid, per kind index.
    pub served: [u16; 8],
}

impl CareerRecord {
    /// A record as the add-car routine creates it: own index `index`, limit 3, everything else 0.
    pub const fn new(index: u8) -> CareerRecord {
        CareerRecord {
            index,
            unknown_01: 0,
            strike_limit: DEFAULT_STRIKE_LIMIT,
            strikes: 0,
            state: 0,
            countdown: 0,
            decay: 0,
            unknown_07: 0,
            unknown_08: [0; 4],
            heat: 0.0,
            bounty: 0,
            evaded: 0,
            busted: 0,
            unserved: [0; 8],
            served: [0; 8],
        }
    }

    /// An unused record: index `0xFF`, every other byte zero. A loaded unused record keeps whatever bytes the
    /// save had.
    pub const fn unused() -> CareerRecord {
        let mut r = CareerRecord::new(UNUSED_INDEX);
        r.strike_limit = 0;
        r
    }

    /// Reads a record from at least [`CAREER_RECORD_LEN`] bytes.
    pub(crate) fn parse(b: &[u8]) -> CareerRecord {
        CareerRecord {
            index: b[0],
            unknown_01: b[1],
            strike_limit: b[2] as i8,
            strikes: b[3] as i8,
            state: b[4],
            countdown: b[5],
            decay: b[6],
            unknown_07: b[7],
            unknown_08: array_at(b, 8),
            heat: f32_at(b, 0xC),
            bounty: u32_at(b, 0x10),
            evaded: u16_at(b, 0x14),
            busted: u16_at(b, 0x16),
            unserved: counters_at(b, 0x18),
            served: counters_at(b, 0x28),
        }
    }

    /// Writes the record into [`CAREER_RECORD_LEN`] bytes.
    pub(crate) fn write(&self, b: &mut [u8]) {
        b[0] = self.index;
        b[1] = self.unknown_01;
        b[2] = self.strike_limit as u8;
        b[3] = self.strikes as u8;
        b[4] = self.state;
        b[5] = self.countdown;
        b[6] = self.decay;
        b[7] = self.unknown_07;
        b[8..12].copy_from_slice(&self.unknown_08);
        put_f32(b, 0xC, self.heat);
        put_u32(b, 0x10, self.bounty);
        put_u16(b, 0x14, self.evaded);
        put_u16(b, 0x16, self.busted);
        put_counters(b, 0x18, &self.unserved);
        put_counters(b, 0x28, &self.served);
    }

    /// True for an unused record (own index `0xFF`).
    pub const fn is_unused(&self) -> bool {
        self.index == UNUSED_INDEX
    }

    /// The decoded impound state.
    pub const fn impound_state(&self) -> ImpoundState {
        ImpoundState::from_byte(self.state)
    }

    /// Sets the impound state.
    pub fn set_impound_state(&mut self, state: ImpoundState) {
        self.state = state.byte();
    }

    /// True while the car can be driven (state 0).
    pub const fn is_free_to_drive(&self) -> bool {
        self.state == 0
    }

    /// Total unserved infractions.
    pub fn unserved_total(&self) -> u32 {
        self.unserved.iter().map(|&c| u32::from(c)).sum()
    }

    /// Total served infractions.
    pub fn served_total(&self) -> u32 {
        self.served.iter().map(|&c| u32::from(c)).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_record_matches_the_add_car_routine() {
        let r = CareerRecord::new(4);
        assert_eq!((r.index, r.strike_limit, r.strikes, r.state, r.countdown, r.decay), (4, 3, 0, 0, 0, 0));
        assert_eq!((r.bounty, r.evaded, r.busted, r.heat), (0, 0, 0, 0.0));
        assert!(r.is_free_to_drive() && !r.is_unused());
        assert!(CareerRecord::unused().is_unused());
    }

    #[test]
    fn a_record_round_trips_every_byte() {
        let bytes: Vec<u8> = (0..CAREER_RECORD_LEN as u8).map(|i| i.wrapping_mul(7).wrapping_add(3)).collect();
        let r = CareerRecord::parse(&bytes);
        let mut out = vec![0u8; CAREER_RECORD_LEN];
        r.write(&mut out);
        assert_eq!(out, bytes);
    }

    #[test]
    fn fields_sit_at_the_documented_offsets() {
        let mut b = vec![0u8; CAREER_RECORD_LEN];
        b[0] = 2;
        b[2] = 3;
        b[3] = 0xFF; // s8 -1
        b[4] = 4;
        b[5] = 5;
        b[6] = 1;
        b[0xC..0x10].copy_from_slice(&1.5f32.to_le_bytes());
        b[0x10..0x14].copy_from_slice(&43_116u32.to_le_bytes());
        b[0x14..0x16].copy_from_slice(&111u16.to_le_bytes());
        b[0x16..0x18].copy_from_slice(&9u16.to_le_bytes());
        b[0x18 + 6] = 4; // unserved[3]
        b[0x28 + 14] = 7; // served[7]
        let r = CareerRecord::parse(&b);
        assert_eq!((r.index, r.strike_limit, r.strikes, r.state, r.countdown, r.decay), (2, 3, -1, 4, 5, 1));
        assert_eq!((r.heat, r.bounty, r.evaded, r.busted), (1.5, 43_116, 111, 9));
        assert_eq!(r.unserved[3], 4);
        assert_eq!(r.served[7], 7);
        assert_eq!((r.unserved_total(), r.served_total()), (4, 7));
        assert_eq!(r.impound_state(), ImpoundState::Releasable);
    }

    #[test]
    fn impound_states_decode_and_encode() {
        for b in 0..=255u8 {
            assert_eq!(ImpoundState::from_byte(b).byte(), b);
        }
        assert_eq!(ImpoundState::from_byte(2), ImpoundState::LimitReached);
        assert_eq!(ImpoundState::from_byte(3), ImpoundState::CashConfiscated);
        assert_eq!(ImpoundState::from_byte(1), ImpoundState::Other(1));
    }
}
