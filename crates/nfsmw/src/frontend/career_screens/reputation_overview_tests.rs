use nfsmw_career::blacklist::{BinDef, Blacklist};

use super::super::blacklist::rival_rows;
use super::*;

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

fn blacklist() -> Blacklist {
    Blacklist::new((1..=15).map(def).collect(), 16).expect("fifteen bins")
}

fn panel_of(bl: &Blacklist, bin: u8, flags: Flags, bounty: u64) -> Panel {
    let rows = rival_rows(bl, flags);
    let row = rows.iter().find(|row| row.bin == bin).expect("a row for every rival");
    panel(row, bl.current_bin(), flags, bounty, 500)
}

#[test]
fn up_goes_to_the_next_rival_and_wraps_to_the_career_rival() {
    assert_eq!(step(12, 12, true), 13);
    assert_eq!(step(14, 12, true), 15);
    assert_eq!(step(15, 12, true), 12, "past rival 15 it comes round to the career's rival");
}

#[test]
fn down_goes_to_the_next_lower_rival_and_wraps_to_rival_15() {
    assert_eq!(step(15, 12, false), 14);
    assert_eq!(step(13, 12, false), 12);
    assert_eq!(step(12, 12, false), 15, "below the career's rival it comes round to rival 15");
}

#[test]
fn a_career_at_rival_15_has_nothing_to_step_to() {
    assert_eq!(step(15, 15, true), 15);
    assert_eq!(step(15, 15, false), 15);
}

#[test]
fn the_page_opens_at_the_asked_rival_between_the_career_and_rival_15() {
    assert_eq!(first_shown(0, 11), 11, "none asked: the career's rival");
    assert_eq!(first_shown(13, 11), 13);
    assert_eq!(first_shown(5, 11), 11, "a rival still to come is out of the range");
    assert_eq!(first_shown(15, 11), 15);
    assert_eq!(lowest(16), 15, "the prologue stage shows rival 15");
    assert_eq!(lowest(0), 1);
}

#[test]
fn a_tick_is_a_met_requirement() {
    assert!(Requirement { have: 3, need: 3 }.met());
    assert!(Requirement { have: 9_000_000_000, need: 5 }.met());
    assert!(!Requirement { have: 2, need: 3 }.met());
}

#[test]
fn the_panel_sets_the_player_against_the_rivals_requirements() {
    let mut bl = blacklist();
    let mut flags = Flags(0);
    bl.enter_bin(15);
    bl.race_finished("15.1.1", true, &mut flags);
    let panel = panel_of(&bl, 15, flags, 4_000);
    assert_eq!(panel.races, Requirement { have: 1, need: 1 });
    assert_eq!(panel.challenges, Requirement { have: 0, need: 1 });
    assert_eq!(panel.bounty, Requirement { have: 4_000, need: 15_000 });
    assert_eq!((panel.races.met(), panel.challenges.met(), panel.bounty.met()), (true, false, false));
    assert_eq!(panel.cash, 500);
}

#[test]
fn the_current_rival_is_not_beaten_and_its_button_waits_for_the_boss_race() {
    let mut bl = blacklist();
    bl.enter_bin(15);
    let panel = panel_of(&bl, 15, Flags(0), 0);
    assert!(!panel.defeated);
    assert_eq!(panel.button, Button::Off);
}

#[test]
fn an_open_boss_race_turns_the_button_on() {
    let mut bl = blacklist();
    let mut flags = Flags(0);
    bl.enter_bin(15);
    bl.bounty_changed(15_000);
    bl.race_finished("15.1.1", true, &mut flags);
    bl.challenge_passed(15, "15.c.1");
    assert_eq!(panel_of(&bl, 15, flags, 15_000).button, Button::On);
}

#[test]
fn a_rival_above_the_career_bin_is_defeated_and_has_no_button() {
    let mut bl = blacklist();
    bl.enter_bin(14);
    let beaten = panel_of(&bl, 15, Flags(0), 0);
    assert!(beaten.defeated);
    assert_eq!(beaten.button, Button::Hidden);
    assert!(!panel_of(&bl, 14, Flags(0), 0).defeated);
}

#[test]
fn a_completed_career_defeats_every_rival() {
    let mut bl = blacklist();
    bl.enter_bin(3);
    let panel = panel_of(&bl, 3, Flags::CAREER_COMPLETE, 0);
    assert!(panel.defeated);
    assert_eq!(panel.button, Button::Hidden);
}

#[test]
fn the_sheets_are_named_by_the_games_labels_and_each_opens_its_package() {
    use blackbox_text::label_hash;
    let labels = [
        (Sheet::RaceEvents, "REP_SHEET_RACE_EVENTS"),
        (Sheet::Milestones, "REP_SHEET_MILESTONE_EVENTS"),
        (Sheet::Bounty, "REP_SHEET_BOUNTY"),
        (Sheet::Bio, "REP_SHEET_BIO"),
    ];
    for (sheet, key) in labels {
        assert_eq!(sheet.label(), label_hash(key), "{key}");
    }
    let screens: Vec<Option<&str>> = SHEETS.into_iter().map(Sheet::screen).collect();
    assert_eq!(
        screens,
        [
            Some("SafeHouseRaceSheet.fng"),
            Some("SafeHouseMilestones.fng"),
            Some("SafeHouseBounty.fng"),
            Some("SafeHouseRivalBio.fng"),
        ]
    );
}

#[test]
fn the_ticker_names_the_bounty_by_its_reward_label() {
    use blackbox_text::label_hash;
    assert_eq!(LABEL_BOUNTY, label_hash("BL_BOUNTY_REWARD"));
}
