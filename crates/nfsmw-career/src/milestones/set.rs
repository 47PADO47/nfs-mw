//! The milestone set of one profile: the statistics, the milestones and the pursuit summary, with the transitions of
//! `docs/specs/career-milestones.md` sections 3.2 to 3.6 and the milestone part of section 3.8.
//!
//! Unconfirmed choices, all **[unconfirmed]** in the spec:
//! - the summary holds at most [`PURSUIT_SUMMARY_MAX`] keys; past the cap a milestone still goes to at stake, it is
//!   only left off the list;
//! - the summary is kept after a bust (the spec only says it is emptied at the start of a pursuit);
//! - banking is not gated by the play mode (the spec gates only the threshold test); in practice nothing is at stake
//!   in a challenge race, because the test never runs there;
//! - the forced bit (debug only) is not modelled, so the banked value is the statistic's current value.
//! - the bounty goes through [`award_player_bounty`], which has the divisor the economy module uses (1).

use super::stat::{STAT_COUNT, StatType, StatValue, threshold_passed};
use super::state::{Milestone, MilestoneDef, MilestoneState};
use crate::economy::{PursuitResult, award_player_bounty};
use crate::stable::CareerRecord;

/// The most keys the pursuit summary lists (section 3.5).
pub const PURSUIT_SUMMARY_MAX: usize = 32;

/// What one statistic update did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatUpdate {
    /// The best value changed. Only then the spec sends progress and tests the milestones.
    pub improved: bool,
    /// The keys of the milestones that moved from active to at stake in this update, in list order.
    pub at_stake: Vec<u32>,
}

/// A milestone banked at an escape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Banked {
    /// The key of the milestone.
    pub key: u32,
    /// The bounty it pays.
    pub bounty: u32,
}

/// What one player pursuit end did to the milestones.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PursuitEndMilestones {
    /// The milestones banked (an escape only). Pay them with [`pay_banked`].
    pub banked: Vec<Banked>,
    /// The milestones moved from at stake back to active (a bust only).
    pub reverted: usize,
}

/// The milestones of one profile and the statistics they are measured in.
#[derive(Debug, Clone, PartialEq)]
pub struct MilestoneSet {
    milestones: Vec<Milestone>,
    stats: [StatValue; STAT_COUNT],
    summary: Vec<u32>,
}

impl MilestoneSet {
    /// A set for a new career: every milestone locked, every statistic not set, the summary empty (section 7).
    pub fn new(defs: &[MilestoneDef]) -> MilestoneSet {
        MilestoneSet {
            milestones: defs.iter().map(|&d| Milestone::new(d)).collect(),
            stats: [StatValue::default(); STAT_COUNT],
            summary: Vec::new(),
        }
    }

    /// A set from stored parts, for a load: the milestones with their states and values, the statistics in
    /// [`StatType`] order and the pursuit summary, as the save holds them. Nothing is checked against the definitions;
    /// [`MilestoneSet::new`] is the constructor for a new career.
    pub fn from_parts(milestones: Vec<Milestone>, stats: [StatValue; STAT_COUNT], summary: Vec<u32>) -> MilestoneSet {
        MilestoneSet { milestones, stats, summary }
    }

    /// The milestones, in the order they were given.
    pub fn milestones(&self) -> &[Milestone] {
        &self.milestones
    }

    /// The state of one milestone, or `None` for an unknown key.
    pub fn state_of(&self, key: u32) -> Option<MilestoneState> {
        self.milestones.iter().find(|m| m.def.key == key).map(|m| m.state)
    }

    /// The statistics, indexed by [`StatType::index`].
    pub fn stats(&self) -> &[StatValue; STAT_COUNT] {
        &self.stats
    }

    /// The current and best value of one statistic.
    pub fn stat(&self, stat: StatType) -> StatValue {
        self.stats[stat.index()]
    }

    /// The keys of the milestones put at stake in the current pursuit, in order (section 3.5).
    pub fn pursuit_summary(&self) -> &[u32] {
        &self.summary
    }

    /// The player entered `bin`: every locked milestone of it becomes active (section 3.2). Nothing goes back to locked.
    pub fn enter_bin(&mut self, bin: u8) {
        for m in self.milestones.iter_mut().filter(|m| m.def.bin == bin && m.state == MilestoneState::Locked) {
            m.state = MilestoneState::Active;
        }
    }

    /// The career's bin moved from `from` to `to` (16 = no bin yet): every bin from `from - 1` down to `to` is entered
    /// (section 3.2). A move that does not go down unlocks nothing.
    pub fn enter_bins(&mut self, from: u8, to: u8) {
        if to >= from {
            return;
        }
        for bin in to..from {
            self.enter_bin(bin);
        }
    }

    /// Updates a statistic (section 3.3). The value is always set as current. When it is a new best and `free_roam`
    /// is true (play mode 0; a pursuit race does not count), every active milestone of this statistic, in any bin, is
    /// tested against the value and the passing ones move to at stake.
    pub fn update_stat(&mut self, stat: StatType, value: f32, free_roam: bool) -> StatUpdate {
        let improved = self.stats[stat.index()].record(stat.bigger_is_better(), value);
        if !improved || !free_roam {
            return StatUpdate { improved, at_stake: Vec::new() };
        }
        StatUpdate { improved, at_stake: self.test_active(stat, value) }
    }

    /// Adds one to a statistic (1 when it is not set yet), then updates it as [`MilestoneSet::update_stat`] does.
    pub fn increment(&mut self, stat: StatType, free_roam: bool) -> StatUpdate {
        let next = self.stat(stat).current.map_or(1.0, |c| c + 1.0);
        self.update_stat(stat, next, free_roam)
    }

    /// The start of a player pursuit: the per-pursuit statistics are set back to "not set" and the summary is emptied
    /// (sections 3.4 and 3.5).
    pub fn start_pursuit(&mut self) {
        for stat in StatType::ALL.into_iter().filter(|s| s.per_pursuit()) {
            self.stats[stat.index()].clear();
        }
        self.summary.clear();
    }

    /// Abandon a pursuit for the safe house: discard unbanked milestones and pursuit statistics without rewards.
    pub fn abandon_pursuit(&mut self) -> usize {
        let reverted = self.revert_at_stake();
        self.start_pursuit();
        reverted
    }

    /// The end of a player pursuit (section 3.6), taking the result from `economy::pursuit`.
    ///
    /// - Escaped (not busted): the cop manager's escape counter goes to `pursuits_in_a_row` and the evasion time to
    ///   `pursuit_evasion_time`, both through [`MilestoneSet::update_stat`]. The evasion time can pass a
    ///   smaller-is-better threshold, and that milestone is banked in the same call. Then every milestone at stake is
    ///   banked: it becomes complete and its bounty is listed.
    /// - Busted: every milestone at stake goes back to active. Nothing is banked, and the escape statistics are not
    ///   touched.
    ///
    /// `evasion_time_s` and `escapes_in_a_row` are used only for an escape.
    pub fn on_pursuit_end(
        &mut self,
        result: &PursuitResult,
        free_roam: bool,
        evasion_time_s: f32,
        escapes_in_a_row: u32,
    ) -> PursuitEndMilestones {
        if result.busted {
            return PursuitEndMilestones { banked: Vec::new(), reverted: self.revert_at_stake() };
        }
        self.update_stat(StatType::PursuitsInARow, escapes_in_a_row as f32, free_roam);
        self.update_stat(StatType::PursuitEvasionTime, evasion_time_s, free_roam);
        PursuitEndMilestones { banked: self.bank_at_stake(), reverted: 0 }
    }

    /// The number of complete milestones of `bin`. The bin counter of the save adds the speed traps to it (section 3.8).
    pub fn complete_in_bin(&self, bin: u8) -> usize {
        self.milestones.iter().filter(|m| m.def.bin == bin && m.state == MilestoneState::Complete).count()
    }

    /// Tests the active milestones of `stat` against `value` and moves the passing ones to at stake (section 3.5).
    /// Returns their keys and adds them to the summary, up to the cap.
    fn test_active(&mut self, stat: StatType, value: f32) -> Vec<u32> {
        let bigger = stat.bigger_is_better();
        let mut keys = Vec::new();
        for m in self.milestones.iter_mut().filter(|m| m.def.stat == stat && m.state == MilestoneState::Active) {
            if !threshold_passed(bigger, value, m.def.threshold) {
                continue;
            }
            m.state = MilestoneState::AtStake;
            keys.push(m.def.key);
            if self.summary.len() < PURSUIT_SUMMARY_MAX {
                self.summary.push(m.def.key);
            }
        }
        keys
    }

    /// The bust path (section 3.6): at stake goes back to active. Returns how many moved.
    fn revert_at_stake(&mut self) -> usize {
        let mut reverted = 0;
        for m in self.milestones.iter_mut().filter(|m| m.state == MilestoneState::AtStake) {
            m.state = MilestoneState::Active;
            reverted += 1;
        }
        reverted
    }

    /// The bank step (section 3.6, "bank one milestone"): every milestone at stake becomes complete with the current
    /// value of its statistic, and its bounty is listed.
    fn bank_at_stake(&mut self) -> Vec<Banked> {
        let mut banked = Vec::new();
        for m in self.milestones.iter_mut().filter(|m| m.state == MilestoneState::AtStake) {
            m.state = MilestoneState::Complete;
            m.value = Some(self.stats[m.def.stat.index()].current.unwrap_or(m.def.threshold));
            banked.push(Banked { key: m.def.key, bounty: m.def.bounty });
        }
        banked
    }
}

/// Pays banked milestones into the current car's career record (section 3.6: the same award as the scripts'
/// `AwardPlayerBounty`). Returns the car's bounty afterwards. No cash moves.
pub fn pay_banked(record: &mut CareerRecord, banked: &[Banked]) -> u32 {
    for b in banked {
        award_player_bounty(record, b.bounty);
    }
    record.bounty
}

#[cfg(test)]
#[path = "abandon_tests.rs"]
mod abandon_tests;
