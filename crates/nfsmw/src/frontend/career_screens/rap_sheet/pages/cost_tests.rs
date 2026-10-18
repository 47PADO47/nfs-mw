use nfsmw_career::rapsheet::{PursuitHistory, cost_rows};

use super::*;

#[test]
fn the_lines_print_their_quantity_and_cost_with_grouping() {
    let mut history = PursuitHistory { extras: [10, 20, 30, 40, 50, 60, 7_000, 8], ..PursuitHistory::default() };
    history.sums[2] = 3;
    history.sums[3] = 2;
    let lines = line_texts(&cost_rows(&history));
    assert_eq!(lines.len(), 9);
    assert_eq!(lines[0], ("RAP_CTS_DAMAGE_TO_PROPERTY", "8".to_owned(), "7,000".to_owned()));
    assert_eq!(lines[1], ("RAP_CTS_INSURANCE_CLAIMS", "20".to_owned(), "10,000".to_owned()));
    assert_eq!(lines[4], ("RAP_CTS_DAMAGED_POLICE_VEHICLES", "3".to_owned(), "750".to_owned()));
    assert_eq!(lines[5], ("RAP_CTS_DESTROYED_POLICE_VEHICLES", "2".to_owned(), "10,000".to_owned()));
}

#[test]
fn a_clean_record_prints_zeros() {
    let lines = line_texts(&cost_rows(&PursuitHistory::default()));
    assert!(lines.iter().all(|(_, quantity, cost)| quantity == "0" && cost == "0"));
}
