use nfsmw_career::regions::{GateSet, closed_gates};
use nfsmw_data::gameplay::Bin;

use super::*;

/// A bin record of the stage `number` with the given `Barriers` list.
fn bin(number: i32, barriers: &[&str]) -> Bin {
    Bin { number, barriers: barriers.iter().map(|s| s.to_string()).collect(), ..Bin::default() }
}

/// The `Barriers` lists of the install (`race_bin_NN`), as in `docs/specs/career-regions.md` section 1.1.
fn install_bins() -> Vec<Bin> {
    const NORTH_WEST: &[&str] =
        &["BARRIER_LOCKED_15", "BARRIER_LOCKED_14", "BARRIER_LOCKED_11", "BARRIER_LOCKED_13", "BARRIER_LOCKED_12"];
    const MIDDLE: &[&str] = &["BARRIER_LOCKED_12", "BARRIER_LOCKED_13", "BARRIER_LOCKED_10"];
    vec![
        bin(16, NORTH_WEST),
        bin(15, NORTH_WEST),
        bin(14, NORTH_WEST),
        bin(13, NORTH_WEST),
        bin(12, MIDDLE),
        bin(11, MIDDLE),
        bin(10, MIDDLE),
        bin(9, MIDDLE),
        bin(8, &[]),
        bin(7, &[]),
        bin(6, &[]),
        bin(5, &[]),
        bin(4, &[]),
        bin(3, &[]),
        bin(2, &[]),
        bin(1, &[]),
    ]
}

#[test]
fn the_bin_record_of_a_stage_is_found_by_its_number() {
    let bins = install_bins();
    assert_eq!(bin_barriers(&bins, 12).unwrap().len(), 3);
    assert_eq!(bin_barriers(&bins, 16).unwrap().len(), 5, "the prologue bin is a record too");
}

#[test]
fn a_missing_record_closes_no_gate() {
    assert_eq!(bin_barriers(&[], 12), None);
    assert_eq!(bin_barriers(&install_bins(), 17), None);
    assert_eq!(stage_gates(None), GateSet::EMPTY);
}

#[test]
fn the_barriers_list_of_every_stage_closes_the_documented_set() {
    let bins = install_bins();
    for stage in 1..=16u8 {
        assert_eq!(stage_gates(bin_barriers(&bins, stage)), closed_gates(stage), "stage {stage}");
    }
}

#[test]
fn only_region_gate_entries_count_and_a_star_is_removed() {
    let list: Vec<String> =
        ["*BARRIER_LOCKED_12", "SCENERY_GROUP_DOOR", "BARRIER_SPLINE_4", "barrier_locked_13", "BARRIER_LOCKED_9"]
            .iter()
            .map(|s| s.to_string())
            .collect();
    assert_eq!(stage_gates(Some(list.as_slice())).numbers(), [12]);
}

#[test]
fn the_first_evaluation_stores_the_set_without_any_change_to_report() {
    let mut state = RegionState::default();
    let update = state.evaluate(13, closed_gates(13)).unwrap();
    assert_eq!(update.unlock, None);
    assert_eq!(update.change.opened, GateSet::EMPTY);
    assert_eq!(update.change.closed, closed_gates(13));
    assert_eq!(state.closed_gates(), closed_gates(13));
}

#[test]
fn an_unchanged_stage_evaluates_to_nothing() {
    let mut state = RegionState::default();
    state.evaluate(12, closed_gates(12));
    assert_eq!(state.evaluate(12, closed_gates(12)), None);
}

#[test]
fn stage_13_to_12_opens_the_eastern_gates_and_closes_gate_10() {
    let mut state = RegionState::default();
    state.evaluate(13, closed_gates(13));
    let update = state.evaluate(12, closed_gates(12)).unwrap();
    assert_eq!(update.change.opened.numbers(), [11, 14, 15]);
    assert_eq!(update.change.closed.numbers(), [10]);
    assert_eq!(state.closed_gates(), closed_gates(12));
}

#[test]
fn stage_9_to_8_opens_the_southern_gates() {
    let mut state = RegionState::default();
    state.evaluate(9, closed_gates(9));
    let update = state.evaluate(8, closed_gates(8)).unwrap();
    assert_eq!(update.change.opened.numbers(), [10, 12, 13]);
    assert_eq!(update.change.closed, GateSet::EMPTY);
}

#[test]
fn stage_12_reached_from_above_asks_for_the_unlock_screen_once() {
    let mut state = RegionState::default();
    state.evaluate(13, closed_gates(13));
    assert_eq!(state.evaluate(12, closed_gates(12)).unwrap().unlock, Some(12));
    assert_eq!(state.evaluate(12, closed_gates(12)), None, "no second request at the same stage");
}

#[test]
fn stage_8_reached_from_above_asks_for_the_unlock_screen() {
    let mut state = RegionState::default();
    state.evaluate(9, closed_gates(9));
    assert_eq!(state.evaluate(8, closed_gates(8)).unwrap().unlock, Some(8));
}

#[test]
fn a_stage_reached_by_skipping_past_12_still_asks_when_it_is_12_or_8() {
    let mut state = RegionState::default();
    state.evaluate(16, closed_gates(16));
    assert_eq!(state.evaluate(12, closed_gates(12)).unwrap().unlock, Some(12));
}

#[test]
fn a_save_loaded_at_12_or_8_asks_for_nothing() {
    for stage in [12u8, 8] {
        let mut state = RegionState::default();
        assert_eq!(state.evaluate(stage, closed_gates(stage)).unwrap().unlock, None, "stage {stage}");
    }
}

#[test]
fn a_rise_in_stage_or_a_stage_without_a_screen_asks_for_nothing() {
    let mut state = RegionState::default();
    state.evaluate(8, closed_gates(8));
    assert_eq!(state.evaluate(12, closed_gates(12)).unwrap().unlock, None, "stage going up");
    assert_eq!(state.evaluate(11, closed_gates(11)).unwrap().unlock, None, "11 has no screen");
    assert_eq!(state.evaluate(10, closed_gates(10)).unwrap().unlock, None, "10 has no screen");
}

#[test]
fn only_stages_12_and_8_ask_for_the_screen() {
    for stage in 1..=15u8 {
        let mut state = RegionState::default();
        state.evaluate(stage + 1, closed_gates(stage + 1));
        let unlock = state.evaluate(stage, closed_gates(stage)).unwrap().unlock;
        assert_eq!(unlock.is_some(), matches!(stage, 8 | 12), "stage {stage}");
    }
}

#[test]
fn a_race_lifts_the_closed_gates_and_the_end_gives_them_back() {
    let mut state = RegionState::default();
    state.evaluate(16, closed_gates(16));
    assert!(!state.is_lifted());
    assert_eq!(state.lift(), closed_gates(16), "the prologue's gates go off for the run");
    assert!(state.is_lifted());
    assert_eq!(state.restore(), closed_gates(16), "and come back with the end");
    assert!(!state.is_lifted());
}

#[test]
fn lifting_twice_or_restoring_without_a_lift_changes_nothing() {
    let mut state = RegionState::default();
    state.evaluate(13, closed_gates(13));
    assert_eq!(state.restore(), GateSet::EMPTY, "nothing was lifted");
    state.lift();
    assert_eq!(state.lift(), GateSet::EMPTY, "a restart lifts what is lifted already");
    assert_eq!(state.restore(), closed_gates(13));
}

#[test]
fn a_race_that_begins_before_the_first_evaluation_lifts_nothing_and_the_gates_stay_off() {
    // The queued race of a new career starts as soon as the car stands, which can be before the career's first frame.
    let mut state = RegionState::default();
    assert_eq!(state.lift(), GateSet::EMPTY);
    let update = state.evaluate(16, closed_gates(16)).unwrap();
    assert_eq!(update.change.closed, closed_gates(16), "reported, but the world skips it while the race is lifted");
    assert!(state.is_lifted());
    assert_eq!(state.restore(), closed_gates(16), "the end closes the stage's set");
}

#[test]
fn a_stage_that_changes_during_the_race_is_the_one_that_comes_back() {
    let mut state = RegionState::default();
    state.evaluate(13, closed_gates(13));
    state.lift();
    let update = state.evaluate(12, closed_gates(12)).unwrap();
    assert_eq!(update.unlock, Some(12), "the unlock screen is still asked for");
    assert_eq!(state.restore().numbers(), [10, 12, 13], "the set of the new stage, not the old one");
}
