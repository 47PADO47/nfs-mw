use super::*;
use crate::safehouse::EventProgress;
use crate::settings::CareerSettings;

/// Bins 15 to 1: (number, RequiredRacesWon, RequiredChallenges, RequiredBounty, WorldRaces, BossRaces), from the
/// table of `career-blacklist.md` section 1.
const TABLE: [(u8, u32, u32, u32, usize, usize); 15] = [
    (15, 3, 3, 20_000, 3, 2),
    (14, 4, 3, 50_000, 6, 2),
    (13, 4, 3, 100_000, 7, 2),
    (12, 4, 3, 180_000, 7, 2),
    (11, 5, 3, 300_000, 8, 2),
    (10, 5, 4, 500_000, 8, 2),
    (9, 5, 4, 790_000, 8, 2),
    (8, 5, 4, 1_180_000, 7, 2),
    (7, 7, 4, 1_680_000, 10, 2),
    (6, 7, 4, 2_300_000, 11, 2),
    (5, 7, 4, 3_050_000, 10, 3),
    (4, 7, 5, 4_050_000, 11, 3),
    (3, 8, 5, 5_550_000, 12, 3),
    (2, 8, 5, 7_550_000, 12, 3),
    (1, 9, 5, 10_000_000, 11, 6),
];

/// The race and challenge ids are made up (`15.w1`, `15.b2`, `15.c3`); the counts and the numbers are the spec's.
fn defs() -> Vec<BinDef> {
    TABLE
        .iter()
        .map(|&(number, races_needed, challenges_needed, bounty_needed, world, boss)| {
            let mut rival_events: Vec<String> = (1..=boss).map(|k| format!("{number}.b{k}")).collect();
            if number == 1 {
                rival_events[0] = FINAL_RACE_EVENT.to_owned();
            }
            BinDef {
                number,
                races: (1..=world).map(|k| format!("{number}.w{k}")).collect(),
                races_needed,
                challenges_needed,
                bounty_needed,
                rival_events,
            }
        })
        .collect()
}

fn def(number: u8) -> BinDef {
    defs().into_iter().find(|d| d.number == number).expect("bin in table")
}

fn start(stage: u8) -> Blacklist {
    Blacklist::new(defs(), stage).expect("valid table")
}

fn state(bl: &Blacklist, number: u8) -> Option<BinState> {
    bl.bin(number).and_then(|record| record.state)
}

/// Bounty, every challenge and every free race of bin `n` (a first place each). Returns the bins that reached done.
fn open_gate(bl: &mut Blacklist, flags: &mut Flags, n: u8) -> Vec<u8> {
    let d = def(n);
    let mut done = bl.bounty_changed(d.bounty_needed);
    for k in 1..=d.challenges_needed {
        done.extend(bl.challenge_passed(n, &format!("{n}.c{k}")));
    }
    for race in d.races.iter().take(d.races_needed as usize) {
        done.extend(bl.race_finished(race, true, flags));
    }
    done
}

/// Opens bin `n`, then wins its boss races with first place. Returns the bins that reached done.
fn clear_bin(bl: &mut Blacklist, flags: &mut Flags, n: u8) -> Vec<u8> {
    let mut done = open_gate(bl, flags, n);
    for boss in def(n).rival_events {
        done.extend(bl.race_finished(&boss, true, flags));
    }
    done
}

#[test]
fn the_fixture_carries_the_spec_numbers() {
    let defs = defs();
    assert_eq!(defs.len(), 15);
    let fifteen = def(15);
    assert_eq!((fifteen.races_needed, fifteen.challenges_needed, fifteen.bounty_needed), (3, 3, 20_000));
    assert_eq!((fifteen.races.len(), fifteen.rival_events.len()), (3, 2));
    let one = def(1);
    assert_eq!((one.races_needed, one.challenges_needed, one.bounty_needed), (9, 5, 10_000_000));
    assert_eq!(one.rival_events.len(), 6);
    assert_eq!(one.rival_events[0], FINAL_RACE_EVENT);
}

#[test]
fn construction_checks_the_bin_set_and_the_stage() {
    let mut missing = defs();
    missing.retain(|d| d.number != 7);
    assert_eq!(Blacklist::new(missing, 16).err(), Some(BlacklistError::MissingBin(7)));

    let mut twice = defs();
    twice.push(def(3));
    assert_eq!(Blacklist::new(twice, 16).err(), Some(BlacklistError::DuplicateBin(3)));

    let mut extra = defs();
    extra.push(BinDef { number: 18, ..def(1) });
    assert_eq!(Blacklist::new(extra, 16).err(), Some(BlacklistError::BadBin(18)));

    assert_eq!(Blacklist::new(defs(), 0).err(), Some(BlacklistError::BadStage(0)));
    assert_eq!(Blacklist::new(defs(), 17).err(), Some(BlacklistError::BadStage(17)));
}

#[test]
fn a_new_career_starts_at_16_with_no_graph_entered() {
    let bl = start(16);
    assert_eq!(bl.current_bin(), 16);
    assert!((1..=15).all(|n| state(&bl, n).is_none()));
}

#[test]
fn the_first_visit_enters_bin_15_locked_and_entering_at_or_above_the_current_bin_does_nothing() {
    let mut bl = start(16);
    assert!(bl.enter_bin(16).is_empty());
    assert_eq!(bl.enter_bin(15), vec![15]);
    assert_eq!(bl.current_bin(), 15);
    assert_eq!(state(&bl, 15), Some(BinState::RivalLocked));
    assert!(bl.enter_bin(15).is_empty());
}

#[test]
fn the_current_bin_is_written_to_the_stage_byte() {
    let mut settings = CareerSettings::new_career();
    let mut bl = start(16);
    bl.enter_bin(15);
    bl.apply_to_settings(&mut settings);
    assert_eq!(settings.stage, 15);
}

#[test]
fn the_gate_needs_races_challenges_and_bounty_together() {
    let mut bl = start(16);
    let mut flags = Flags::EMPTY;
    bl.enter_bin(15);
    for race in ["15.w1", "15.w2", "15.w3"] {
        bl.race_finished(race, true, &mut flags);
    }
    assert_eq!(state(&bl, 15), Some(BinState::RivalLocked), "races only");

    bl.challenge_passed(15, "15.c1");
    bl.challenge_passed(15, "15.c2");
    bl.bounty_changed(20_000);
    assert_eq!(state(&bl, 15), Some(BinState::RivalLocked), "two of three challenges");

    bl.challenge_passed(15, "15.c3");
    assert_eq!(state(&bl, 15), Some(BinState::RivalUnlocked));
}

#[test]
fn a_bounty_one_short_keeps_the_rival_locked() {
    let mut bl = start(16);
    let mut flags = Flags::EMPTY;
    bl.enter_bin(15);
    for k in 1..=3 {
        bl.challenge_passed(15, &format!("15.c{k}"));
        bl.race_finished(&format!("15.w{k}"), true, &mut flags);
    }
    assert!(bl.bounty_changed(19_999).is_empty());
    assert_eq!(state(&bl, 15), Some(BinState::RivalLocked));
    assert!(bl.bounty_changed(20_000).is_empty());
    assert_eq!(state(&bl, 15), Some(BinState::RivalUnlocked));
}

#[test]
fn a_repeated_race_or_challenge_counts_once() {
    let mut bl = start(16);
    let mut flags = Flags::EMPTY;
    bl.enter_bin(15);
    bl.race_finished("15.w1", true, &mut flags);
    bl.race_finished("15.w1", true, &mut flags);
    bl.challenge_passed(15, "15.c1");
    bl.challenge_passed(15, "15.c1");
    let record = bl.bin(15).expect("bin 15");
    assert_eq!(record.counters(), Counters { races_won: 1, challenges: 1 });
}

#[test]
fn a_race_outside_the_bin_is_not_counted_for_it() {
    let mut bl = start(16);
    let mut flags = Flags::EMPTY;
    bl.enter_bin(15);
    bl.race_finished("14.w1", true, &mut flags);
    assert_eq!(bl.bin(15).expect("bin 15").counters().races_won, 0);
    assert_eq!(bl.bin(14).expect("bin 14").counters().races_won, 1);
}

#[test]
fn a_second_place_win_counts_but_does_not_recheck_until_gameplay_is_entered() {
    // Unconfirmed reading of section 2.6 (first place only); see `race_finished`.
    let mut bl = start(16);
    let mut flags = Flags::EMPTY;
    bl.enter_bin(15);
    for k in 1..=3 {
        bl.challenge_passed(15, &format!("15.c{k}"));
    }
    bl.bounty_changed(20_000);
    for k in 1..=3 {
        bl.race_finished(&format!("15.w{k}"), false, &mut flags);
    }
    assert_eq!(bl.bin(15).expect("bin 15").counters().races_won, 3);
    assert_eq!(state(&bl, 15), Some(BinState::RivalLocked));
    bl.entering_gameplay();
    assert_eq!(state(&bl, 15), Some(BinState::RivalUnlocked));
}

#[test]
fn a_boss_race_is_only_counted_once_the_rival_is_unlocked() {
    let mut bl = start(16);
    let mut flags = Flags::EMPTY;
    bl.enter_bin(15);
    bl.race_finished("15.b1", true, &mut flags);
    assert!(bl.bin(15).expect("bin 15").rival_races_won.is_empty());
}

#[test]
fn beating_both_boss_races_finishes_bin_15_and_enters_bin_14() {
    let mut bl = start(16);
    let mut flags = Flags::EMPTY;
    bl.enter_bin(15);
    open_gate(&mut bl, &mut flags, 15);
    assert_eq!(state(&bl, 15), Some(BinState::RivalUnlocked));

    assert!(bl.race_finished("15.b1", false, &mut flags).is_empty());
    assert!(bl.bin(15).expect("bin 15").rival_races_won.is_empty(), "second place does not count");
    assert!(bl.race_finished("15.b1", true, &mut flags).is_empty());
    let done = bl.race_finished("15.b2", true, &mut flags);

    assert_eq!(done, vec![15]);
    assert_eq!(state(&bl, 15), Some(BinState::Done));
    assert_eq!(bl.current_bin(), 14);
    assert_eq!(state(&bl, 14), Some(BinState::RivalLocked));
}

#[test]
fn a_bin_without_boss_races_is_done_as_soon_as_its_gate_opens() {
    let mut d = defs();
    d.iter_mut().find(|b| b.number == 15).expect("bin 15").rival_events.clear();
    let mut bl = Blacklist::new(d, 16).expect("valid table");
    let mut flags = Flags::EMPTY;
    bl.enter_bin(15);
    let done = open_gate(&mut bl, &mut flags, 15);
    assert_eq!(done, vec![15]);
    assert_eq!(state(&bl, 15), Some(BinState::Done));
    assert_eq!(bl.current_bin(), 14);
}

#[test]
fn the_final_race_needs_a_first_place_and_sets_career_completed() {
    let mut bl = start(2);
    let mut flags = Flags::EMPTY;
    assert_eq!(bl.enter_bin(1), vec![1]);
    open_gate(&mut bl, &mut flags, 1);

    assert!(bl.race_finished(FINAL_RACE_EVENT, false, &mut flags).is_empty());
    assert!(!flags.contains(Flags::CAREER_COMPLETE));
    assert!(bl.race_finished(FINAL_RACE_EVENT, true, &mut flags).is_empty(), "one of six boss races");
    assert!(flags.contains(Flags::CAREER_COMPLETE));
    assert!(bl.is_complete(FINAL_RACE_EVENT));
    assert!(bl.is_complete("1.w1"));
    assert!(!bl.is_complete("16.2.1"));
}

#[test]
fn the_challenge_series_last_race_sets_its_flag_on_any_finish() {
    let mut bl = start(16);
    let mut flags = Flags::EMPTY;
    bl.race_finished(CHALLENGE_SERIES_LAST_RACE, false, &mut flags);
    assert!(flags.contains(Flags::EVENT_19_8_69_DONE));
}

#[test]
fn the_rival_challenge_button_follows_the_three_rules() {
    let bl = start(10);
    let none = Flags::EMPTY;
    assert!(!bl.rival_challenge_active(10, none, false));
    assert!(bl.rival_challenge_active(10, none, true), "a boss race is open");
    assert!(bl.rival_challenge_active(11, none, false), "already beaten, replayable");
    assert!(!bl.rival_challenge_active(9, none, false), "below the current bin and not open");
    assert!(bl.rival_challenge_active(3, Flags::CAREER_COMPLETE, false), "career completed");
}

#[test]
fn beating_each_rival_in_turn_walks_down_to_bin_1_and_stops_there() {
    let mut bl = start(16);
    let mut flags = Flags::EMPTY;
    assert_eq!(bl.enter_bin(15), vec![15]);
    for n in (1..=15u8).rev() {
        let done = clear_bin(&mut bl, &mut flags, n);
        assert_eq!(done, vec![n], "bin {n} reaches done");
        let next = n.saturating_sub(1).max(1);
        assert_eq!(bl.current_bin(), next, "after bin {n}");
        if n > 1 {
            assert_eq!(state(&bl, n - 1), Some(BinState::RivalLocked), "bin {} entered", n - 1);
        }
    }
    assert_eq!(state(&bl, 1), Some(BinState::Done));
    assert!(flags.contains(Flags::CAREER_COMPLETE));
}

#[test]
fn a_done_bin_above_the_current_one_does_not_move_the_walk() {
    let mut bl = start(16);
    let mut flags = Flags::EMPTY;
    bl.enter_bin(15);
    clear_bin(&mut bl, &mut flags, 15);
    assert_eq!(bl.current_bin(), 14);
    bl.race_finished("15.b1", true, &mut flags);
    assert_eq!(bl.current_bin(), 14);
    assert_eq!(state(&bl, 15), Some(BinState::Done));
}

#[test]
fn the_stage_byte_in_the_settings_follows_the_walk() {
    let mut settings = CareerSettings::new_career();
    let mut bl = start(16);
    let mut flags = Flags::EMPTY;
    bl.enter_bin(15);
    clear_bin(&mut bl, &mut flags, 15);
    bl.apply_to_settings(&mut settings);
    assert_eq!(settings.stage, 14);
}

#[test]
fn the_state_names_and_save_hashes_match_the_spec() {
    assert_eq!(BinState::Done.save_hash(), 0x3295_6993);
    assert_eq!(BinState::RivalLocked.save_hash(), 0x2ABB_39C2);
    assert_eq!(BinState::RivalUnlocked.save_hash(), 0x76F2_1CB8);
    assert_eq!(BinState::RivalUnlocked.name(), "rival_unlocked");
}

#[test]
fn the_side_bins_keep_their_numbers_and_only_opm_is_a_test_bin() {
    let sides = [SideBin::Tollbooth, SideBin::ChallengeSeries, SideBin::Opm, SideBin::Collectors];
    let numbers = sides.map(SideBin::number);
    assert_eq!(numbers, [18, 19, 20, 21]);
    assert!(SideBin::Opm.is_test_bin());
    assert!(!SideBin::ChallengeSeries.is_test_bin());
}
