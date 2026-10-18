//! The Blacklist: the 15 race bins, their state graphs, the rival gate and the walk down the list
//! (`docs/specs/career-blacklist.md` sections 2 to 5 and 7, `docs/specs/career-scripts.md` sections 3 and 5).
//!
//! Pure logic. The caller owns the world, the race results, the bounty total and the flags, and passes the events
//! in. [`Blacklist`] holds one [`BinRecord`] per bin 1 to 15 and the current bin (`CurrentBin`, the settings
//! `stage` byte). Bin 16 (the prologue, see [`crate::safehouse::resume`]) is not a graph here; the side bins are
//! listed in [`SideBin`] and have no graph.
//!
//! Every event method returns the bins that reached `done` during the call, so the caller can run the native side
//! effects (messages, unlock lists, the screen sequence). Rules the specs do not settle are marked **unconfirmed**
//! next to the code:
//!
//! - Stage advance **[unconfirmed]**: the spec has no rule for how `CurrentBin` moves after a rival is beaten
//!   (`career-blacklist.md` section 5.3, `career-scripts.md` section 5.1). The rule used here: when bin N reaches
//!   `done` while it is the current bin, bin N-1 is entered. Bin 1 stays current.
//! - A bin that is `done` the moment it is entered (no boss races) does not advance the walk again **[unconfirmed]**.
//! - Races, challenges and bounty are counted from the records, not from script tables (section 4.3), so entering a
//!   bin does not reset them **[unconfirmed]**; section 2.3 resets the script copies only.
//! - Bounty going down does not lock an unlocked rival; the spec is silent **[unconfirmed]**.
//! - Event ids are the race event names (`1.5.2`); the bins' SMS, message and unlock ids are data the caller owns.

mod bin;
mod gate;
mod results;
#[cfg(test)]
mod tests;

pub use bin::{BinDef, BinRecord, BinState, SideBin};
pub use gate::{Counters, challenge_button_active, rival_gate_open};
pub use results::{EventOutcome, EventResult, apply_event_result};

use std::collections::BTreeMap;

use crate::safehouse::EventProgress;
use crate::settings::{CareerSettings, FIRST_RIVAL_STAGE, Flags, PROLOGUE_STAGE};

/// The final race, `1.8.1` (Razor's pursuit, the last boss race of bin 1). A first place sets
/// [`Flags::CAREER_COMPLETE`] **[unconfirmed]** (section 7.1 ties the flag to the post-race return).
pub const FINAL_RACE_EVENT: &str = "1.8.1";
/// The last race of the challenge series, `19.8.69`. Any finish sets [`Flags::EVENT_19_8_69_DONE`] (section 7.1).
pub const CHALLENGE_SERIES_LAST_RACE: &str = "19.8.69";

/// Why [`Blacklist::new`] refused its input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BlacklistError {
    /// No definition for this bin. Bins 1 to 15 are all needed.
    #[error("no definition for Blacklist bin {0}")]
    MissingBin(u8),
    /// Two definitions for this bin.
    #[error("two definitions for Blacklist bin {0}")]
    DuplicateBin(u8),
    /// A definition for a number outside 1 to 15.
    #[error("Blacklist bin {0} is outside 1 to 15")]
    BadBin(u8),
    /// A stage outside 1 to 16.
    #[error("career stage {0} is outside 1 to 16")]
    BadStage(u8),
}

/// The Blacklist: one record per bin 1 to 15, the current bin and the total bounty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blacklist {
    bins: BTreeMap<u8, BinRecord>,
    current: u8,
    bounty: u32,
}

impl Blacklist {
    /// A Blacklist at `stage` (16 for a new career, 15 down to 1 afterwards) with no bin entered yet. `defs` must
    /// hold bins 1 to 15 once each. The bounty starts at 0; the caller passes the real total through
    /// [`Blacklist::bounty_changed`].
    pub fn new(defs: Vec<BinDef>, stage: u8) -> Result<Blacklist, BlacklistError> {
        if !(1..=PROLOGUE_STAGE).contains(&stage) {
            return Err(BlacklistError::BadStage(stage));
        }
        let mut bins = BTreeMap::new();
        for def in defs {
            let number = def.number;
            if !(1..=FIRST_RIVAL_STAGE).contains(&number) {
                return Err(BlacklistError::BadBin(number));
            }
            if bins.insert(number, BinRecord::new(def)).is_some() {
                return Err(BlacklistError::DuplicateBin(number));
            }
        }
        if let Some(missing) = (1..=FIRST_RIVAL_STAGE).find(|n| !bins.contains_key(n)) {
            return Err(BlacklistError::MissingBin(missing));
        }
        Ok(Blacklist { bins, current: stage, bounty: 0 })
    }

    /// `CurrentBin`: the bin whose rival is challenged next.
    pub fn current_bin(&self) -> u8 {
        self.current
    }

    /// The record of bin `number`, or `None` outside 1 to 15.
    pub fn bin(&self, number: u8) -> Option<&BinRecord> {
        self.bins.get(&number)
    }

    /// The total bounty last given to the Blacklist.
    pub fn bounty(&self) -> u32 {
        self.bounty
    }

    /// Writes `CurrentBin` into the career settings (the `stage` byte).
    pub fn apply_to_settings(&self, settings: &mut CareerSettings) {
        settings.stage = self.current;
    }

    /// Enters the bins from `CurrentBin - 1` down to `target` (section 5.2): each step sets the current bin and
    /// starts that bin's graph. Returns the bins entered, in order. Does nothing when `target` is not below the
    /// current bin. The first safe-house visit calls this with 15 (section 6 of `career-flow.md`, via
    /// `safehouse::first_visit`).
    pub fn enter_bin(&mut self, target: u8) -> Vec<u8> {
        let bounty = self.bounty;
        let mut entered = Vec::new();
        for number in (target..self.current).rev() {
            let Some(record) = self.bins.get_mut(&number) else {
                break;
            };
            record.enter(bounty);
            self.current = number;
            entered.push(number);
        }
        entered
    }

    /// A race finish in career mode. `race` is the event id and `first` whether the local player placed first.
    /// Records the race as completed (section 4.2), counts it for races won, counts a first-place boss race of an
    /// unlocked rival, and re-checks a locked rival where section 2.6 calls for it.
    ///
    /// **Unconfirmed:** the finish sets the completed flag whatever the placing (section 4.2), but the gate is
    /// re-checked only on a first place (section 2.6). A second-place win therefore counts towards races won and
    /// can leave a bin locked until [`Blacklist::entering_gameplay`] runs. The spec does not say which the original
    /// does.
    pub fn race_finished(&mut self, race: &str, first: bool, flags: &mut Flags) -> Vec<u8> {
        if first && race == FINAL_RACE_EVENT {
            flags.insert(Flags::CAREER_COMPLETE);
        }
        if race == CHALLENGE_SERIES_LAST_RACE {
            flags.insert(Flags::EVENT_19_8_69_DONE);
        }
        let bounty = self.bounty;
        let mut done = Vec::new();
        for record in self.bins.values_mut() {
            let counted = record.record_world_race(race);
            let mut reached_done = counted && first && record.recheck(bounty);
            if first {
                reached_done |= record.record_boss_race(race);
            }
            if reached_done {
                done.push(record.def.number);
            }
        }
        self.advance(&done);
        done
    }

    /// A challenge (milestone or speed trap) of `bin` was passed. Counts it once and re-checks a locked rival
    /// (`NotifyChallengePassed`, section 2.6). Returns the bins that reached `done`.
    pub fn challenge_passed(&mut self, bin: u8, challenge: &str) -> Vec<u8> {
        let bounty = self.bounty;
        let Some(record) = self.bins.get_mut(&bin) else {
            return Vec::new();
        };
        let counted = record.record_challenge(challenge);
        let reached_done = counted && record.recheck(bounty);
        let done: Vec<u8> = reached_done.then_some(bin).into_iter().collect();
        self.advance(&done);
        done
    }

    /// The total bounty changed to `total` (`NotifyPlayerRep`, section 3.2). Re-checks every locked rival.
    pub fn bounty_changed(&mut self, total: u32) -> Vec<u8> {
        self.bounty = total;
        self.recheck_locked()
    }

    /// Entering gameplay re-checks every locked rival (section 2.6).
    pub fn entering_gameplay(&mut self) -> Vec<u8> {
        self.recheck_locked()
    }

    /// The rival challenge button of the selected bin (section 3.3). `boss_race_open` is whether a boss race of that
    /// bin is unlocked and not completed, which the caller reads from the race records.
    pub fn rival_challenge_active(&self, selected: u8, flags: Flags, boss_race_open: bool) -> bool {
        challenge_button_active(self.current, selected, flags, boss_race_open)
    }

    fn recheck_locked(&mut self) -> Vec<u8> {
        let bounty = self.bounty;
        let done: Vec<u8> =
            self.bins.values_mut().filter_map(|record| record.recheck(bounty).then_some(record.def.number)).collect();
        self.advance(&done);
        done
    }

    /// The stage rule **[unconfirmed]** (see the module docs): a bin that reaches `done` while it is the current
    /// bin hands the walk to the bin below it. Bin 1 has none.
    fn advance(&mut self, done: &[u8]) {
        for &number in done {
            if number != self.current || number <= 1 {
                continue;
            }
            self.enter_bin(number - 1);
        }
    }
}

impl EventProgress for Blacklist {
    /// A race node is complete when this Blacklist counts it as won (a free race or a first-place boss race). The
    /// prologue and the other career nodes are not known here; a caller holding them chains its own closure.
    fn is_complete(&self, event: &str) -> bool {
        self.bins
            .values()
            .any(|record| record.races_won.iter().chain(record.rival_races_won.iter()).any(|id| id.as_str() == event))
    }
}
