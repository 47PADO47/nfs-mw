//! The career data readers against the install: the Blacklist bins, the milestones, speed traps and milestone types
//! (`gameplay`), the car lot (`career_cars`), the cop heat (`pursuit`) and the rap sheet thresholds
//! (`rank_thresholds`). Specs: `docs/specs/career-blacklist.md` section 1, `docs/specs/career-milestones-data.md`,
//! `docs/specs/career-car-lot.md` section 2, `docs/specs/ai-pursuit-heat.md` sections 2.3 and 3.2.

use blackbox_attrib::Database;
use game_install::GameDir;
use nfsmw_data::career_cars::{self, CarClass};
use nfsmw_data::gameplay::{self, GameplayData};
use nfsmw_data::pursuit::career_heat;
use nfsmw_data::rank_thresholds::rank_thresholds;
use nfsmw_data::shared;

use crate::{install, unwrapped};

/// `docs/specs/career-blacklist.md` section 1 and `career-milestones-data.md` table 4: for bins 1 to 15, the races
/// won, the challenges passed and the total bounty that open the rival (`RequiredRacesWon`, `RequiredChallenges`,
/// `RequiredBounty`). Row: `(bin, races won, challenges, bounty)`.
const RIVAL_TABLE: [(u8, i32, i32, i32); 15] = [
    (15, 3, 3, 20_000),
    (14, 4, 3, 50_000),
    (13, 4, 3, 100_000),
    (12, 4, 3, 180_000),
    (11, 5, 3, 300_000),
    (10, 5, 4, 500_000),
    (9, 5, 4, 790_000),
    (8, 5, 4, 1_180_000),
    (7, 7, 4, 1_680_000),
    (6, 7, 4, 2_300_000),
    (5, 7, 4, 3_050_000),
    (4, 7, 5, 4_050_000),
    (3, 8, 5, 5_550_000),
    (2, 8, 5, 7_550_000),
    (1, 9, 5, 10_000_000),
];

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

fn near_opt(a: Option<f32>, b: f32) -> bool {
    a.is_some_and(|a| near(a, b))
}

/// The attribute database with `GLOBAL/gameplay.bin` loaded, as [`gameplay::load`] builds it.
fn gameplay_database(dir: &GameDir) -> Database {
    let attributes = shared::attributes(dir).expect("attributes.bin");
    let mut db = Database::clone(&attributes);
    db.load(&unwrapped(dir, gameplay::GAMEPLAY_FILE)).expect("gameplay.bin");
    db
}

/// The attribute database with `GLOBAL/FE_ATTRIB.bin` loaded, as [`career_cars::load`] builds it.
fn frontend_database(dir: &GameDir) -> Database {
    let attributes = shared::attributes(dir).expect("attributes.bin");
    let mut db = Database::clone(&attributes);
    db.load(&unwrapped(dir, career_cars::FE_ATTRIB_FILE)).expect("FE_ATTRIB.bin");
    db
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_blacklist_bins_are_present_with_the_documented_requirements() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");

    for number in 1..=16 {
        assert!(data.bins.iter().any(|b| b.number == number), "bin {number} is missing");
    }
    for (bin, races, challenges, bounty) in RIVAL_TABLE {
        let b = data.bins.iter().find(|b| b.number == i32::from(bin)).expect("bin");
        assert_eq!(
            (b.required_races_won, b.required_challenges, b.required_bounty),
            (races, challenges, bounty),
            "bin {bin}"
        );
    }

    // career-blacklist.md section 1: bin 15 has 3 world races, the first rival of the career.
    let bin15 = data.bins.iter().find(|b| b.number == 15).expect("bin 15");
    assert_eq!(bin15.world_races.len(), 3);
    // Bin 16 (the prologue) has only RequiredRacesWon 4.
    let bin16 = data.bins.iter().find(|b| b.number == 16).expect("bin 16");
    assert_eq!(bin16.required_races_won, 4);
    // Bin 1 lists six boss races.
    let bin1 = data.bins.iter().find(|b| b.number == 1).expect("bin 1");
    assert_eq!(bin1.boss_races.len(), 6);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_bin_barrier_lists_give_the_documented_gate_sets() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");

    // Same sets as crates/nfsmw-career/src/regions/gates/tests.rs: bins 13 to 16 hold BARRIER_LOCKED_11..15,
    // bins 9 to 12 hold BARRIER_LOCKED_10, 12 and 13, bins 1 to 8 hold none. Order does not matter.
    let names = |numbers: &[u8]| {
        let mut out: Vec<String> = numbers.iter().map(|n| format!("BARRIER_LOCKED_{n}")).collect();
        out.sort();
        out
    };
    for bin in data.bins.iter().filter(|b| (1..=16).contains(&b.number)) {
        let want = match bin.number {
            13..=16 => names(&[11, 12, 13, 14, 15]),
            9..=12 => names(&[10, 12, 13]),
            _ => Vec::new(),
        };
        let mut got = bin.barriers.clone();
        got.sort();
        assert_eq!(got, want, "bin {}", bin.number);
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn some_races_enable_track_path_barriers() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    // Race.barriers are the track-path barrier names of each race (career-regions.md 3.4), not the bin lists.
    assert!(data.races.iter().any(|r| !r.barriers.is_empty()), "no race has a Barriers entry");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_milestones_speed_traps_and_types_have_the_documented_counts() {
    let Some(dir) = install() else { return };
    let data: GameplayData = gameplay::load(&dir).expect("gameplay data");

    // Counts are from docs/specs/career-milestones-data.md. A modded install may differ; the spec's counts are the
    // stock ones.
    assert_eq!(data.milestones.len(), 55, "milestones");
    assert_eq!(data.speed_traps.len(), 39, "speed traps");
    assert_eq!(data.milestone_types.len(), 30, "milestone types");

    // Bins 1 to 10 have four milestones each, bins 11 to 15 three.
    for bin in 1..=15u8 {
        let want = if bin <= 10 { 4 } else { 3 };
        let got = data.milestones.iter().filter(|m| m.bin == bin).count();
        assert_eq!(got, want, "milestones of bin {bin}");
    }
    // Bins 1 to 10 have three speed traps each, bins 11 to 14 two, bin 15 one.
    for bin in 1..=15u8 {
        let want = match bin {
            1..=10 => 3,
            11..=14 => 2,
            _ => 1,
        };
        let got = data.speed_traps.iter().filter(|t| t.bin == bin).count();
        assert_eq!(got, want, "speed traps of bin {bin}");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn bin_one_milestones_and_speed_traps_match_the_documented_values() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");

    // career-milestones-data.md table 2: bin 1 cops_damaged completes at 35, bounty 220000.
    let damaged = data
        .milestones
        .iter()
        .find(|m| m.bin == 1 && m.kind.as_deref() == Some("cops_damaged"))
        .expect("bin 1 cops_damaged");
    assert!(near(damaged.threshold, 35.0), "threshold {}", damaged.threshold);
    assert_eq!(damaged.bounty, 220_000);

    // Bin 1 pursuit_evasion_time completes at 120 seconds.
    let evade = data
        .milestones
        .iter()
        .find(|m| m.bin == 1 && m.kind.as_deref() == Some("pursuit_evasion_time"))
        .expect("bin 1 pursuit_evasion_time");
    assert!(near(evade.threshold, 120.0), "threshold {}", evade.threshold);

    // career-milestones-data.md table 3: bin 1 speed traps 320, 225, 241 km/h, bounty 136000 each. Names sort in
    // trap order (speedtrap1 to speedtrap3).
    let mut traps: Vec<_> = data.speed_traps.iter().filter(|t| t.bin == 1).collect();
    traps.sort_by(|a, b| a.name.cmp(&b.name));
    let speeds: Vec<f32> = traps.iter().map(|t| t.threshold_kmh).collect();
    assert_eq!(speeds, vec![320.0, 225.0, 241.0]);
    assert!(traps.iter().all(|t| t.bounty == 136_000));

    // Bin 15 has one speed trap at 130 km/h, bounty 4000.
    let last = data.speed_traps.iter().find(|t| t.bin == 15).expect("bin 15 trap");
    assert!(near(last.threshold_kmh, 130.0));
    assert_eq!(last.bounty, 4_000);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_milestone_types_say_which_way_is_better() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    let by_name = |name: &str| data.milestone_types.iter().find(|t| t.name == name).expect(name);

    // career-milestones-data.md table 1: pursuit_evasion_time is type 2 (smaller is better), per pursuit, label 2.
    let evade = by_name("pursuit_evasion_time");
    assert_eq!(evade.kind, 2);
    assert!(!evade.bigger_is_better());
    assert!(evade.per_pursuit);
    assert_eq!(evade.label, Some(2));

    // cops_destroyed is type 1, not per pursuit, label 3.
    let destroyed = by_name("cops_destroyed");
    assert!(destroyed.bigger_is_better());
    assert!(!destroyed.per_pursuit);
    assert_eq!(destroyed.label, Some(3));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_rival_requirements_match_the_documented_table() {
    let Some(dir) = install() else { return };
    let db = gameplay_database(&dir);
    let rivals = gameplay::rival_requirements(&db);
    assert_eq!(rivals.len(), 15);
    for (bin, races, challenges, bounty) in RIVAL_TABLE {
        let r = rivals.iter().find(|r| r.bin == bin).expect("rival");
        assert_eq!((r.races_won, r.challenges, r.bounty), (races, challenges, bounty), "bin {bin}");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_car_lot_has_the_documented_prices() {
    let Some(dir) = install() else { return };
    let cars = career_cars::load(&dir).expect("car lot");

    // career-car-lot.md section 2 lists 32 cars for sale. The installed FE_ATTRIB.bin may be modded and add more,
    // so this is a lower bound.
    let for_sale = cars.iter().filter(|c| c.for_sale).count();
    assert!(for_sale >= 32, "for-sale cars: {for_sale}");

    let find = |name: &str| cars.iter().find(|c| c.name.as_deref() == Some(name)).expect(name);

    // Rank 16 entry car: 26,000 dollars, North America.
    let cobalt = find("cobaltss");
    assert!(cobalt.for_sale);
    assert_eq!(cobalt.price, 26_000);
    assert_eq!(cobalt.unlock_rank, Some(16));
    assert_eq!(cobalt.class, Some(CarClass::NorthAmerica));

    // Rank 2 top car: 300,000 dollars, Europe.
    let slr = find("slr");
    assert!(slr.for_sale);
    assert_eq!(slr.price, 300_000);
    assert_eq!(slr.unlock_rank, Some(2));
    assert_eq!(slr.class, Some(CarClass::Europe));

    // The starting BMW M3 GTR has a price field but is not for sale.
    let m3 = find("m3_gtre46");
    assert!(!m3.for_sale);
    assert_eq!(m3.price, 500_000);
    assert_eq!(m3.unlock_rank, Some(15));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_career_heat_factors_and_growth_match_the_documented_values() {
    let Some(dir) = install() else { return };
    let db = shared::attributes(&dir).expect("attributes.bin");
    let heat = career_heat(&db).expect("pursuitlevels default");

    // ai-pursuit-heat.md section 3.2: the default collection's factors.
    assert!(near_opt(heat.evade_factor, 0.95), "evade {:?}", heat.evade_factor);
    assert!(near_opt(heat.race_win_factor, 0.94), "race win {:?}", heat.race_win_factor);
    assert!(near_opt(heat.milestone_factor, 0.93), "milestone {:?}", heat.milestone_factor);

    // Level 1: 1 s per heat level times the bin scale. 600 s in bin 14, 360 s in bin 13, 180 s in bin 11, 120 s in bin 10.
    let level1 = heat.growth[0].expect("level 1 growth");
    assert!(near(level1.time_per_heat_level, 1.0));
    assert!(near(level1.seconds_for_bin(14), 600.0), "bin 14: {}", level1.seconds_for_bin(14));
    assert!(near(level1.seconds_for_bin(13), 360.0), "bin 13: {}", level1.seconds_for_bin(13));
    assert!(near(level1.seconds_for_bin(11), 180.0), "bin 11: {}", level1.seconds_for_bin(11));
    assert!(near(level1.seconds_for_bin(10), 120.0), "bin 10: {}", level1.seconds_for_bin(10));

    // Level 5 always takes 600 s; level 10 takes 2000 s (scale 1 in every bin).
    let level5 = heat.growth[4].expect("level 5 growth");
    assert!(near(level5.seconds_for_bin(0), 600.0));
    let level10 = heat.growth[9].expect("level 10 growth");
    assert!(near(level10.seconds_for_bin(0), 2000.0));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_rap_sheet_thresholds_match_the_default_tables() {
    let Some(dir) = install() else { return };
    let db = frontend_database(&dir);
    let t = rank_thresholds(&db).expect("the 20 ranking tables");

    // crates/nfsmw-career/src/rapsheet/rank.rs LIFETIME: pursuit length 14400 down to 600 s; cost to state 25000 at
    // the last rank; bounty 5000 at the last rank.
    assert_eq!(t.lifetime[0][0], 14_400.0);
    assert_eq!(t.lifetime[0][14], 600.0);
    assert_eq!(t.lifetime[1][0], 1_200.0);
    assert_eq!(t.lifetime[7][14], 25_000.0);
    assert_eq!(t.lifetime[9][14], 5_000.0);

    // rank.rs SINGLE: pursuit length 1800 down to 60 s in one pursuit.
    assert_eq!(t.single[0][0], 1_800.0);
    assert_eq!(t.single[0][14], 60.0);
    assert_eq!(t.single[2][14], 1.0);
    assert_eq!(t.single[9][14], 1_000.0);
}
