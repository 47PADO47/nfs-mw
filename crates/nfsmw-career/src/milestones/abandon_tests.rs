use super::*;

#[test]
fn batch12_safe_house_discards_only_unbanked_pursuit_progress() {
    let defs = [
        MilestoneDef { key: 1, bin: 15, stat: StatType::CopsDamaged, threshold: 1.0, bounty: 100 },
        MilestoneDef { key: 2, bin: 15, stat: StatType::PursuitLength, threshold: 10.0, bounty: 200 },
    ];
    let mut set = MilestoneSet::new(&defs);
    set.enter_bin(15);
    set.update_stat(StatType::CopsDamaged, 2.0, true);
    set.on_pursuit_end(&PursuitResult::default(), true, 100.0, 1);
    assert_eq!(set.state_of(1), Some(MilestoneState::Complete));
    set.start_pursuit();
    set.update_stat(StatType::PursuitLength, 15.0, true);
    assert_eq!(set.state_of(2), Some(MilestoneState::AtStake));
    assert_eq!(set.abandon_pursuit(), 1);
    assert_eq!(set.state_of(1), Some(MilestoneState::Complete));
    assert_eq!(set.state_of(2), Some(MilestoneState::Active));
    assert!(set.pursuit_summary().is_empty());
    assert_eq!(set.stat(StatType::PursuitLength), Default::default());
    assert_eq!(set.abandon_pursuit(), 0, "repeated cleanup cannot bank or revert a completed milestone");
}
