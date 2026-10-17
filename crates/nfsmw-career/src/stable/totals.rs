//! Stable totals and the per-record walkers (`docs/formats/career-state.md` section 4, `career-economy.md`
//! section 1.2): every total is "sold history plus the sum over the live records", computed on demand and never
//! stored.

use super::{CAREER_SLOTS, CareerRecord, Stable};
use crate::economy::{FINES, InfractionMask};

/// Which of the two infraction counter sets to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfractionSet {
    /// Infractions of busted or evaded pursuits that are not paid yet.
    Unserved,
    /// Infractions that were paid.
    Served,
}

impl InfractionSet {
    fn of_record(self, record: &CareerRecord) -> &[u16; 8] {
        match self {
            InfractionSet::Unserved => &record.unserved,
            InfractionSet::Served => &record.served,
        }
    }
}

impl Stable {
    /// The indices of the live career records: the car record has a handle, a class mask, the owned bit and a
    /// career record index, and that index points at a used record. Each index appears once, in car slot order.
    pub fn live_career_indices(&self) -> Vec<usize> {
        let mut seen = [false; CAREER_SLOTS];
        let mut out = Vec::new();
        for car in self.cars.iter().filter(|car| car.counts()) {
            let Some(index) = car.career_record_index() else { continue };
            let Some(record) = self.careers.get(index) else { continue };
            if record.is_unused() || seen[index] {
                continue;
            }
            seen[index] = true;
            out.push(index);
        }
        out
    }

    /// The live career records.
    pub fn live_records(&self) -> impl Iterator<Item = &CareerRecord> {
        self.live_career_indices().into_iter().map(|i| &self.careers[i])
    }

    /// Total bounty: the sold base plus the bounty of every live record. In `u64` so the sum cannot wrap.
    pub fn total_bounty(&self) -> u64 {
        u64::from(self.sold.bounty) + self.live_records().map(|r| u64::from(r.bounty)).sum::<u64>()
    }

    /// Total pursuits evaded.
    pub fn total_evaded(&self) -> u32 {
        u32::from(self.sold.evaded) + self.live_records().map(|r| u32::from(r.evaded)).sum::<u32>()
    }

    /// Total times busted.
    pub fn total_busted(&self) -> u32 {
        u32::from(self.sold.busted) + self.live_records().map(|r| u32::from(r.busted)).sum::<u32>()
    }

    /// The per-kind totals of one infraction set (sold history plus live records).
    pub fn infraction_totals(&self, set: InfractionSet) -> [u32; 8] {
        let mut totals = [0u32; 8];
        let sold = match set {
            InfractionSet::Unserved => &self.sold.unserved,
            InfractionSet::Served => &self.sold.served,
        };
        for (total, &count) in totals.iter_mut().zip(sold) {
            *total = u32::from(count);
        }
        for record in self.live_records() {
            for (total, &count) in totals.iter_mut().zip(set.of_record(record)) {
                *total += u32::from(count);
            }
        }
        totals
    }

    /// Infractions of the kinds in `mask` in one set: the sum of their per-kind totals.
    pub fn infraction_count(&self, mask: InfractionMask, set: InfractionSet) -> u32 {
        let totals = self.infraction_totals(set);
        mask.kinds().map(|kind| totals[kind.index()]).sum()
    }

    /// All infractions in one set.
    pub fn infraction_set_total(&self, set: InfractionSet) -> u32 {
        self.infraction_totals(set).iter().sum()
    }

    /// Total infractions, both sets (rap sheet statistic 8).
    pub fn total_infractions(&self) -> u32 {
        self.infraction_set_total(InfractionSet::Unserved) + self.infraction_set_total(InfractionSet::Served)
    }

    /// The fine due for all unserved infractions of the stable (the rap sheet "fines due").
    pub fn fines_due(&self) -> u64 {
        let totals = self.infraction_totals(InfractionSet::Unserved);
        totals.iter().zip(FINES).map(|(&count, fine)| u64::from(count) * u64::from(fine)).sum()
    }

    /// Number of live cars that can be driven (impound state 0).
    pub fn free_car_count(&self) -> usize {
        self.live_records().filter(|r| r.is_free_to_drive()).count()
    }

    /// The evade ticker for player 0 over every live record. Returns how many cars became
    /// releasable on this tick.
    pub fn evade_tick(&mut self) -> usize {
        let mut released = 0;
        for index in self.live_career_indices() {
            if self.careers[index].evade_tick() {
                released += 1;
            }
        }
        released
    }

    /// The race-win ticker over every live record. Returns how many cars became releasable; for
    /// each the original fires script event `0x78`, which is SMS 120 (the phone module names it `IMPOUND_RELEASE_SMS`).
    pub fn win_tick(&mut self) -> usize {
        let mut released = 0;
        for index in self.live_career_indices() {
            if self.careers[index].win_tick() {
                released += 1;
            }
        }
        released
    }
}
