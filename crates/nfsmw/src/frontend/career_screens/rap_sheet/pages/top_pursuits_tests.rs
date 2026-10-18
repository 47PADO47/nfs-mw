use nfsmw_career::rapsheet::PursuitEntry;

use super::*;

fn stored(ticks: u32, bounty: u32) -> PursuitEntry {
    let mut case_id = [0u8; 12];
    case_id[..11].copy_from_slice(b"MW-7K3Q9X2A");
    PursuitEntry { case_id, bounty, ticks, ..PursuitEntry::default() }
}

#[test]
fn a_used_entry_shows_its_case_id_bounty_and_length() {
    // 62 s and 2,000 ticks: " 1:02.50" with the page flags.
    let shown = entry(&stored(62 * 4000 + 2000, 12_345), "COBALT".to_owned());
    assert_eq!(
        shown,
        Entry {
            used: true,
            car: "COBALT".to_owned(),
            bounty: 12_345,
            case_id: "MW-7K3Q9X2A".to_owned(),
            length: " 1:02.50".to_owned(),
        }
    );
}

#[test]
fn an_entry_with_no_duration_is_unused() {
    assert!(!entry(&stored(0, 0), String::new()).used);
}

#[test]
fn used_flags_follow_the_entries_in_order() {
    let entries: Vec<Entry> =
        [stored(4000, 1), stored(0, 0), stored(4000, 1)].iter().map(|row| entry(row, String::new())).collect();
    assert_eq!(used_flags(&entries), [true, false, true, false, false]);
}
