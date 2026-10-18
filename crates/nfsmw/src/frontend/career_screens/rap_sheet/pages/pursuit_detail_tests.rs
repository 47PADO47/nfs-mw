use nfsmw_career::rapsheet::PursuitEntry;

use super::*;

#[test]
fn the_entry_number_becomes_a_zero_based_index() {
    assert_eq!(entry_index(Some(1)), 0);
    assert_eq!(entry_index(Some(5)), 4);
    assert_eq!(entry_index(None), 0);
    assert_eq!(entry_index(Some(0)), 0, "a number below 1 reads as entry 1");
    assert_eq!(entry_index(Some(9)), 4, "a number above 5 reads as entry 5");
}

#[test]
fn the_figures_of_an_entry_are_its_counts_and_its_length() {
    let entry = PursuitEntry {
        ticks: 62 * 4000 + 2000,
        cops_involved: 4,
        cops_damaged: 1,
        cops_destroyed: 0,
        roadblocks_dodged: 2,
        spikes_dodged: 3,
        cost_to_state: 12_000,
        infraction_kinds: 5,
        helicopters: 0,
        bounty: 25_000,
        ..PursuitEntry::default()
    };
    let figures = figures(&entry);
    assert_eq!(figures.len(), 10);
    assert_eq!(figures[0], (LENGTH, " 1:02.50".to_owned()));
    assert_eq!(figures[1], (COPS_INVOLVED, "4".to_owned()));
    assert_eq!(figures[6], (COST_TO_STATE, "12,000".to_owned()));
    assert_eq!(figures[9], (BOUNTY, "25,000".to_owned()));
}
