//! Speed traps of the career: the 39 trap entries in the 15 bins, how a trap is unlocked, armed and completed, what a
//! crossing pays, and what the done traps feed into the bin counter (`docs/specs/career-milestones.md` sections 1.3,
//! 2.4, 3.7 and 3.8; `docs/specs/career-milestones-data.md` table 3; `docs/specs/career-scripts-events.md` section 7.2).
//!
//! Pure logic: no I/O, no Bevy, no dependency on the data crate or on the other milestone files. The caller reads each
//! trap's collection key, bin and trap number from the install and passes them to [`def_for`]. The thresholds and
//! bounties of the shipped data are in [`TRAP_TABLE`], which is also the test oracle.
//!
//! Evidence: the state rules are **[exe]** (read in `speed.exe`, not run in game) and the numbers are **[verified]**.
//! Choices the spec does not settle are marked **[unconfirmed]** next to the code.

/// Number of speed traps in the game: bins 1 to 10 have 3, bins 11 to 14 have 2, bin 15 has 1.
pub const TRAP_COUNT: usize = 39;

/// The factor the spec gives for km/h to m/s (`x 0.27778`).
pub const KMH_TO_MPS: f32 = 0.27778;

/// The statistic a crossing sets: `speedtrap_speed`, km/h, bigger is better, not reset per pursuit.
pub const SPEED_STATISTIC: &str = "speedtrap_speed";

/// The "not set" value of a statistic's current and best value (spec 2.2).
pub const NOT_SET: f32 = -1.0;

/// Threshold used when a trap has none in the data (spec 7.2). **[unconfirmed]** the spec gives the value but not
/// the code path that falls back to it.
pub const DEFAULT_THRESHOLD_KMH: u16 = 88;

/// Bounty used when a trap has none in the data (spec 7.2). **[unconfirmed]** same caveat as the threshold.
pub const DEFAULT_BOUNTY: u32 = 100;

/// Flag bit 0 of a trap entry: the trap is unlocked (its bin was entered).
pub const FLAG_UNLOCKED: u16 = 1;
/// Flag bit 1 of a trap entry: the trap is armed; a fast enough crossing can complete it.
pub const FLAG_ARMED: u16 = 2;
/// Flag bit 2 of a trap entry: the trap is done.
pub const FLAG_DONE: u16 = 4;

/// One row of the data: the bin, the trap number (1 to 3 in the leaf name `speedtrapK`), the threshold in km/h and
/// the bounty. Every trap of a bin pays the same bounty (spec 1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrapRow {
    /// The bin index, 1 to 15.
    pub bin: u8,
    /// The trap number `K` of `speedtraps/bin_NN/speedtrapK`, 1 to 3.
    pub index: u8,
    /// The `ThreshholdSpeed` field in km/h. The trap must be crossed faster than this.
    pub threshold_kmh: u16,
    /// The `Bounty` field, paid to the career when the trap is completed.
    pub bounty: u32,
}

const fn row(bin: u8, index: u8, threshold_kmh: u16, bounty: u32) -> TrapRow {
    TrapRow { bin, index, threshold_kmh, bounty }
}

/// The 39 traps in bin then trap order, from table 3 of the data file (**[verified]**).
pub const TRAP_TABLE: [TrapRow; TRAP_COUNT] = [
    row(1, 1, 320, 136_000),
    row(1, 2, 225, 136_000),
    row(1, 3, 241, 136_000),
    row(2, 1, 220, 114_000),
    row(2, 2, 266, 114_000),
    row(2, 3, 313, 114_000),
    row(3, 1, 255, 94_000),
    row(3, 2, 285, 94_000),
    row(3, 3, 225, 94_000),
    row(4, 1, 265, 76_000),
    row(4, 2, 245, 76_000),
    row(4, 3, 290, 76_000),
    row(5, 1, 300, 60_000),
    row(5, 2, 290, 60_000),
    row(5, 3, 210, 60_000),
    row(6, 1, 173, 46_000),
    row(6, 2, 255, 46_000),
    row(6, 3, 180, 46_000),
    row(7, 1, 180, 34_000),
    row(7, 2, 255, 34_000),
    row(7, 3, 230, 34_000),
    row(8, 1, 214, 24_000),
    row(8, 2, 238, 24_000),
    row(8, 3, 238, 24_000),
    row(9, 1, 225, 16_000),
    row(9, 2, 220, 16_000),
    row(9, 3, 240, 16_000),
    row(10, 1, 175, 8_000),
    row(10, 2, 185, 8_000),
    row(10, 3, 225, 8_000),
    row(11, 1, 190, 6_000),
    row(11, 2, 190, 6_000),
    row(12, 1, 185, 5_500),
    row(12, 2, 195, 5_500),
    row(13, 1, 175, 5_000),
    row(13, 2, 170, 5_000),
    row(14, 1, 145, 4_500),
    row(14, 2, 150, 4_500),
    row(15, 1, 130, 4_000),
];

/// The static definition of one trap: its collection key and its data row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeedTrapDef {
    /// The collection key of the trap leaf (a hash of its name, from the install).
    pub key: u32,
    /// The bin index, 1 to 15.
    pub bin: u8,
    /// The trap number, 1 to 3.
    pub index: u8,
    /// The threshold in km/h.
    pub threshold_kmh: u16,
    /// The bounty paid on completion.
    pub bounty: u32,
}

/// The definition of the trap `index` of `bin` with this collection key, or `None` when the table has no such trap
/// (for example bin 16, or trap 3 of bin 15). The spec builds the array from the data, so an unknown trap is skipped
/// by the caller.
pub fn def_for(bin: u8, index: u8, key: u32) -> Option<SpeedTrapDef> {
    let row = TRAP_TABLE.iter().find(|row| row.bin == bin && row.index == index)?;
    Some(SpeedTrapDef { key, bin, index, threshold_kmh: row.threshold_kmh, bounty: row.bounty })
}

/// The result of driving through a trap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Crossing {
    /// The trap completed. The caller pays `bounty` to the career and sets the `speedtrap_speed` statistic to
    /// `speed_kmh` (through [`keep_best`]).
    Passed {
        /// The speed of the crossing in km/h.
        speed_kmh: f32,
        /// The bounty this completion pays.
        bounty: u32,
    },
    /// Nothing completed: the speed was not above the threshold, or the trap was not active. The spec says the HUD
    /// shows "too slow" for every other case, so an inactive trap gets the same message.
    TooSlow,
}

/// The runtime entry of one trap: the flags, the recorded speed and the static fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedTrap {
    key: u32,
    bin: u8,
    threshold_kmh: u16,
    bounty: u32,
    flags: u16,
    recorded_mps: f32,
}

impl SpeedTrap {
    /// A new career's trap: flags 0 (locked, not armed, not done) and no recorded speed (spec 7).
    pub fn new(def: SpeedTrapDef) -> SpeedTrap {
        SpeedTrap {
            key: def.key,
            bin: def.bin,
            threshold_kmh: def.threshold_kmh,
            bounty: def.bounty,
            flags: 0,
            recorded_mps: 0.0,
        }
    }

    /// A trap as a save holds it: the static fields from `def`, the flag bits and the speed recorded at completion
    /// (`0` before) from the entry. Nothing is checked, so the caller passes the stored values as they are.
    pub fn restore(def: SpeedTrapDef, flags: u16, recorded_mps: f32) -> SpeedTrap {
        SpeedTrap {
            key: def.key,
            bin: def.bin,
            threshold_kmh: def.threshold_kmh,
            bounty: def.bounty,
            flags,
            recorded_mps,
        }
    }

    /// The collection key.
    pub fn key(&self) -> u32 {
        self.key
    }

    /// The bin index.
    pub fn bin(&self) -> u8 {
        self.bin
    }

    /// The threshold in km/h.
    pub fn threshold_kmh(&self) -> u16 {
        self.threshold_kmh
    }

    /// The threshold converted to m/s with [`KMH_TO_MPS`], as the entry stores it.
    pub fn threshold_mps(&self) -> f32 {
        f32::from(self.threshold_kmh) * KMH_TO_MPS
    }

    /// The raw flag bits (bit 0 unlocked, bit 1 armed, bit 2 done).
    pub fn flags(&self) -> u16 {
        self.flags
    }

    /// True when the trap is unlocked (flag bit 0).
    pub fn unlocked(&self) -> bool {
        self.flags & FLAG_UNLOCKED != 0
    }

    /// True when the trap is armed (flag bit 1).
    pub fn armed(&self) -> bool {
        self.flags & FLAG_ARMED != 0
    }

    /// True when the trap is done (flag bit 2).
    pub fn done(&self) -> bool {
        self.flags & FLAG_DONE != 0
    }

    /// The speed in m/s recorded when the trap completed; 0 before.
    pub fn recorded_mps(&self) -> f32 {
        self.recorded_mps
    }

    /// The bounty this trap pays on completion.
    pub fn bounty(&self) -> u32 {
        self.bounty
    }

    /// True when a crossing can complete the trap: armed and not done. **[unconfirmed]** the spec does not show the
    /// body of the active test (`IsActiveSpeedTrap`). This reading matches the completion routine, which needs the
    /// armed bit, and the rule that a done trap cannot complete again.
    pub fn is_active(&self) -> bool {
        self.armed() && !self.done()
    }

    /// Drives through the trap at `speed_kmh`. The trap completes only when it is active and the speed is strictly
    /// above the threshold (spec 3.7 and 7.2). Completion clears the armed bit, sets the done bit, stores the speed
    /// in m/s and returns the bounty. A done trap never completes again, so later crossings change nothing, not even
    /// the recorded speed: the spec keeps the first completion's speed, not the best (see [`keep_best`]).
    pub fn crossing(&mut self, speed_kmh: f32) -> Crossing {
        if !self.is_active() || speed_kmh <= f32::from(self.threshold_kmh) {
            return Crossing::TooSlow;
        }
        self.flags = (self.flags & !FLAG_ARMED) | FLAG_DONE;
        self.recorded_mps = speed_kmh * KMH_TO_MPS;
        Crossing::Passed { speed_kmh, bounty: self.bounty }
    }
}

/// Entering a bin unlocks and arms each of its traps: flags `|= 3` (spec 5.2 of the blacklist file, 3.7). Only the
/// traps of `bin` are touched. A done trap gets the bits too but stays not active, because it is done.
pub fn unlock_bin(traps: &mut [SpeedTrap], bin: u8) {
    for trap in traps.iter_mut().filter(|trap| trap.bin == bin) {
        trap.flags |= FLAG_UNLOCKED | FLAG_ARMED;
    }
}

/// The speed-trap part of the bin counter: how many traps of `bin` are done. The bin counter at `+0x18` adds the
/// completed milestones to this number; the milestones module does that part (spec 3.8).
pub fn done_count(traps: &[SpeedTrap], bin: u8) -> u16 {
    let count = traps.iter().filter(|trap| trap.bin == bin && trap.done()).count();
    u16::try_from(count).unwrap_or(u16::MAX)
}

/// The best value of a bigger-is-better statistic after a new value (spec 3.4, step 2): a new value at or below a best
/// that is set is dropped, anything else becomes the new best. [`NOT_SET`] means no best yet. Applied to
/// `speedtrap_speed` by the caller after each [`Crossing::Passed`].
pub fn keep_best(best: f32, value: f32) -> f32 {
    if best != NOT_SET && value <= best {
        return best;
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_traps() -> Vec<SpeedTrap> {
        TRAP_TABLE
            .iter()
            .map(|row| {
                let key = u32::from(row.bin) * 10 + u32::from(row.index);
                let def = def_for(row.bin, row.index, key).expect("table row");
                SpeedTrap::new(def)
            })
            .collect()
    }

    fn trap_of(traps: &[SpeedTrap], bin: u8, index: u8) -> SpeedTrap {
        let key = u32::from(bin) * 10 + u32::from(index);
        *traps.iter().find(|trap| trap.key() == key).expect("trap")
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-2
    }

    #[test]
    fn table_has_39_traps_in_the_spec_bins() {
        assert_eq!(TRAP_TABLE.len(), TRAP_COUNT);
        let per_bin = |bin: u8| TRAP_TABLE.iter().filter(|row| row.bin == bin).count();
        assert!((1..=10).all(|bin| per_bin(bin) == 3));
        assert!((11..=14).all(|bin| per_bin(bin) == 2));
        assert_eq!(per_bin(15), 1);
        assert_eq!(per_bin(16), 0);
    }

    #[test]
    fn thresholds_match_the_sorted_list_of_spec_7_2() {
        let sorted = |bin: u8| {
            let mut v: Vec<u16> = TRAP_TABLE.iter().filter(|row| row.bin == bin).map(|row| row.threshold_kmh).collect();
            v.sort_unstable();
            v
        };
        assert_eq!(sorted(1), [225, 241, 320]);
        assert_eq!(sorted(6), [173, 180, 255]);
        assert_eq!(sorted(9), [220, 225, 240]);
        assert_eq!(sorted(13), [170, 175]);
        assert_eq!(sorted(15), [130]);
    }

    #[test]
    fn every_trap_of_a_bin_pays_the_same_bounty() {
        let bounty = |bin: u8| TRAP_TABLE.iter().find(|row| row.bin == bin).map(|row| row.bounty);
        assert_eq!(bounty(1), Some(136_000));
        assert_eq!(bounty(10), Some(8_000));
        assert_eq!(bounty(15), Some(4_000));
        let uniform = (1..=15u8)
            .all(|bin| TRAP_TABLE.iter().filter(|row| row.bin == bin).all(|row| Some(row.bounty) == bounty(bin)));
        assert!(uniform);
    }

    #[test]
    fn unknown_traps_have_no_definition() {
        assert_eq!(def_for(16, 1, 0), None);
        assert_eq!(def_for(15, 2, 0), None);
        assert_eq!(def_for(1, 0, 0), None);
        assert!(def_for(1, 3, 7).is_some());
    }

    #[test]
    fn a_new_trap_is_locked_inactive_and_has_no_speed() {
        let trap = SpeedTrap::new(def_for(1, 1, 11).expect("def"));
        assert_eq!(trap.flags(), 0);
        assert!(!trap.unlocked() && !trap.armed() && !trap.done());
        assert!(!trap.is_active());
        assert_eq!(trap.recorded_mps(), 0.0);
    }

    #[test]
    fn unlock_bin_only_touches_its_own_traps() {
        let mut traps = all_traps();
        unlock_bin(&mut traps, 1);
        assert!(traps.iter().filter(|t| t.bin() == 1).all(|t| t.flags() == FLAG_UNLOCKED | FLAG_ARMED));
        assert!(traps.iter().filter(|t| t.bin() == 2).all(|t| t.flags() == 0));
    }

    #[test]
    fn a_crossing_at_the_threshold_is_too_slow() {
        let mut traps = all_traps();
        unlock_bin(&mut traps, 1);
        let mut trap = trap_of(&traps, 1, 1);
        assert_eq!(trap.threshold_kmh(), 320);
        assert_eq!(trap.crossing(320.0), Crossing::TooSlow);
        assert!(trap.is_active());
        assert!(!trap.done());
    }

    #[test]
    fn a_locked_trap_never_completes() {
        let mut trap = SpeedTrap::new(def_for(1, 1, 11).expect("def"));
        assert_eq!(trap.crossing(400.0), Crossing::TooSlow);
        assert_eq!(trap.flags(), 0);
    }

    #[test]
    fn first_fast_crossing_completes_and_pays_the_bounty() {
        let mut traps = all_traps();
        unlock_bin(&mut traps, 1);
        let mut trap = trap_of(&traps, 1, 1);
        assert_eq!(trap.crossing(330.0), Crossing::Passed { speed_kmh: 330.0, bounty: 136_000 });
        assert_eq!(trap.flags(), FLAG_UNLOCKED | FLAG_DONE);
        assert!(trap.done() && !trap.armed());
        assert!(near(trap.recorded_mps(), 330.0 / 3.6));
    }

    #[test]
    fn a_done_trap_never_completes_again_and_keeps_its_first_speed() {
        let mut traps = all_traps();
        unlock_bin(&mut traps, 1);
        let mut trap = trap_of(&traps, 1, 1);
        trap.crossing(330.0);
        let flags = trap.flags();
        let recorded = trap.recorded_mps();
        assert_eq!(trap.crossing(400.0), Crossing::TooSlow);
        assert_eq!(trap.flags(), flags);
        assert_eq!(trap.recorded_mps(), recorded);
    }

    #[test]
    fn restore_keeps_the_stored_flags_and_speed() {
        let def = def_for(1, 1, 11).expect("bin 1 trap 1");
        let trap = SpeedTrap::restore(def, FLAG_UNLOCKED | FLAG_DONE, 91.7);
        assert_eq!(trap.flags(), 5);
        assert!(trap.done() && !trap.armed());
        assert_eq!(trap.recorded_mps(), 91.7);
        assert_eq!(trap.threshold_kmh(), 320);
        assert_eq!(trap.bounty(), 136_000);
    }

    #[test]
    fn threshold_converts_to_mps() {
        let trap = SpeedTrap::new(def_for(1, 1, 11).expect("def"));
        assert!(near(trap.threshold_mps(), 320.0 * KMH_TO_MPS));
    }

    #[test]
    fn done_count_counts_done_traps_of_one_bin() {
        let mut traps = all_traps();
        unlock_bin(&mut traps, 1);
        assert_eq!(done_count(&traps, 1), 0);
        let index = traps.iter().position(|t| t.bin() == 1).expect("bin 1");
        traps[index].crossing(330.0);
        assert_eq!(done_count(&traps, 1), 1);
        assert_eq!(done_count(&traps, 2), 0);
    }

    #[test]
    fn keep_best_keeps_the_higher_speed() {
        assert_eq!(keep_best(NOT_SET, 200.0), 200.0);
        assert_eq!(keep_best(200.0, 150.0), 200.0);
        assert_eq!(keep_best(200.0, 200.0), 200.0);
        assert_eq!(keep_best(200.0, 250.0), 250.0);
    }
}
