use nfsmw_career::blacklist::{BinDef, Blacklist};
use nfsmw_career::settings::Flags;

use super::*;
use crate::frontend::ids::screen;

/// A rival with one free race, one challenge, a bounty of 1000 per rank and one boss race.
fn def(number: u8) -> BinDef {
    BinDef {
        number,
        races: vec![format!("{number}.1.1")],
        races_needed: 1,
        challenges_needed: 1,
        bounty_needed: 1000 * u32::from(number),
        rival_events: vec![format!("{number}.8.1")],
    }
}

/// A Blacklist of the fifteen small rivals at `stage`, nothing entered yet.
fn blacklist(stage: u8) -> Blacklist {
    Blacklist::new((1..=15).map(def).collect(), stage).expect("fifteen bins")
}

fn row(rows: &[RivalRow], bin: u8) -> RivalRow {
    *rows.iter().find(|row| row.bin == bin).expect("a row for every rival")
}

#[test]
fn the_rows_run_from_rival_15_down_to_rival_1() {
    let rows = rival_rows(&blacklist(16), Flags(0));
    let bins: Vec<u8> = rows.iter().map(|row| row.bin).collect();
    assert_eq!(bins, (1..=15).rev().collect::<Vec<u8>>());
}

#[test]
fn a_rival_never_entered_is_locked_with_its_challenge_off() {
    let rows = rival_rows(&blacklist(16), Flags(0));
    assert!(rows.iter().all(|row| row.state == RivalState::Locked && !row.button_active));
}

#[test]
fn the_requirements_come_from_the_bin_definition() {
    let rows = rival_rows(&blacklist(16), Flags(0));
    let fifteen = row(&rows, 15);
    assert_eq!((fifteen.races_needed, fifteen.challenges_needed, fifteen.bounty_needed), (1, 1, 15_000));
    assert_eq!(row(&rows, 1).bounty_needed, 1_000);
}

#[test]
fn a_rival_above_the_current_bin_has_its_challenge_on() {
    let mut bl = blacklist(16);
    bl.enter_bin(14);
    let rows = rival_rows(&bl, Flags(0));
    assert_eq!(bl.current_bin(), 14);
    assert!(row(&rows, 15).button_active, "rival 15 is beaten, so its races can be replayed");
    assert!(!row(&rows, 14).button_active, "the current rival is locked and has no boss race open");
}

#[test]
fn the_counters_come_from_the_race_records() {
    let mut bl = blacklist(16);
    bl.enter_bin(15);
    let mut flags = Flags(0);
    bl.race_finished("15.1.1", true, &mut flags);
    let rows = rival_rows(&bl, flags);
    let fifteen = row(&rows, 15);
    assert_eq!(fifteen.counters.races_won, 1);
    assert_eq!(fifteen.counters.challenges, 0);
    assert_eq!(fifteen.state, RivalState::Locked, "no challenge and no bounty yet");
}

#[test]
fn an_open_gate_unlocks_the_rival_and_turns_the_challenge_on() {
    let mut bl = blacklist(16);
    let mut flags = Flags(0);
    bl.enter_bin(15);
    bl.bounty_changed(15_000);
    bl.race_finished("15.1.1", true, &mut flags);
    bl.challenge_passed(15, "15.c.1");
    let rows = rival_rows(&bl, flags);
    let fifteen = row(&rows, 15);
    assert_eq!(fifteen.state, RivalState::Unlocked);
    assert!(fifteen.gate_open);
    assert!(fifteen.button_active);
}

#[test]
fn a_rival_with_every_boss_race_won_is_beaten() {
    let mut bl = blacklist(16);
    let mut flags = Flags(0);
    bl.enter_bin(15);
    bl.bounty_changed(15_000);
    bl.race_finished("15.1.1", true, &mut flags);
    bl.challenge_passed(15, "15.c.1");
    bl.race_finished("15.8.1", true, &mut flags);
    let rows = rival_rows(&bl, flags);
    assert_eq!(row(&rows, 15).state, RivalState::Beaten);
}

#[test]
fn the_career_complete_flag_turns_every_challenge_on() {
    let rows = rival_rows(&blacklist(16), Flags::CAREER_COMPLETE);
    assert!(rows.iter().all(|row| row.button_active));
}

#[test]
fn the_stepper_starts_at_the_current_bin_within_the_rivals() {
    assert_eq!(start_bin(16), 15, "the prologue starts at the first rival");
    assert_eq!(start_bin(9), 9);
    assert_eq!(start_bin(1), 1);
    assert_eq!(start_bin(0), 1);
}

#[test]
fn the_stepper_stops_at_rival_15_and_rival_1() {
    assert_eq!(step_bin(15, false), 15, "Left at the first rival stays");
    assert_eq!(step_bin(1, true), 1, "Right at the last rival stays");
    assert_eq!(step_bin(8, true), 7, "Right goes to the next rival down the list");
    assert_eq!(step_bin(8, false), 9, "Left goes back up the list");
}

#[test]
fn the_stepper_walk_never_leaves_the_rivals() {
    let mut bin = 15;
    for _ in 0..40 {
        bin = step_bin(bin, true);
        assert!((1..=15).contains(&bin));
    }
    assert_eq!(bin, 1);
    for _ in 0..40 {
        bin = step_bin(bin, false);
        assert!((1..=15).contains(&bin));
    }
    assert_eq!(bin, 15);
}

#[test]
fn leaving_goes_to_the_safe_house_menu_at_every_stage() {
    for stage in [1, 7, 8, 9, 12, 13, 15, 16] {
        assert_eq!(leave_screen(stage), screen::MAIN_MENU_SUB, "stage {stage}");
    }
}

#[test]
fn the_rival_text_keys_use_two_digits() {
    assert_eq!(rival_key(7, "NAME"), "BLACKLIST_RIVAL_07_NAME");
    assert_eq!(rival_key(15, "BIO"), "BLACKLIST_RIVAL_15_BIO");
}

#[test]
fn make_answers_only_the_two_rival_screens() {
    assert!(make(&screen::SAFE_HOUSE_RIVAL_CHALLENGE.to_ascii_lowercase(), Args::default()).is_some());
    assert!(make(&screen::SAFE_HOUSE_RIVAL_BIO.to_ascii_lowercase(), Args::default()).is_some());
    assert!(make(&screen::MAIN_MENU_SUB.to_ascii_lowercase(), Args::default()).is_none());
}

#[test]
fn a_rival_is_complete_in_progress_or_locked_by_where_the_career_is() {
    let rows = rival_rows(&blacklist(10), Flags(0));
    let beaten = RivalRow { state: RivalState::Beaten, ..row(&rows, 12) };
    assert_eq!(status_of(&beaten, 10), RivalStatus::Complete, "a rival beaten is complete wherever the career is");
    assert_eq!(status_of(&row(&rows, 10), 10), RivalStatus::InProgress, "the rival the career is at");
    assert_eq!(status_of(&row(&rows, 9), 10), RivalStatus::Locked, "a rival further down the list");
    let open = RivalRow { state: RivalState::Unlocked, ..row(&rows, 9) };
    assert_eq!(status_of(&open, 10), RivalStatus::InProgress, "a rival with its gate open is in hand");
}

#[test]
fn a_rival_entry_names_the_portrait_by_rank() {
    assert_eq!(fe_hash_upper("RIVAL_07"), fe_hash_upper(&format!("RIVAL_{:02}", 7)));
    assert_eq!(rival_key(7, "NAME"), "BLACKLIST_RIVAL_07_NAME");
}
