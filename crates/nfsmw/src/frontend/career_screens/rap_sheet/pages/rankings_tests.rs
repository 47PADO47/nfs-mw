use nfsmw_career::rapsheet::PursuitHistory;
use nfsmw_career::rapsheet::rank::{CATEGORY_ROWS, LIFETIME, SINGLE, THRESHOLDS};
use nfsmw_career::stable::Stable;
use nfsmw_data::rank_thresholds::RankThresholds;

use super::*;

#[test]
fn the_row_objects_of_the_tenth_row_have_their_own_hashes() {
    assert_eq!(rank_slot(1), 0x7711_109B);
    assert_eq!(rank_slot(3), 0x7711_109B + 2);
    assert_eq!(rank_slot(10), 0x5933_242B);
    assert_eq!(button_slot(1), 0xCDA0_A66B);
    assert_eq!(button_slot(9), 0xCDA0_A66B + 8);
    assert_eq!(button_slot(10), 0x81B5_73FB);
}

#[test]
fn a_fresh_profile_is_unranked_in_every_row_and_view() {
    let stable = Stable::empty();
    let history = PursuitHistory::default();
    for lifetime in [true, false] {
        let ranks = categories(None, &stable, &history, lifetime);
        assert_eq!(ranks.len(), 10);
        assert!(ranks.iter().all(|&rank| rank == UNRANKED), "lifetime {lifetime}");
    }
}

#[test]
fn the_rows_follow_the_page_order() {
    // Row 5 of the page is the roadblocks statistic (index 5); the 30 of its first single threshold ranks it first.
    assert_eq!(CATEGORY_ROWS[4], 5);
    let mut history = PursuitHistory::default();
    history.best[5].value = 30;
    let ranks = categories(None, &Stable::empty(), &history, false);
    assert_eq!(ranks[4], 1);
    assert_eq!(ranks.iter().filter(|&&rank| rank != UNRANKED).count(), 1);
}

#[test]
fn the_single_view_ranks_the_best_pursuit_and_the_lifetime_view_ignores_it() {
    let mut history = PursuitHistory::default();
    // Pursuit length: 1800 s is the first single-pursuit threshold, in ticks of 1/4000 s.
    history.best[0].value = 1800 * 4000;
    let stable = Stable::empty();
    assert_eq!(rank_of(None, &stable, &history, 0, false), 1);
    assert_eq!(rank_of(None, &stable, &history, 0, true), UNRANKED);
}

#[test]
fn without_the_install_tables_the_crate_defaults_are_used() {
    assert_eq!(thresholds(None, 1, true), Some(LIFETIME[1]));
    assert_eq!(thresholds(None, 1, false), Some(SINGLE[1]));
    assert_eq!(thresholds(None, 10, true), None);
}

#[test]
fn the_install_tables_win_over_the_defaults() {
    let ranks = RankThresholds { lifetime: [[100.0; THRESHOLDS]; 10], single: [[7.0; THRESHOLDS]; 10] };
    assert_eq!(thresholds(Some(&ranks), 3, true), Some([100.0; THRESHOLDS]));
    assert_eq!(thresholds(Some(&ranks), 3, false), Some([7.0; THRESHOLDS]));
    let mut history = PursuitHistory::default();
    history.best[3].value = 7;
    assert_eq!(rank_of(Some(&ranks), &Stable::empty(), &history, 3, false), 1, "7 reaches the install's 7 threshold");
    assert_eq!(rank_of(Some(&ranks), &Stable::empty(), &history, 3, true), UNRANKED, "lifetime uses its own table");
}
