use nfsmw_career::blacklist::BinDef;
use nfsmw_career::settings::Flags;

use super::*;

/// A bin with one free race `n.1.1` and one boss race `n.5.1`, and `races_needed` free races to unlock the rival.
fn def(number: u8, races_needed: u32) -> BinDef {
    BinDef {
        number,
        races: vec![format!("{number}.1.1")],
        races_needed,
        challenges_needed: 0,
        bounty_needed: 0,
        rival_events: vec![format!("{number}.5.1")],
    }
}

/// The Blacklist at stage 16 (nothing entered). Only bin 15 needs `races_needed_15` free races; the others need none.
fn blacklist(races_needed_15: u32) -> Blacklist {
    let defs = (1..=LAST_BIN).map(|number| def(number, if number == LAST_BIN { races_needed_15 } else { 0 })).collect();
    Blacklist::new(defs, 16).expect("bins 1 to 15 are all given")
}

#[test]
fn a_race_the_blacklist_does_not_list_is_not_a_career_race() {
    let bl = blacklist(0);
    assert_eq!(race_entry(&bl, "16.1.0"), RaceEntry::NotCareer, "prologue race");
    assert_eq!(race_entry(&bl, "19.8.69"), RaceEntry::NotCareer, "challenge series");
    assert_eq!(race_entry(&bl, "no_such_race"), RaceEntry::NotCareer, "unknown race");
}

#[test]
fn a_race_in_a_bin_not_entered_yet_is_locked() {
    let bl = blacklist(0);
    assert_eq!(race_entry(&bl, "15.1.1"), RaceEntry::Locked, "free race, bin not entered");
    assert_eq!(race_entry(&bl, "15.5.1"), RaceEntry::Locked, "boss race, bin not entered");
}

#[test]
fn an_entered_bin_opens_its_free_races_and_an_unlocked_rival_its_boss_race() {
    let mut bl = blacklist(0);
    assert_eq!(bl.enter_bin(15), vec![15]);
    assert_eq!(race_entry(&bl, "15.1.1"), RaceEntry::Open);
    assert_eq!(race_entry(&bl, "15.5.1"), RaceEntry::Open, "rival unlocked on entry");
}

#[test]
fn a_locked_rival_keeps_its_boss_race_locked_while_the_free_races_open() {
    let mut bl = blacklist(1);
    bl.enter_bin(15);
    assert_eq!(race_entry(&bl, "15.1.1"), RaceEntry::Open, "free race of an entered bin");
    assert_eq!(race_entry(&bl, "15.5.1"), RaceEntry::Locked, "rival needs a free race won");
}

#[test]
fn winning_the_gate_race_opens_the_rival() {
    let mut bl = blacklist(1);
    bl.enter_bin(15);
    let mut flags = Flags::default();
    assert!(bl.race_finished("15.1.1", true, &mut flags).is_empty(), "the gate opens, the bin is not done");
    assert_eq!(bl.bin(15).and_then(|record| record.state), Some(BinState::RivalUnlocked));
    assert_eq!(race_entry(&bl, "15.5.1"), RaceEntry::Open);
}

#[test]
fn a_done_bin_keeps_its_boss_race_open_and_hands_the_walk_down() {
    let mut bl = blacklist(0);
    bl.enter_bin(15);
    let mut flags = Flags::default();
    assert_eq!(bl.race_finished("15.5.1", true, &mut flags), vec![15], "boss race won, bin 15 done");
    assert_eq!(race_entry(&bl, "15.5.1"), RaceEntry::Open, "replayable once done");
    assert_eq!(race_entry(&bl, "14.1.1"), RaceEntry::Open, "bin 14 entered after bin 15 is done");
    assert_eq!(race_entry(&bl, "13.1.1"), RaceEntry::Locked, "bin 13 not reached yet");
}

#[test]
fn a_boss_race_is_told_apart_from_a_free_race() {
    let bl = blacklist(0);
    assert!(is_boss_race(&bl, "15.5.1"));
    assert!(!is_boss_race(&bl, "15.1.1"), "a free race is not a boss race");
    assert!(!is_boss_race(&bl, "99.5.1"), "an unknown race is not a boss race");
}
