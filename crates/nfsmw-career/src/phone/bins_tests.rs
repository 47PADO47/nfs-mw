use super::*;
use crate::phone::is_known;

#[test]
fn bin_14_sends_its_two_entry_messages() {
    assert_eq!(bin_messages(14, BinMoment::Entry), &[125, 126]);
}

#[test]
fn bin_15_sends_nothing_on_entry() {
    assert!(bin_messages(15, BinMoment::Entry).is_empty());
}

#[test]
fn entry_messages_match_the_spec_table() {
    let expected: [(u8, u32); 13] = [
        (13, 30),
        (12, 37),
        (11, 42),
        (10, 48),
        (9, 54),
        (8, 58),
        (7, 64),
        (6, 70),
        (5, 73),
        (4, 77),
        (3, 80),
        (2, 85),
        (1, 89),
    ];
    for (bin, id) in expected {
        assert_eq!(bin_messages(bin, BinMoment::Entry), &[id], "entry of bin {bin}");
    }
}

#[test]
fn every_bin_from_one_to_fifteen_has_a_rival_message() {
    let expected: [(u8, &[u32]); 15] = [
        (1, &[93, 94]),
        (2, &[88]),
        (3, &[84]),
        (4, &[79]),
        (5, &[76]),
        (6, &[72]),
        (7, &[69]),
        (8, &[63]),
        (9, &[57]),
        (10, &[52]),
        (11, &[46]),
        (12, &[41]),
        (13, &[36]),
        (14, &[29]),
        (15, &[20]),
    ];
    for (bin, ids) in expected {
        assert_eq!(bin_messages(bin, BinMoment::RivalUnlock), ids, "rival of bin {bin}");
    }
}

#[test]
fn race_win_messages_fire_on_their_exact_count() {
    assert_eq!(bin_messages(15, BinMoment::RaceWins(1)), &[123]);
    assert_eq!(bin_messages(14, BinMoment::RaceWins(2)), &[27]);
    assert_eq!(bin_messages(13, BinMoment::RaceWins(4)), &[31]);
    assert_eq!(bin_messages(7, BinMoment::RaceWins(4)), &[65], "world heat is not a message");
    assert_eq!(bin_messages(7, BinMoment::RaceWins(7)), &[134]);
    assert_eq!(bin_messages(1, BinMoment::RaceWins(6)), &[91]);
    assert!(bin_messages(7, BinMoment::RaceWins(2)).is_empty(), "no message on a count the spec does not list");
}

#[test]
fn bin_5_has_no_race_win_message() {
    for wins in 1..=7 {
        assert!(bin_messages(5, BinMoment::RaceWins(wins)).is_empty(), "bin 5 at {wins} wins");
    }
}

#[test]
fn challenge_messages_match_the_spec_table() {
    assert_eq!(bin_messages(15, BinMoment::Challenges(1)), &[124]);
    assert_eq!(bin_messages(15, BinMoment::Challenges(3)), &[137]);
    assert_eq!(bin_messages(14, BinMoment::Challenges(2)), &[19]);
    assert_eq!(bin_messages(13, BinMoment::Challenges(2)), &[25]);
    assert_eq!(bin_messages(13, BinMoment::Challenges(3)), &[26]);
    assert_eq!(bin_messages(5, BinMoment::Challenges(3)), &[101]);
    assert_eq!(bin_messages(4, BinMoment::Challenges(4)), &[102]);
}

#[test]
fn bins_2_and_3_and_1_have_no_challenge_message() {
    for bin in [1, 2, 3] {
        for count in 1..=4 {
            assert!(bin_messages(bin, BinMoment::Challenges(count)).is_empty(), "bin {bin} at {count}");
        }
    }
}

#[test]
fn an_unknown_bin_sends_nothing() {
    assert!(bin_messages(16, BinMoment::Entry).is_empty());
    assert!(bin_messages(0, BinMoment::RivalUnlock).is_empty());
}

#[test]
fn every_message_in_the_table_is_a_known_message() {
    for (bin, moment, ids) in TABLE {
        for id in ids.iter() {
            assert!(is_known(*id), "bin {bin} {moment:?} sends unknown id {id}");
        }
    }
}

#[test]
fn each_message_belongs_to_one_trigger_only() {
    let mut seen = Vec::new();
    for (_, _, ids) in TABLE {
        for id in ids.iter() {
            assert!(!seen.contains(id), "id {id} is listed twice");
            seen.push(*id);
        }
    }
}
