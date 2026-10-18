use super::*;
use crate::blacklist::BinDef;
use crate::milestones::speedtrap::def_for;
use crate::settings::CareerSettings;
use crate::stable::Stable;

/// Fifteen bins with no race or challenge requirement and one boss race each (`15.b1` .. `1.b1`).
fn defs() -> Vec<BinDef> {
    (1..=15u8)
        .map(|number| BinDef {
            number,
            races: Vec::new(),
            races_needed: 0,
            challenges_needed: 0,
            bounty_needed: 0,
            rival_events: vec![format!("{number}.b1")],
        })
        .collect()
}

/// A Blacklist walked down from the prologue to `target`, plus the other parts of the career.
fn start_at(target: u8) -> (CareerSettings, Blacklist, MilestoneSet, Vec<SpeedTrap>, Stable) {
    let career = CareerSettings::new_career();
    let mut blacklist = Blacklist::new(defs(), 16).unwrap();
    blacklist.enter_bin(target);
    let traps = vec![SpeedTrap::new(def_for(14, 1, 7).unwrap())];
    (career, blacklist, MilestoneSet::new(&[]), traps, Stable::empty())
}

fn race(event: &str, placing: u8, cash_value: f32) -> EventResult {
    EventResult {
        event: event.to_owned(),
        placing,
        cash_value,
        is_challenge: false,
        bin: 0,
        career: true,
        bounty_total: None,
        challenge_series_complete: false,
    }
}

#[test]
fn boss_win_pays_cash_walks_down_and_unlocks_bin_14() {
    let (mut career, mut blacklist, mut milestones, mut traps, mut stable) = start_at(15);
    let before = career.cash;
    let outcome = apply_event_result(
        &mut career,
        &mut blacklist,
        &mut milestones,
        &mut traps,
        &mut stable,
        &race("15.b1", 1, 1500.0),
    );
    assert_eq!(outcome.bins_done, vec![15]);
    assert_eq!(outcome.bins_entered, vec![14]);
    assert_eq!(outcome.cash_paid, 1500);
    assert_eq!(career.cash, before + 1500);
    assert_eq!(career.stage, 14);
    assert_eq!(blacklist.current_bin(), 14);
    assert_eq!(outcome.traps_unlocked, 1);
    assert!(traps[0].unlocked());
    assert_eq!(outcome.region_screen, None);
}

#[test]
fn second_place_pays_nothing_and_does_not_beat_the_rival() {
    let (mut career, mut blacklist, mut milestones, mut traps, mut stable) = start_at(15);
    let before = career.cash;
    let outcome = apply_event_result(
        &mut career,
        &mut blacklist,
        &mut milestones,
        &mut traps,
        &mut stable,
        &race("15.b1", 2, 1500.0),
    );
    assert!(outcome.bins_done.is_empty());
    assert!(outcome.bins_entered.is_empty());
    assert_eq!(outcome.cash_paid, 0);
    assert_eq!(career.cash, before);
    assert_eq!(career.stage, 15);
}

#[test]
fn stage_12_after_rival_13_shows_the_region_screen() {
    let (mut career, mut blacklist, mut milestones, mut traps, mut stable) = start_at(13);
    let outcome = apply_event_result(
        &mut career,
        &mut blacklist,
        &mut milestones,
        &mut traps,
        &mut stable,
        &race("13.b1", 1, 2500.0),
    );
    assert_eq!(outcome.bins_done, vec![13]);
    assert_eq!(outcome.bins_entered, vec![12]);
    assert_eq!(career.stage, 12);
    let screen = outcome.region_screen.expect("stage 12 after rival 13 shows the screen");
    assert_eq!(screen.picture_key, 13);
    assert_eq!(screen.label, "REGION_UNLOCK_COSTAL");
}

#[test]
fn outside_career_mode_nothing_is_applied() {
    let (mut career, mut blacklist, mut milestones, mut traps, mut stable) = start_at(15);
    let before = career.cash;
    let mut result = race("15.b1", 1, 1500.0);
    result.career = false;
    let outcome = apply_event_result(&mut career, &mut blacklist, &mut milestones, &mut traps, &mut stable, &result);
    assert_eq!(outcome, EventOutcome::default());
    assert_eq!(career.cash, before);
    assert_eq!(blacklist.current_bin(), 15);
    assert!(!traps[0].unlocked());
}

#[test]
fn a_challenge_is_counted_without_cash() {
    let (mut career, mut blacklist, mut milestones, mut traps, mut stable) = start_at(15);
    let before = career.cash;
    let result = EventResult {
        event: "15.c1".to_owned(),
        placing: 0,
        cash_value: 0.0,
        is_challenge: true,
        bin: 15,
        career: true,
        bounty_total: None,
        challenge_series_complete: false,
    };
    let outcome = apply_event_result(&mut career, &mut blacklist, &mut milestones, &mut traps, &mut stable, &result);
    assert!(outcome.bins_done.is_empty());
    assert_eq!(outcome.cash_paid, 0);
    assert_eq!(career.cash, before);
    assert_eq!(blacklist.bin(15).unwrap().challenges, vec!["15.c1".to_owned()]);
}

#[test]
fn bounty_total_and_challenge_series_flag_are_written() {
    let (mut career, mut blacklist, mut milestones, mut traps, mut stable) = start_at(15);
    let mut result = race("19.8.5", 2, 0.0);
    result.bounty_total = Some(5);
    result.challenge_series_complete = true;
    let outcome = apply_event_result(&mut career, &mut blacklist, &mut milestones, &mut traps, &mut stable, &result);
    assert_eq!(blacklist.bounty(), 5);
    assert!(outcome.bins_done.is_empty());
    assert!(career.flags.contains(Flags::CHALLENGE_SERIES_DONE));
}
