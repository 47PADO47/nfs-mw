//! Milestone tests. The thresholds and bounties are the numbers of `docs/specs/career-milestones-data.md` (table 2 for
//! bins 1 to 4). The keys are invented test keys, not collection hashes.

use super::stat::threshold_passed;
use super::*;
use crate::economy::PursuitResult;
use crate::stable::CareerRecord;

fn def(key: u32, bin: u8, stat: StatType, threshold: f32, bounty: u32) -> MilestoneDef {
    MilestoneDef { key, bin, stat, threshold, bounty }
}

/// Bin 1 cops tagged 35 (220000), bin 1 evade time 120 s (220000), bin 1 pursuit length 780 s (220000), bin 2 pursuit
/// length 540 s (180000), bin 3 cops tagged 32 (140000), bin 4 infractions 6 (100000).
fn defs() -> Vec<MilestoneDef> {
    vec![
        def(101, 1, StatType::CopsDamaged, 35.0, 220_000),
        def(102, 1, StatType::PursuitEvasionTime, 120.0, 220_000),
        def(103, 1, StatType::PursuitLength, 780.0, 220_000),
        def(201, 2, StatType::PursuitLength, 540.0, 180_000),
        def(301, 3, StatType::CopsDamaged, 32.0, 140_000),
        def(401, 4, StatType::TotalInfractions, 6.0, 100_000),
    ]
}

fn ended(busted: bool) -> PursuitResult {
    PursuitResult { busted, ..Default::default() }
}

// --- states -------------------------------------------------------------------------------------------------------

#[test]
fn state_bytes_are_the_save_bytes() {
    assert_eq!(MilestoneState::Locked.byte(), 1);
    assert_eq!(MilestoneState::Active.byte(), 2);
    assert_eq!(MilestoneState::AtStake.byte(), 3);
    assert_eq!(MilestoneState::Complete.byte(), 4);
    for b in 1..=4u8 {
        assert_eq!(MilestoneState::from_byte(b).unwrap().byte(), b);
    }
    assert_eq!(MilestoneState::from_byte(0), None);
    assert_eq!(MilestoneState::from_byte(5), None);
}

#[test]
fn a_new_set_is_locked_and_no_statistic_is_set() {
    let set = MilestoneSet::new(&defs());
    assert!(set.milestones().iter().all(|m| m.state == MilestoneState::Locked && m.value.is_none()));
    for stat in StatType::ALL {
        assert_eq!(set.stat(stat), StatValue::default(), "{}", stat.name());
    }
    assert!(set.pursuit_summary().is_empty());
}

// --- statistic table (table 1) ------------------------------------------------------------------------------------

#[test]
fn the_thirty_statistics_are_in_table_order_with_distinct_names() {
    assert_eq!(StatType::ALL.len(), STAT_COUNT);
    for (i, stat) in StatType::ALL.iter().enumerate() {
        assert_eq!(stat.index(), i);
    }
    let mut names: Vec<&str> = StatType::ALL.iter().map(|s| s.name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 30);
    assert_eq!(StatType::Call911.name(), "911_calls");
}

#[test]
fn only_pursuit_evasion_time_is_smaller_is_better() {
    let smaller: Vec<StatType> = StatType::ALL.into_iter().filter(|s| !s.bigger_is_better()).collect();
    assert_eq!(smaller, vec![StatType::PursuitEvasionTime]);
}

#[test]
fn nine_statistics_are_per_pursuit_and_they_are_the_right_ones() {
    let per: Vec<StatType> = StatType::ALL.into_iter().filter(|s| s.per_pursuit()).collect();
    assert_eq!(per.len(), 9);
    assert!(per.contains(&StatType::CopsDestroyedInPursuit));
    assert!(!per.contains(&StatType::CopsDestroyed));
    assert!(!per.contains(&StatType::CostToState));
    assert!(!per.contains(&StatType::PursuitsInARow));
}

#[test]
fn eighteen_statistics_have_a_producer() {
    assert_eq!(StatType::ALL.iter().filter(|s| s.has_producer()).count(), 18);
    assert!(!StatType::OpponentsDamaged.has_producer());
    assert!(StatType::HeatMeter.has_producer());
}

// --- best value and threshold rules (3.3, 3.5) --------------------------------------------------------------------

#[test]
fn a_bigger_is_better_value_keeps_the_highest_best() {
    let mut v = StatValue::default();
    assert!(v.record(true, 500.0));
    assert!(!v.record(true, 400.0));
    assert_eq!((v.current, v.best), (Some(400.0), Some(500.0)), "current is always set, best only goes up");
    assert!(v.record(true, 600.0));
    assert_eq!(v.best, Some(600.0));
}

#[test]
fn a_smaller_is_better_value_keeps_the_lowest_best() {
    let mut v = StatValue::default();
    assert!(v.record(false, 150.0));
    assert!(v.record(false, 100.0));
    assert!(!v.record(false, 130.0));
    assert_eq!((v.current, v.best), (Some(130.0), Some(100.0)));
    v.clear();
    assert_eq!(v, StatValue::default());
}

#[test]
fn the_threshold_passes_at_the_number_and_beyond_in_each_direction() {
    assert!(threshold_passed(true, 35.0, 35.0));
    assert!(!threshold_passed(true, 34.0, 35.0));
    assert!(threshold_passed(false, 120.0, 120.0));
    assert!(!threshold_passed(false, 121.0, 120.0));
}

#[test]
fn an_active_milestone_passes_at_its_threshold_in_free_roam() {
    let mut set = MilestoneSet::new(&defs());
    set.enter_bin(1);
    let low = set.update_stat(StatType::CopsDamaged, 34.0, true);
    assert_eq!(low, StatUpdate { improved: true, at_stake: vec![] });
    assert_eq!(set.state_of(101), Some(MilestoneState::Active));
    let at = set.update_stat(StatType::CopsDamaged, 35.0, true);
    assert_eq!(at.at_stake, vec![101]);
    assert_eq!(set.state_of(101), Some(MilestoneState::AtStake));
}

#[test]
fn a_smaller_is_better_milestone_passes_at_or_below_its_threshold() {
    let mut set = MilestoneSet::new(&defs());
    set.enter_bin(1);
    assert!(set.update_stat(StatType::PursuitEvasionTime, 150.0, true).at_stake.is_empty());
    let up = set.update_stat(StatType::PursuitEvasionTime, 120.0, true);
    assert_eq!(up.at_stake, vec![102]);
    assert_eq!(set.state_of(102), Some(MilestoneState::AtStake));
}

#[test]
fn a_pursuit_race_updates_the_statistic_but_never_tests_the_milestones() {
    let mut set = MilestoneSet::new(&defs());
    set.enter_bin(1);
    let up = set.update_stat(StatType::CopsDamaged, 40.0, false);
    assert!(up.improved);
    assert!(up.at_stake.is_empty());
    assert_eq!(set.state_of(101), Some(MilestoneState::Active));
    assert_eq!(set.stat(StatType::CopsDamaged).current, Some(40.0));
}

#[test]
fn a_value_that_is_not_a_new_best_is_not_tested() {
    let mut set = MilestoneSet::new(&defs());
    set.update_stat(StatType::CopsDamaged, 40.0, true); // 101 is locked: nothing to test
    set.enter_bin(1);
    let lower = set.update_stat(StatType::CopsDamaged, 39.0, true);
    assert!(!lower.improved);
    assert!(lower.at_stake.is_empty());
    assert_eq!(set.state_of(101), Some(MilestoneState::Active));
}

#[test]
fn every_active_milestone_of_the_statistic_is_tested_across_bins() {
    let mut set = MilestoneSet::new(&defs());
    set.enter_bins(4, 1);
    let up = set.update_stat(StatType::CopsDamaged, 40.0, true);
    assert_eq!(up.at_stake, vec![101, 301]);
    assert_eq!(set.state_of(103), Some(MilestoneState::Active), "other statistics are not tested");
    assert_eq!(set.state_of(201), Some(MilestoneState::Active));
}

#[test]
fn increment_starts_at_one_and_passes_on_the_sixth() {
    let mut set = MilestoneSet::new(&defs());
    set.enter_bins(5, 4);
    for _ in 0..5 {
        assert!(set.increment(StatType::TotalInfractions, true).at_stake.is_empty());
    }
    assert_eq!(set.stat(StatType::TotalInfractions).current, Some(5.0));
    assert_eq!(set.increment(StatType::TotalInfractions, true).at_stake, vec![401]);
}

// --- bins (3.2) ----------------------------------------------------------------------------------------------------

#[test]
fn entering_a_bin_activates_its_milestones_and_nothing_goes_back() {
    let mut set = MilestoneSet::new(&defs());
    set.enter_bin(1);
    assert_eq!(set.state_of(101), Some(MilestoneState::Active));
    assert_eq!(set.state_of(103), Some(MilestoneState::Active));
    assert_eq!(set.state_of(201), Some(MilestoneState::Locked));
    set.enter_bins(16, 15);
    assert_eq!(set.state_of(201), Some(MilestoneState::Locked), "bins 15 only, from no bin");
    set.enter_bins(3, 2);
    assert_eq!(set.state_of(201), Some(MilestoneState::Active));
    assert_eq!(set.state_of(301), Some(MilestoneState::Locked), "the bin left is not entered again");
    set.enter_bins(2, 3);
    assert_eq!(set.state_of(401), Some(MilestoneState::Locked), "a move up unlocks nothing");
    assert_eq!(set.state_of(101), Some(MilestoneState::Active), "older bins keep their progress");
}

// --- pursuit end (3.6) ---------------------------------------------------------------------------------------------

#[test]
fn an_escape_banks_the_milestone_at_stake_and_pays_its_bounty_to_the_car() {
    let mut set = MilestoneSet::new(&defs());
    let mut record = CareerRecord::new(0);
    set.enter_bin(1);
    set.update_stat(StatType::CopsDamaged, 36.0, true);
    let end = set.on_pursuit_end(&ended(false), true, 200.0, 1);
    assert_eq!(end.banked, vec![Banked { key: 101, bounty: 220_000 }]);
    assert_eq!(end.reverted, 0);
    assert_eq!(set.state_of(101), Some(MilestoneState::Complete));
    assert_eq!(set.milestones()[0].value, Some(36.0), "banked at the statistic's current value");
    assert_eq!(set.state_of(102), Some(MilestoneState::Active), "200 s does not pass 120 s");
    assert_eq!(set.stat(StatType::PursuitsInARow).current, Some(1.0));
    assert_eq!(pay_banked(&mut record, &end.banked), 220_000);
    assert_eq!(record.bounty, 220_000);
    assert_eq!(set.complete_in_bin(1), 1);
}

#[test]
fn a_fast_escape_passes_the_evasion_time_and_banks_it_in_the_same_escape() {
    let mut set = MilestoneSet::new(&defs());
    set.enter_bin(1);
    let end = set.on_pursuit_end(&ended(false), true, 110.0, 1);
    assert_eq!(end.banked, vec![Banked { key: 102, bounty: 220_000 }]);
    assert_eq!(set.state_of(102), Some(MilestoneState::Complete));
    assert_eq!(set.milestones()[1].value, Some(110.0));
    assert_eq!(set.state_of(101), Some(MilestoneState::Active));
}

#[test]
fn a_bust_puts_the_milestones_at_stake_back_to_active_and_banks_nothing() {
    let mut set = MilestoneSet::new(&defs());
    let mut record = CareerRecord::new(0);
    set.enter_bin(1);
    set.update_stat(StatType::CopsDamaged, 36.0, true);
    let end = set.on_pursuit_end(&ended(true), true, 110.0, 1);
    assert_eq!(end, PursuitEndMilestones { banked: vec![], reverted: 1 });
    assert_eq!(set.state_of(101), Some(MilestoneState::Active));
    assert_eq!(set.state_of(102), Some(MilestoneState::Active), "a bust does not set the evasion time");
    assert_eq!(set.stat(StatType::PursuitEvasionTime), StatValue::default());
    assert_eq!(set.stat(StatType::PursuitsInARow), StatValue::default());
    assert_eq!(pay_banked(&mut record, &end.banked), 0);
}

#[test]
fn complete_is_final() {
    let mut set = MilestoneSet::new(&defs());
    set.enter_bin(1);
    set.update_stat(StatType::CopsDamaged, 36.0, true);
    set.on_pursuit_end(&ended(false), true, 200.0, 1);
    let up = set.update_stat(StatType::CopsDamaged, 50.0, true);
    assert!(up.at_stake.is_empty());
    assert_eq!(set.state_of(101), Some(MilestoneState::Complete));
    assert!(set.on_pursuit_end(&ended(false), true, 200.0, 2).banked.is_empty());
    assert_eq!(set.on_pursuit_end(&ended(true), true, 200.0, 2).reverted, 0);
    assert_eq!(set.state_of(101), Some(MilestoneState::Complete));
}

#[test]
fn one_escape_banks_milestones_of_several_bins_and_each_bin_counts_its_own() {
    let mut set = MilestoneSet::new(&defs());
    let mut record = CareerRecord::new(0);
    set.enter_bins(4, 1);
    set.update_stat(StatType::CopsDamaged, 40.0, true);
    let end = set.on_pursuit_end(&ended(false), true, 200.0, 1);
    assert_eq!(end.banked, vec![Banked { key: 101, bounty: 220_000 }, Banked { key: 301, bounty: 140_000 }]);
    assert_eq!(pay_banked(&mut record, &end.banked), 360_000);
    assert_eq!((set.complete_in_bin(1), set.complete_in_bin(2), set.complete_in_bin(3)), (1, 0, 1));
}

// --- pursuit start (3.4, 3.5) and the summary ----------------------------------------------------------------------

#[test]
fn a_new_pursuit_clears_the_per_pursuit_statistics_and_keeps_the_others() {
    let mut set = MilestoneSet::new(&defs());
    set.update_stat(StatType::CopsDamaged, 40.0, true);
    set.update_stat(StatType::CopsDestroyed, 3.0, true);
    set.start_pursuit();
    assert_eq!(set.stat(StatType::CopsDamaged), StatValue::default());
    assert_eq!(set.stat(StatType::CopsDestroyed).current, Some(3.0));
    assert_eq!(set.stat(StatType::CopsDestroyed).best, Some(3.0));
}

#[test]
fn the_summary_holds_at_most_32_keys_and_a_new_pursuit_empties_it() {
    let many: Vec<MilestoneDef> = (0..40u32).map(|i| def(1000 + i, 1, StatType::CopsDamaged, 1.0, 1)).collect();
    let mut set = MilestoneSet::new(&many);
    set.enter_bin(1);
    let up = set.update_stat(StatType::CopsDamaged, 5.0, true);
    assert_eq!(up.at_stake.len(), 40, "all 40 go to at stake");
    assert_eq!(set.pursuit_summary().len(), PURSUIT_SUMMARY_MAX);
    assert_eq!(set.pursuit_summary()[0], 1000);
    assert_eq!(set.pursuit_summary()[31], 1031);
    set.start_pursuit();
    assert!(set.pursuit_summary().is_empty());
}

// --- bin count (3.8, milestone part) -------------------------------------------------------------------------------

#[test]
fn complete_in_bin_counts_only_complete_milestones_of_that_bin() {
    let mut set = MilestoneSet::new(&defs());
    assert_eq!(set.complete_in_bin(1), 0);
    set.enter_bin(1);
    assert_eq!(set.complete_in_bin(1), 0, "active is not complete");
    set.update_stat(StatType::CopsDamaged, 36.0, true);
    set.on_pursuit_end(&ended(false), true, 200.0, 1);
    assert_eq!(set.complete_in_bin(1), 1);
    assert_eq!(set.complete_in_bin(2), 0);
}
