use glam::Vec3;
use nfsmw_career::blacklist::{BinDef, BinState, Blacklist};
use nfsmw_career::milestones::speedtrap::{SpeedTrap, SpeedTrapDef};
use nfsmw_career::milestones::{MilestoneDef, MilestoneSet, MilestoneState, StatType};
use nfsmw_data::gameplay::{BountyJump, GameplayData, Milestone, MilestoneType, Placement, Race, RaceParams, RaceType};

use super::bounty::{bounty_rows, on_offer};
use super::milestones::{Entry, goal_text, milestone_rows, render_xy};
use super::race::{race_rows, speed_text, status_of};
use super::{Kind, Status, walk};

/// A bin with two free races and one boss race.
fn def(number: u8) -> BinDef {
    BinDef {
        number,
        races: vec![format!("{number}.1.1"), format!("{number}.2.1")],
        races_needed: 1,
        challenges_needed: 0,
        bounty_needed: 0,
        rival_events: vec![format!("{number}.8.1")],
    }
}

fn blacklist(stage: u8) -> Blacklist {
    Blacklist::new((1..=15).map(def).collect(), stage).expect("fifteen bins")
}

fn race(name: &str, kind: RaceType) -> Race {
    Race {
        key: 1,
        name: name.into(),
        event: None,
        event_icon: String::new(),
        race_type: Some(kind),
        region: None,
        laps: 3,
        start: Some(Placement { position: Vec3::ZERO, heading_deg: 0.0 }),
        finish: None,
        checkpoints: Vec::new(),
        barriers: Vec::new(),
        params: RaceParams { cash_value: 5_000.0, ..RaceParams::default() },
        story: Default::default(),
    }
}

/// The three races of bin 14 in the data, in database order (the boss race first, to check the order of the rows).
fn races() -> Vec<Race> {
    vec![
        race("race_bin_14/14_8_1_sprint", RaceType::Sprint),
        race("race_bin_14/14_2_1_circuit", RaceType::Circuit),
        race("race_bin_14/14_1_1_sprint", RaceType::Sprint),
    ]
}

#[test]
fn the_walk_wraps_both_ways() {
    assert_eq!(walk(0, -1, 5), 4);
    assert_eq!(walk(4, 1, 5), 0);
    assert_eq!(walk(2, 1, 5), 3);
    assert_eq!(walk(0, 1, 0), 0, "an empty sheet stays at the first entry");
}

#[test]
fn the_packages_name_their_sheets() {
    assert_eq!(Kind::of("safehouseracesheet.fng"), Some(Kind::Race));
    assert_eq!(Kind::of("safehousemilestones.fng"), Some(Kind::Milestones));
    assert_eq!(Kind::of("safehousebounty.fng"), Some(Kind::Bounty));
    assert_eq!(Kind::of("safehouserivalbio.fng"), None);
}

#[test]
fn the_race_rows_are_the_free_races_then_the_boss_races() {
    let mut list = blacklist(15);
    list.enter_bin(14);
    let rows = race_rows(&list, &races(), None, 14);
    let events: Vec<&str> = rows.iter().map(|row| row.event.as_str()).collect();
    assert_eq!(events, ["14.1.1", "14.2.1", "14.8.1"]);
    assert_eq!(rows.iter().map(|row| row.boss).collect::<Vec<_>>(), [false, false, true]);
    assert_eq!(rows[1].kind, RaceType::Circuit);
    assert_eq!(rows[1].laps, 3);
    assert_eq!(rows[0].cash, 5_000);
}

#[test]
fn a_race_without_data_is_left_out() {
    let mut list = blacklist(15);
    list.enter_bin(14);
    let only = vec![race("race_bin_14/14_1_1_sprint", RaceType::Sprint)];
    assert_eq!(race_rows(&list, &only, None, 14).len(), 1);
    assert!(race_rows(&list, &only, None, 99).is_empty(), "no such bin");
}

#[test]
fn a_bin_not_entered_has_all_its_races_locked() {
    let list = blacklist(16);
    let rows = race_rows(&list, &races(), None, 14);
    assert!(rows.iter().all(|row| row.status == Status::Locked));
}

#[test]
fn an_entered_bin_opens_its_free_races_and_keeps_the_boss_race_shut_until_the_rival_opens() {
    let mut list = blacklist(15);
    list.enter_bin(14);
    let record = list.bin(14).expect("bin 14");
    assert_eq!(record.state, Some(BinState::RivalLocked));
    assert_eq!(status_of(record, "14.1.1", false), Status::Open);
    assert_eq!(status_of(record, "14.8.1", true), Status::Locked);
}

#[test]
fn a_won_race_is_done() {
    let mut list = blacklist(15);
    list.enter_bin(14);
    let mut flags = nfsmw_career::settings::Flags(0);
    list.race_finished("14.1.1", true, &mut flags);
    let rows = race_rows(&list, &races(), None, 14);
    assert_eq!(rows[0].status, Status::Done);
    assert_eq!(rows[1].status, Status::Open);
}

#[test]
fn the_first_places_of_the_bounty_page_are_always_on_offer() {
    assert!((1..=4).all(|n| on_offer(n, 15)));
    assert!((1..=4).all(|n| on_offer(n, 16)));
}

#[test]
fn the_other_places_open_as_the_career_goes_down_the_list() {
    assert!(!on_offer(5, 13) && on_offer(5, 12), "places 5 to 8 below bin 13");
    assert!(!on_offer(8, 15) && on_offer(8, 1));
    assert!(!on_offer(9, 9) && on_offer(9, 8), "places 9 and up below bin 9");
    assert!(!on_offer(12, 12) && on_offer(12, 1));
}

fn jump(number: i32, position: Option<Vec3>) -> BountyJump {
    BountyJump { key: number as u32, name: format!("bounty_jump/{number}"), number, position }
}

#[test]
fn the_bounty_rows_follow_the_stage_and_skip_a_place_with_no_position() {
    let jumps: Vec<BountyJump> = (1..=12).map(|n| jump(n, (n != 3).then_some(Vec3::new(1.0, 0.0, 2.0)))).collect();
    let numbers = |stage: u8| bounty_rows(&jumps, stage).iter().map(|row| row.number).collect::<Vec<_>>();
    assert_eq!(numbers(15), [1, 2, 4]);
    assert_eq!(numbers(12), [1, 2, 4, 5, 6, 7, 8]);
    assert_eq!(numbers(8), [1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
}

#[test]
fn a_jump_goes_to_the_render_space_ground_point() {
    // Physics space (x right, y up, z forward) to render space (x forward, y left): `(z, -x)`.
    assert_eq!(render_xy(Vec3::new(10.0, 99.0, 30.0)), [30.0, -10.0]);
    let rows = bounty_rows(&[jump(1, Some(Vec3::new(10.0, 99.0, 30.0)))], 15);
    assert_eq!(rows[0].at, [30.0, -10.0]);
}

#[test]
fn a_goal_is_written_in_minutes_for_the_two_time_statistics() {
    assert_eq!(goal_text(StatType::PursuitLength, 125.0), "2:05");
    assert_eq!(goal_text(StatType::PursuitEvasionTime, 60.0), "1:00");
    assert_eq!(goal_text(StatType::RoadblocksDodged, 20.0), "20");
}

fn milestone_def(key: u32, bin: u8) -> MilestoneDef {
    MilestoneDef { key, bin, stat: StatType::RoadblocksDodged, threshold: 12.0, bounty: 8_000 }
}

fn data() -> GameplayData {
    let spawn = Some(Vec3::new(4.0, 0.0, 6.0));
    let milestone = |key: u32| Milestone {
        key,
        name: format!("milestones/bin_14/{key}"),
        bin: 14,
        kind: None,
        statistic: "roadblocks_dodged".into(),
        threshold: 12.0,
        bounty: 8_000,
        spawn_point: Some(1),
        spawn,
        unconfirmed_goal: 0.0,
        unconfirmed_step: 0.0,
    };
    GameplayData {
        milestones: vec![milestone(1), milestone(2)],
        milestone_types: vec![MilestoneType {
            name: StatType::RoadblocksDodged.name().into(),
            kind: 1,
            per_pursuit: false,
            label: Some(6),
        }],
        ..Default::default()
    }
}

#[test]
fn the_milestone_rows_are_the_milestones_then_the_traps_of_the_bin() {
    let mut set = MilestoneSet::new(&[milestone_def(1, 14), milestone_def(2, 14), milestone_def(3, 13)]);
    set.enter_bin(14);
    let trap = SpeedTrap::new(SpeedTrapDef { key: 9, bin: 14, index: 1, threshold_kmh: 180, bounty: 4_000 });
    let other = SpeedTrap::new(SpeedTrapDef { key: 10, bin: 13, index: 1, threshold_kmh: 180, bounty: 4_000 });
    let rows = milestone_rows(&set, &[trap, other], &data(), 14);
    assert_eq!(rows.len(), 3, "two milestones and one trap of bin 14");
    assert!(matches!(rows[0].entry, Entry::Pursuit { label: Some(6), .. }));
    assert!(matches!(rows[2].entry, Entry::Trap { threshold: 180, .. }));
    assert_eq!(rows[0].spawn, Some([6.0, -4.0]));
    assert_eq!(rows[2].spawn, None, "the trap has no spawn point in this data");
}

#[test]
fn a_milestone_is_locked_open_or_done_by_its_state() {
    let mut set = MilestoneSet::new(&[milestone_def(1, 14)]);
    let status = |set: &MilestoneSet| milestone_rows(set, &[], &data(), 14)[0].status();
    assert_eq!(status(&set), Status::Locked);
    set.enter_bin(14);
    assert_eq!(set.milestones()[0].state, MilestoneState::Active);
    assert_eq!(status(&set), Status::Open);
}

#[test]
fn a_trap_not_yet_unlocked_is_locked() {
    let trap = SpeedTrap::new(SpeedTrapDef { key: 9, bin: 14, index: 1, threshold_kmh: 180, bounty: 4_000 });
    let set = MilestoneSet::new(&[]);
    let rows = milestone_rows(&set, std::slice::from_ref(&trap), &GameplayData::default(), 14);
    assert_eq!(rows[0].status(), Status::Locked);
}

#[test]
fn a_recorded_speed_is_written_in_the_players_unit() {
    assert_eq!(speed_text(Some(100.0), true, "KPH"), "100 KPH");
    assert_eq!(speed_text(Some(100.0), false, "MPH"), "62 MPH");
    assert_eq!(speed_text(None, true, "KPH"), "--");
    assert_eq!(speed_text(Some(0.0), false, "MPH"), "--");
}
