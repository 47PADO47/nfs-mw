//! One bin's state graph (`docs/specs/career-blacklist.md` sections 2.1 to 2.6, `docs/specs/career-scripts.md`
//! section 3.2) and the side bins (section 1, notes).
//!
//! The scripts have seven states. The transient four (`initial`, `unlock_races`, `introduce_rival`, `check_rival`)
//! run inside the call that enters the bin, so only the three states a bin rests in are stored:
//! [`BinState::RivalLocked`], [`BinState::RivalUnlocked`] and [`BinState::Done`]. The `initial` wait of bin 16
//! (its entry activity) is not modelled here.

use super::gate::{Counters, rival_gate_open};

/// The requirements and race lists of one bin. The caller reads them from the `race_bin` collections of
/// `gameplay.bin` (table in `career-blacklist.md` section 1) and passes them in. This crate does not depend on
/// `nfsmw-data`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinDef {
    /// The bin number, 1 to 15 (`BinIndex`).
    pub number: u8,
    /// Event ids of the free races (`WorldRaces`). Counted for races won.
    pub races: Vec<String>,
    /// `RequiredRacesWon`.
    pub races_needed: u32,
    /// `RequiredChallenges`.
    pub challenges_needed: u32,
    /// `RequiredBounty`, in the units of the stable's bounty total.
    pub bounty_needed: u32,
    /// Event ids of the rival's races (`BossRaces`). The rival is beaten when every one is won with first place.
    /// Design choice: the task named one `rival_event`, but the spec lists several boss races per bin (bin 1 has
    /// six, `career-blacklist.md` section 1), so this is a list.
    pub rival_events: Vec<String>,
}

/// A resting state of a bin's graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinState {
    /// `rival_locked`: a counter is short. The gate is checked again on the events of section 2.6.
    RivalLocked,
    /// `rival_unlocked`: the gate is open. The boss races are unlocked and the rival can be challenged.
    RivalUnlocked,
    /// `done`: every boss race is won. The free races stay available.
    Done,
}

impl BinState {
    /// The script state name.
    pub const fn name(self) -> &'static str {
        match self {
            BinState::RivalLocked => "rival_locked",
            BinState::RivalUnlocked => "rival_unlocked",
            BinState::Done => "done",
        }
    }

    /// The lookup2 hash of the state name, as the save stores it (the first u32 of the bin's saved table). Checked
    /// in both inspected saves (`career-blacklist.md` section 2.6).
    pub const fn save_hash(self) -> u32 {
        match self {
            BinState::RivalLocked => 0x2ABB_39C2,
            BinState::RivalUnlocked => 0x76F2_1CB8,
            BinState::Done => 0x3295_6993,
        }
    }
}

/// One bin's definition, its counters and its graph state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinRecord {
    /// The bin's requirements and race lists.
    pub def: BinDef,
    /// The resting state. `None` until the bin is entered: a bin that was never entered has no graph
    /// (`career-scripts.md` section 3.1).
    pub state: Option<BinState>,
    /// Free races of the bin completed in career mode, one entry per race.
    pub races_won: Vec<String>,
    /// Boss races won with first place, one entry per race.
    pub rival_races_won: Vec<String>,
    /// Challenges passed (milestones and speed traps), one entry per challenge.
    pub challenges: Vec<String>,
}

impl BinRecord {
    /// A bin that has not been entered, with no counted races, challenges or boss wins.
    pub fn new(def: BinDef) -> BinRecord {
        BinRecord { def, state: None, races_won: Vec::new(), rival_races_won: Vec::new(), challenges: Vec::new() }
    }

    /// The two counters the gate compares (section 2.5). Recomputed from the records (section 4.3).
    pub fn counters(&self) -> Counters {
        Counters {
            races_won: u32::try_from(self.races_won.len()).unwrap_or(u32::MAX),
            challenges: u32::try_from(self.challenges.len()).unwrap_or(u32::MAX),
        }
    }

    /// The entry sequence: `check_rival` picks the resting state. A bin with no boss races is `done` at once.
    /// Returns true when the bin reached `done`.
    pub(super) fn enter(&mut self, bounty: u32) -> bool {
        self.state = Some(self.gate_state(bounty));
        self.settle()
    }

    /// A re-check of a locked rival (section 2.6). Does nothing in any other state. Returns true when the bin
    /// reached `done`.
    pub(super) fn recheck(&mut self, bounty: u32) -> bool {
        if self.state != Some(BinState::RivalLocked) {
            return false;
        }
        self.state = Some(self.gate_state(bounty));
        self.settle()
    }

    /// Counts a free race once. Returns true when it was newly counted.
    pub(super) fn record_world_race(&mut self, race: &str) -> bool {
        if !has(&self.def.races, race) || has(&self.races_won, race) {
            return false;
        }
        self.races_won.push(race.to_owned());
        true
    }

    /// Counts a first-place boss race while the rival is unlocked (section 2.6). Returns true when the bin reached
    /// `done`.
    pub(super) fn record_boss_race(&mut self, race: &str) -> bool {
        let open = self.state == Some(BinState::RivalUnlocked);
        if !open || !has(&self.def.rival_events, race) || has(&self.rival_races_won, race) {
            return false;
        }
        self.rival_races_won.push(race.to_owned());
        self.settle()
    }

    /// Counts a passed challenge once. Returns true when it was newly counted.
    pub(super) fn record_challenge(&mut self, challenge: &str) -> bool {
        if has(&self.challenges, challenge) {
            return false;
        }
        self.challenges.push(challenge.to_owned());
        true
    }

    fn gate_state(&self, bounty: u32) -> BinState {
        if rival_gate_open(&self.def, self.counters(), bounty) {
            return BinState::RivalUnlocked;
        }
        BinState::RivalLocked
    }

    /// `rival_unlocked` goes to `done` once every boss race is won (section 2.6). Returns true on that change.
    fn settle(&mut self) -> bool {
        let all_won = self.rival_races_won.len() >= self.def.rival_events.len();
        if self.state != Some(BinState::RivalUnlocked) || !all_won {
            return false;
        }
        self.state = Some(BinState::Done);
        true
    }
}

fn has(list: &[String], id: &str) -> bool {
    list.iter().any(|item| item.as_str() == id)
}

/// The side bins (`career-blacklist.md` section 1, notes). They are not on the `CurrentBin` chain, so they have no
/// graph here. Their requirements are not in the spec's table, so only their numbers and the test-bin status are
/// modelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideBin {
    /// Bin 18, `race_bin_challenge_tollbooth`: tollbooth races.
    Tollbooth,
    /// Bin 19, `race_bin_challenge`: the challenge series. Its last race is [`super::CHALLENGE_SERIES_LAST_RACE`].
    ChallengeSeries,
    /// Bin 20, `race_bin_opm`: a developer test bin that nothing enters (`career-scripts.md` section 5.2). What
    /// "OPM" stands for is unconfirmed.
    Opm,
    /// Bin 21, `race_bin_collectors`: three races.
    Collectors,
}

impl SideBin {
    /// The bin number.
    pub const fn number(self) -> u8 {
        match self {
            SideBin::Tollbooth => 18,
            SideBin::ChallengeSeries => 19,
            SideBin::Opm => 20,
            SideBin::Collectors => 21,
        }
    }

    /// True for OPM, the developer test bin that the game never enters.
    pub const fn is_test_bin(self) -> bool {
        matches!(self, SideBin::Opm)
    }
}
