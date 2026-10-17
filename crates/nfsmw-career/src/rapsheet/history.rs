//! The pursuit history block (`profile + 0x90E0`, 3,032 bytes) and how a finished pursuit is recorded into it
//! (`docs/specs/career-rapsheet-2.md` section 2, `docs/specs/career-economy-flows.md` 5.2).
//!
//! Only the fields the recording rule names are typed: the eight statistic sums, the ten best pairs, the eight extra
//! sums, the key of the last car and the top-5 table. [`PursuitHistory::write_into`] patches exactly those and
//! leaves every other byte of the block (including the stored-record count at `+0xA0C`, which the recording rule
//! does not mention) as it is.

use crate::LayoutError;
use crate::bytes::{array_at, put_u32, u32_at};

/// Size of the block.
pub const HISTORY_LEN: usize = 0xBD8;
/// Number of top pursuit entries.
pub const TOP_ENTRIES: usize = 5;
/// Size of one top entry.
pub const ENTRY_LEN: usize = 0x38;
/// Number of statistics with a best pair (the ten statistics).
pub const BEST_PAIRS: usize = 10;

mod at {
    pub const TOP: usize = 0xA2C;
    pub const SUMS: usize = 0xB44;
    pub const BEST: usize = 0xB64;
    pub const EXTRAS: usize = 0xBB4;
    pub const LAST_CAR: usize = 0xBD4;
}

/// One of the five best pursuits, 0x38 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PursuitEntry {
    /// `+0x00`: the case id text `MW-` plus 8 characters, zero terminated (12 bytes).
    pub case_id: [u8; 12],
    /// `+0x0C`: key of the car.
    pub car_key: u32,
    /// `+0x10`: bounty.
    pub bounty: u32,
    /// `+0x14`: duration in ticks of 1/4000 s; 0 marks an unused entry.
    pub ticks: u32,
    /// `+0x18`: cops involved.
    pub cops_involved: u32,
    /// `+0x1C`: cops damaged.
    pub cops_damaged: u32,
    /// `+0x20`: cops destroyed.
    pub cops_destroyed: u32,
    /// `+0x24`: roadblocks dodged.
    pub roadblocks_dodged: u32,
    /// `+0x28`: spike strips dodged.
    pub spikes_dodged: u32,
    /// `+0x2C`: cost to state.
    pub cost_to_state: u32,
    /// `+0x30`: number of distinct infraction kinds.
    pub infraction_kinds: u32,
    /// `+0x34`: helicopters involved.
    pub helicopters: u32,
}

impl PursuitEntry {
    /// An entry is used when its duration is not 0.
    pub const fn is_used(&self) -> bool {
        self.ticks != 0
    }

    fn parse(b: &[u8]) -> PursuitEntry {
        PursuitEntry {
            case_id: array_at(b, 0),
            car_key: u32_at(b, 0x0C),
            bounty: u32_at(b, 0x10),
            ticks: u32_at(b, 0x14),
            cops_involved: u32_at(b, 0x18),
            cops_damaged: u32_at(b, 0x1C),
            cops_destroyed: u32_at(b, 0x20),
            roadblocks_dodged: u32_at(b, 0x24),
            spikes_dodged: u32_at(b, 0x28),
            cost_to_state: u32_at(b, 0x2C),
            infraction_kinds: u32_at(b, 0x30),
            helicopters: u32_at(b, 0x34),
        }
    }

    fn write(&self, b: &mut [u8]) {
        b[..12].copy_from_slice(&self.case_id);
        put_u32(b, 0x0C, self.car_key);
        put_u32(b, 0x10, self.bounty);
        put_u32(b, 0x14, self.ticks);
        put_u32(b, 0x18, self.cops_involved);
        put_u32(b, 0x1C, self.cops_damaged);
        put_u32(b, 0x20, self.cops_destroyed);
        put_u32(b, 0x24, self.roadblocks_dodged);
        put_u32(b, 0x28, self.spikes_dodged);
        put_u32(b, 0x2C, self.cost_to_state);
        put_u32(b, 0x30, self.infraction_kinds);
        put_u32(b, 0x34, self.helicopters);
    }
}

/// The best single-pursuit value of a statistic and the car that made it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BestPair {
    /// Key of the car.
    pub car_key: u32,
    /// The best value.
    pub value: u32,
}

/// The case id text for eight characters: `MW-` plus the characters and a zero byte. The original draws the
/// characters at random, alternating a digit and a capital letter; the generator is the caller, who passes the
/// result to [`PursuitHistory::record`].
pub fn case_id_text(chars: &[u8; 8]) -> [u8; 12] {
    let mut id = [0u8; 12];
    id[..3].copy_from_slice(b"MW-");
    id[3..11].copy_from_slice(chars);
    id
}

/// The numbers of one finished pursuit that the history records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PursuitStats {
    /// Key of the car type of the player car.
    pub car_key: u32,
    /// Bounty: the normal plus the destruction rep points.
    pub bounty: u32,
    /// Duration in ticks of 1/4000 s.
    pub ticks: u32,
    /// Cops involved.
    pub cops_involved: u32,
    /// Cops damaged.
    pub cops_damaged: u32,
    /// Cops destroyed.
    pub cops_destroyed: u32,
    /// Roadblocks dodged.
    pub roadblocks_dodged: u32,
    /// Spike strips dodged.
    pub spikes_dodged: u32,
    /// Cost to state.
    pub cost_to_state: u32,
    /// Distinct infraction kinds.
    pub infraction_kinds: u32,
    /// Helicopters involved.
    pub helicopters: u32,
    /// The eight extra sums of the pursuit: roadblocks deployed, traffic cars hit, spike strips, helicopter spike
    /// strips, patrol cars, support vehicles, property damage value, property damage count.
    pub extras: [u32; 8],
}

impl PursuitStats {
    /// The ten best-pair values in statistic order: duration, cops involved, damaged, destroyed, spikes dodged,
    /// roadblocks dodged, helicopters, cost to state, distinct infraction kinds, bounty.
    pub const fn values(&self) -> [u32; BEST_PAIRS] {
        [
            self.ticks,
            self.cops_involved,
            self.cops_damaged,
            self.cops_destroyed,
            self.spikes_dodged,
            self.roadblocks_dodged,
            self.helicopters,
            self.cost_to_state,
            self.infraction_kinds,
            self.bounty,
        ]
    }

    fn entry(&self, case_id: [u8; 12]) -> PursuitEntry {
        PursuitEntry {
            case_id,
            car_key: self.car_key,
            bounty: self.bounty,
            ticks: self.ticks,
            cops_involved: self.cops_involved,
            cops_damaged: self.cops_damaged,
            cops_destroyed: self.cops_destroyed,
            roadblocks_dodged: self.roadblocks_dodged,
            spikes_dodged: self.spikes_dodged,
            cost_to_state: self.cost_to_state,
            infraction_kinds: self.infraction_kinds,
            helicopters: self.helicopters,
        }
    }
}

/// What recording a pursuit did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recorded {
    /// Busted: only the last car key was written.
    BustedOnlyLastCar,
    /// Not busted: sums and best pairs updated; the pursuit entered the top 5 at this position (0 to 4), or `None`
    /// when no entry had a bounty strictly lower.
    Evaded {
        /// Where the pursuit entered the top 5.
        top_position: Option<usize>,
    },
}

/// The pursuit history of the profile.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PursuitHistory {
    /// `+0xA2C`: the five best pursuits, best bounty first.
    pub top: [PursuitEntry; TOP_ENTRIES],
    /// `+0xB44` (profile `+0x9C24`): the lifetime sums of statistics 0 to 7, over pursuits that were not busted.
    pub sums: [u32; 8],
    /// `+0xB64`: the best pairs for statistics 0 to 9.
    pub best: [BestPair; BEST_PAIRS],
    /// `+0xBB4` (profile `+0x9C94`): the eight extra sums.
    pub extras: [u32; 8],
    /// `+0xBD4` (profile `+0x9CB4`): the car key of the last pursuit, busted or not.
    pub last_car_key: u32,
}

impl PursuitHistory {
    /// Reads the typed fields from a history block of [`HISTORY_LEN`] bytes.
    pub fn from_block(block: &[u8]) -> Result<PursuitHistory, LayoutError> {
        LayoutError::check("pursuit history block", block, HISTORY_LEN)?;
        let mut h = PursuitHistory::default();
        for (i, entry) in h.top.iter_mut().enumerate() {
            *entry = PursuitEntry::parse(&block[at::TOP + i * ENTRY_LEN..]);
        }
        for (i, sum) in h.sums.iter_mut().enumerate() {
            *sum = u32_at(block, at::SUMS + 4 * i);
        }
        for (i, pair) in h.best.iter_mut().enumerate() {
            *pair = BestPair { car_key: u32_at(block, at::BEST + 8 * i), value: u32_at(block, at::BEST + 8 * i + 4) };
        }
        for (i, extra) in h.extras.iter_mut().enumerate() {
            *extra = u32_at(block, at::EXTRAS + 4 * i);
        }
        h.last_car_key = u32_at(block, at::LAST_CAR);
        Ok(h)
    }

    /// Writes the typed fields into a block of [`HISTORY_LEN`] bytes; every other byte is left as it is.
    pub fn write_into(&self, block: &mut [u8]) -> Result<(), LayoutError> {
        LayoutError::check("pursuit history block", block, HISTORY_LEN)?;
        for (i, entry) in self.top.iter().enumerate() {
            entry.write(&mut block[at::TOP + i * ENTRY_LEN..]);
        }
        for (i, &sum) in self.sums.iter().enumerate() {
            put_u32(block, at::SUMS + 4 * i, sum);
        }
        for (i, pair) in self.best.iter().enumerate() {
            put_u32(block, at::BEST + 8 * i, pair.car_key);
            put_u32(block, at::BEST + 8 * i + 4, pair.value);
        }
        for (i, &extra) in self.extras.iter().enumerate() {
            put_u32(block, at::EXTRAS + 4 * i, extra);
        }
        put_u32(block, at::LAST_CAR, self.last_car_key);
        Ok(())
    }

    /// Records one finished player pursuit (career mode), in the order of the spec:
    ///
    /// 1. the last car key is written for every pursuit;
    /// 2. a busted pursuit stops there;
    /// 3. the eight statistic sums and the eight extra sums are added to (saturating, so a signed 32-bit compare
    ///    stays valid);
    /// 4. each best pair is replaced when the new value is **strictly greater** (signed 32-bit);
    /// 5. the top 5: the first entry whose bounty is strictly lower than the new one is the insert position; entries
    ///    from there shift down, the last is lost; a bounty of 0 never enters. `case_id` is used only when the
    ///    entry is stored.
    pub fn record(&mut self, stats: &PursuitStats, busted: bool, case_id: [u8; 12]) -> Recorded {
        self.last_car_key = stats.car_key;
        if busted {
            return Recorded::BustedOnlyLastCar;
        }
        let sums = [
            stats.ticks,
            stats.cops_involved,
            stats.cops_damaged,
            stats.cops_destroyed,
            stats.spikes_dodged,
            stats.roadblocks_dodged,
            stats.helicopters,
            stats.cost_to_state,
        ];
        for (sum, add) in self.sums.iter_mut().zip(sums) {
            *sum = sum.saturating_add(add);
        }
        for (extra, add) in self.extras.iter_mut().zip(stats.extras) {
            *extra = extra.saturating_add(add);
        }
        for (pair, value) in self.best.iter_mut().zip(stats.values()) {
            if (value as i32) > (pair.value as i32) {
                *pair = BestPair { car_key: stats.car_key, value };
            }
        }
        let position = self.top.iter().position(|entry| (entry.bounty as i32) < (stats.bounty as i32));
        if let Some(at) = position {
            self.top.copy_within(at..TOP_ENTRIES - 1, at + 1);
            self.top[at] = stats.entry(case_id);
        }
        Recorded::Evaded { top_position: position }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(bounty: u32, ticks: u32) -> PursuitStats {
        PursuitStats {
            car_key: 0xC0DE,
            bounty,
            ticks,
            cops_involved: 12,
            cops_damaged: 3,
            cops_destroyed: 1,
            roadblocks_dodged: 2,
            spikes_dodged: 4,
            cost_to_state: 50_000,
            infraction_kinds: 3,
            helicopters: 1,
            extras: [1, 2, 3, 4, 5, 6, 7, 8],
        }
    }

    fn id(n: u8) -> [u8; 12] {
        case_id_text(&[b'0' + n, b'A', b'0' + n, b'B', b'0' + n, b'C', b'0' + n, b'D'])
    }

    #[test]
    fn the_block_layout_matches_the_documented_offsets() {
        assert_eq!(HISTORY_LEN, 3032);
        assert_eq!(at::TOP + TOP_ENTRIES * ENTRY_LEN, 0xB44, "the top table ends where the sums start");
        assert_eq!(at::SUMS + 8 * 4, 0xB64);
        assert_eq!(at::BEST + BEST_PAIRS * 8, at::EXTRAS);
        assert_eq!(at::EXTRAS + 8 * 4, at::LAST_CAR);
        // profile offsets: +0x9C24 sums, +0x9C44 best pairs, +0x9C94 extras, +0x9CB4 last car, +0x9B0C top 5
        assert_eq!(0x90E0 + at::SUMS, 0x9C24);
        assert_eq!(0x90E0 + at::BEST, 0x9C44);
        assert_eq!(0x90E0 + at::EXTRAS, 0x9C94);
        assert_eq!(0x90E0 + at::LAST_CAR, 0x9CB4);
        assert_eq!(0x90E0 + at::TOP, 0x9B0C);
    }

    #[test]
    fn a_block_round_trips_the_typed_fields_and_keeps_every_other_byte() {
        let block: Vec<u8> = (0..HISTORY_LEN).map(|i| (i * 13 % 251) as u8).collect();
        let h = PursuitHistory::from_block(&block).unwrap();
        let mut out = block.clone();
        h.write_into(&mut out).unwrap();
        assert_eq!(out, block);
        let mut changed = h.clone();
        changed.sums[3] = 99;
        let mut out2 = block.clone();
        changed.write_into(&mut out2).unwrap();
        let diff: Vec<usize> = (0..HISTORY_LEN).filter(|&i| out2[i] != block[i]).collect();
        assert!(diff.iter().all(|&i| (at::SUMS + 12..at::SUMS + 16).contains(&i)));
    }

    #[test]
    fn a_short_block_is_refused() {
        assert!(PursuitHistory::from_block(&[0; HISTORY_LEN - 1]).is_err());
    }

    #[test]
    fn a_busted_pursuit_only_writes_the_last_car() {
        let mut h = PursuitHistory::default();
        let out = h.record(&stats(5000, 8000), true, id(1));
        assert_eq!(out, Recorded::BustedOnlyLastCar);
        assert_eq!(h, PursuitHistory { last_car_key: 0xC0DE, ..PursuitHistory::default() });
    }

    #[test]
    fn an_evaded_pursuit_updates_sums_pairs_extras_and_the_top() {
        let mut h = PursuitHistory::default();
        let out = h.record(&stats(5000, 8000), false, id(1));
        assert_eq!(out, Recorded::Evaded { top_position: Some(0) });
        assert_eq!(h.sums, [8000, 12, 3, 1, 4, 2, 1, 50_000]);
        assert_eq!(h.extras, [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(h.best[0], BestPair { car_key: 0xC0DE, value: 8000 });
        assert_eq!(h.best[8].value, 3);
        assert_eq!(h.best[9].value, 5000);
        assert_eq!(h.top[0].bounty, 5000);
        assert_eq!(h.top[0].case_id, id(1));
        assert_eq!(&h.top[0].case_id[..3], b"MW-");
        assert_eq!(h.top[0].roadblocks_dodged, 2);
        assert_eq!(h.top[0].spikes_dodged, 4);
        assert!(h.top[0].is_used() && !h.top[1].is_used());
        h.record(&stats(100, 4000), false, id(2));
        assert_eq!(h.sums[0], 12_000);
        assert_eq!(h.extras[7], 16);
    }

    #[test]
    fn best_pairs_replace_only_on_a_strictly_greater_value() {
        let mut h = PursuitHistory::default();
        h.record(&PursuitStats { car_key: 1, ..stats(5000, 8000) }, false, id(1));
        h.record(&PursuitStats { car_key: 2, ..stats(5000, 8000) }, false, id(2));
        assert_eq!(h.best[0].car_key, 1, "an equal value keeps the first car");
        h.record(&PursuitStats { car_key: 3, ..stats(5000, 8001) }, false, id(3));
        assert_eq!(h.best[0], BestPair { car_key: 3, value: 8001 });
    }

    #[test]
    fn the_top_five_inserts_before_the_first_strictly_lower_bounty_and_drops_the_last() {
        let mut h = PursuitHistory::default();
        for (n, bounty) in [(1u8, 300u32), (2, 500), (3, 400), (4, 100), (5, 200)] {
            h.record(&stats(bounty, 4000 * u32::from(n)), false, id(n));
        }
        let bounties: Vec<u32> = h.top.iter().map(|e| e.bounty).collect();
        assert_eq!(bounties, [500, 400, 300, 200, 100]);
        let out = h.record(&stats(250, 4000), false, id(6));
        assert_eq!(out, Recorded::Evaded { top_position: Some(3) });
        let bounties: Vec<u32> = h.top.iter().map(|e| e.bounty).collect();
        assert_eq!(bounties, [500, 400, 300, 250, 200], "the fifth (100) is lost");
        assert_eq!(h.top[3].case_id, id(6));
        assert_eq!(h.top[4].case_id, id(5));
    }

    #[test]
    fn an_equal_or_lower_bounty_is_not_stored_but_still_counts_in_the_sums() {
        let mut h = PursuitHistory::default();
        for n in 1..=5u8 {
            h.record(&stats(1000 * u32::from(n), 4000), false, id(n));
        }
        let before = h.top;
        let out = h.record(&stats(1000, 4000), false, id(9));
        assert_eq!(out, Recorded::Evaded { top_position: None }, "equal to the lowest does not enter");
        assert_eq!(h.top, before);
        assert_eq!(h.sums[0], 24_000);
    }

    #[test]
    fn a_bounty_of_zero_never_enters() {
        let mut h = PursuitHistory::default();
        let out = h.record(&stats(0, 4000), false, id(1));
        assert_eq!(out, Recorded::Evaded { top_position: None });
        assert!(!h.top[0].is_used());
    }

    #[test]
    fn the_sums_saturate_instead_of_wrapping() {
        let mut h = PursuitHistory::default();
        h.sums[0] = u32::MAX - 10;
        h.record(&stats(1, 100), false, id(1));
        assert_eq!(h.sums[0], u32::MAX);
    }
}
