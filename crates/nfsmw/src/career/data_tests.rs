//! Tests of the career table conversion: synthetic inputs for [`CareerData::from_parts`] and an install test.

use blackbox_attrib::vlt_hash;
use game_install::GameDir;
use nfsmw_career::blacklist::BinDef;
use nfsmw_career::carlot::LotClass;
use nfsmw_career::milestones::speedtrap::def_for;
use nfsmw_career::milestones::{MilestoneDef, StatType};
use nfsmw_data::career_cars::{CarClass, CatalogueCar};
use nfsmw_data::gameplay::{self, GameplayData, Race};
use nfsmw_data::pursuit::{CareerHeat, HEAT_LEVELS};
use nfsmw_data::rank_thresholds::{RankThresholds, STATISTICS, THRESHOLDS};

use super::{CareerData, event_id};

/// A race with a collection key and a data name; everything else empty.
fn race(key: u32, name: &str) -> Race {
    Race {
        key,
        name: name.to_owned(),
        event: None,
        event_icon: String::new(),
        race_type: None,
        region: None,
        laps: 0,
        start: None,
        finish: None,
        checkpoints: Vec::new(),
        barriers: Vec::new(),
        params: Default::default(),
        story: Default::default(),
    }
}

/// A Blacklist bin with the given number, counts and race keys.
fn bin(number: i32, world: Vec<u32>, boss: Vec<u32>) -> gameplay::Bin {
    gameplay::Bin {
        number,
        required_races_won: 1,
        required_challenges: 2,
        required_bounty: 3,
        world_races: world,
        boss_races: boss,
        ..Default::default()
    }
}

/// A milestone of `bin` tracking `statistic`.
fn milestone(key: u32, bin: u8, statistic: &str, bounty: i32) -> gameplay::Milestone {
    gameplay::Milestone {
        key,
        name: format!("milestones/bin_{bin:02}/challenge_{key}"),
        bin,
        kind: None,
        statistic: statistic.to_owned(),
        threshold: 42.5,
        bounty,
        spawn_point: None,
        unconfirmed_goal: 0.0,
        unconfirmed_step: 0.0,
    }
}

/// A speed trap of `bin` with a data name ending in `number`.
fn trap(key: u32, bin: u8, name: &str) -> gameplay::SpeedTrap {
    gameplay::SpeedTrap {
        key,
        name: name.to_owned(),
        bin,
        threshold_kmh: 100.0,
        bounty: 500,
        position: None,
        spawn_point: None,
        radius: None,
        unconfirmed_field: None,
    }
}

/// A car of the lot with every field the rules read.
fn car(key: u32, name: Option<&str>, price: i32, unlock_rank: Option<u8>, class: Option<CarClass>) -> CatalogueCar {
    CatalogueCar { key, name: name.map(str::to_owned), price, for_sale: true, unlock_rank, class, manufacturer: None }
}

fn heat() -> CareerHeat {
    CareerHeat {
        evade_factor: Some(0.95),
        race_win_factor: None,
        milestone_factor: None,
        growth: std::array::from_fn(|_| None),
    }
}

fn ranks() -> RankThresholds {
    RankThresholds { lifetime: [[1.0; THRESHOLDS]; STATISTICS], single: [[2.0; THRESHOLDS]; STATISTICS] }
}

/// Gameplay with two races of bin 1, one boss race and one world race, and the rows the tests change.
fn gameplay_with(
    bins: Vec<gameplay::Bin>,
    milestones: Vec<gameplay::Milestone>,
    traps: Vec<gameplay::SpeedTrap>,
) -> GameplayData {
    GameplayData {
        races: vec![race(7, "race_bin_01/1_5_2_speedtrap"), race(8, "race_bin_01/1_7_3_drag")],
        bins,
        milestones,
        speed_traps: traps,
        ..Default::default()
    }
}

#[test]
fn a_bin_keeps_its_counts_and_maps_race_keys_to_event_ids() {
    let gameplay = gameplay_with(vec![bin(1, vec![7], vec![8])], Vec::new(), Vec::new());
    let data = CareerData::from_parts(gameplay, &[], None, None);
    assert_eq!(
        data.bins,
        vec![BinDef {
            number: 1,
            races: vec!["1.5.2".to_owned()],
            races_needed: 1,
            challenges_needed: 2,
            bounty_needed: 3,
            rival_events: vec!["1.7.3".to_owned()],
        }]
    );
}

#[test]
fn bins_outside_one_to_fifteen_and_unknown_race_keys_are_skipped() {
    let gameplay = gameplay_with(
        vec![bin(16, vec![7], Vec::new()), bin(1, vec![7, 999], Vec::new()), bin(-3, Vec::new(), Vec::new())],
        Vec::new(),
        Vec::new(),
    );
    let data = CareerData::from_parts(gameplay, &[], None, None);
    let numbers: Vec<u8> = data.bins.iter().map(|b| b.number).collect();
    assert_eq!(numbers, vec![1]);
    assert_eq!(data.bins[0].races, vec!["1.5.2".to_owned()]);
}

#[test]
fn a_negative_bin_count_drops_the_bin() {
    let mut negative = bin(2, Vec::new(), Vec::new());
    negative.required_bounty = -1;
    let data = CareerData::from_parts(gameplay_with(vec![negative], Vec::new(), Vec::new()), &[], None, None);
    assert!(data.bins.is_empty());
}

#[test]
fn event_ids_come_from_the_first_three_numbers_of_the_leaf() {
    assert_eq!(event_id("race_bin_10/10_2_1_sprint"), "10.2.1");
    assert_eq!(event_id("1_5_2"), "1.5.2");
    assert_eq!(event_id("race_bin_01/epic_pursuit"), "epic_pursuit");
    assert_eq!(event_id("race_bin_01/1_x_2"), "1_x_2");
}

#[test]
fn a_milestone_takes_its_statistic_by_name() {
    let gameplay = gameplay_with(
        Vec::new(),
        vec![milestone(11, 1, "pursuit_length", 250), milestone(12, 15, "cops_destroyed_in_pursuit", 0)],
        Vec::new(),
    );
    let data = CareerData::from_parts(gameplay, &[], None, None);
    assert_eq!(
        data.milestones,
        vec![
            MilestoneDef { key: 11, bin: 1, stat: StatType::PursuitLength, threshold: 42.5, bounty: 250 },
            MilestoneDef { key: 12, bin: 15, stat: StatType::CopsDestroyedInPursuit, threshold: 42.5, bounty: 0 },
        ]
    );
}

#[test]
fn milestones_with_an_unknown_statistic_bin_or_bounty_are_skipped() {
    let gameplay = gameplay_with(
        Vec::new(),
        vec![
            milestone(1, 1, "no_such_statistic", 10),
            milestone(2, 0, "pursuit_length", 10),
            milestone(3, 16, "pursuit_length", 10),
            milestone(4, 1, "pursuit_length", -10),
            milestone(5, 1, "pursuit_length", 10),
        ],
        Vec::new(),
    );
    let data = CareerData::from_parts(gameplay, &[], None, None);
    let keys: Vec<u32> = data.milestones.iter().map(|m| m.key).collect();
    assert_eq!(keys, vec![5]);
}

#[test]
fn a_speed_trap_takes_its_number_from_the_name_and_the_table_row() {
    let gameplay = gameplay_with(Vec::new(), Vec::new(), vec![trap(77, 1, "speedtraps/bin_01/speedtrap2")]);
    let data = CareerData::from_parts(gameplay, &[], None, None);
    let expected = def_for(1, 2, 77).expect("bin 1 has trap 2");
    assert_eq!(data.traps, vec![expected]);
    assert_eq!((expected.key, expected.bin, expected.index), (77, 1, 2));
}

#[test]
fn a_speed_trap_without_a_trailing_number_or_table_row_is_skipped() {
    let gameplay = gameplay_with(
        Vec::new(),
        Vec::new(),
        vec![
            trap(1, 1, "speedtraps/bin_01/speedtrap"),
            trap(2, 16, "speedtraps/bin_16/speedtrap1"),
            trap(3, 15, "speedtraps/bin_15/speedtrap3"),
        ],
    );
    let data = CareerData::from_parts(gameplay, &[], None, None);
    assert!(data.traps.is_empty());
}

#[test]
fn a_car_takes_its_fields_and_the_name_hash_as_handle_and_vehicle_key() {
    let cars = [car(5, Some("cobaltss"), 25_000, Some(16), Some(CarClass::Europe))];
    let data = CareerData::from_parts(GameplayData::default(), &cars, None, None);
    let lot = &data.cars[0];
    assert_eq!(lot.name, "cobaltss");
    assert_eq!((lot.frontend_key, lot.price, lot.unlock_rank), (5, 25_000, 16));
    assert_eq!(lot.class, LotClass::Europe);
    assert!(lot.for_sale);
    assert_eq!((lot.handle, lot.vehicle_key), (vlt_hash("cobaltss"), vlt_hash("cobaltss")));
}

#[test]
fn a_car_without_name_rank_or_class_is_skipped_and_a_negative_price_is_zero() {
    let cars = [
        car(1, None, 100, Some(2), Some(CarClass::Japan)),
        car(2, Some("a"), 100, None, Some(CarClass::Japan)),
        car(3, Some("b"), 100, Some(2), None),
        car(4, Some("c"), -50, Some(2), Some(CarClass::Root)),
    ];
    let data = CareerData::from_parts(GameplayData::default(), &cars, None, None);
    assert_eq!(data.cars.len(), 1);
    assert_eq!(data.cars[0].name, "c");
    assert_eq!(data.cars[0].price, 0);
    assert_eq!(data.cars[0].class, LotClass::Vehicles);
}

#[test]
fn the_stats_are_parallel_to_the_cars_and_empty_without_an_install() {
    let cars = [
        car(1, Some("a"), 100, Some(2), Some(CarClass::Japan)),
        car(2, None, 100, Some(2), Some(CarClass::Japan)),
        car(3, Some("c"), 100, Some(2), Some(CarClass::Europe)),
    ];
    let data = CareerData::from_parts(GameplayData::default(), &cars, None, None);
    assert_eq!(data.cars.len(), 2);
    assert_eq!(data.stats, vec![None, None], "no physics read without an install");
}

#[test]
fn heat_and_ranks_pass_through_and_stay_none_when_missing() {
    let with = CareerData::from_parts(GameplayData::default(), &[], Some(heat()), Some(ranks()));
    assert_eq!(with.heat.as_ref().and_then(|h| h.evade_factor), Some(0.95));
    assert_eq!(with.ranks, Some(ranks()));
    assert_eq!(with.heat.as_ref().map(|h| h.growth.len()), Some(HEAT_LEVELS as usize));

    let without = CareerData::from_parts(GameplayData::default(), &[], None, None);
    assert!(without.heat.is_none());
    assert!(without.ranks.is_none());
}

#[test]
fn the_gameplay_data_is_kept_whole() {
    let gameplay = gameplay_with(vec![bin(1, vec![7], Vec::new())], Vec::new(), Vec::new());
    let races = gameplay.races.len();
    let data = CareerData::from_parts(gameplay, &[], None, None);
    assert_eq!(data.gameplay.races.len(), races);
}

/// The install test: the counts the specs give. Runs only with `--ignored` and `NFSMW_GAME_DIR` set to the install.
#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn the_install_gives_fifteen_bins_fifty_five_milestones_and_thirty_nine_traps() {
    let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else {
        return;
    };
    let dir = GameDir::open(std::path::PathBuf::from(path)).expect("NFSMW_GAME_DIR cannot be indexed");
    let data = CareerData::load(&dir).expect("career data");
    assert_eq!(data.bins.len(), 15);
    assert_eq!(data.milestones.len(), 55);
    assert_eq!(data.traps.len(), 39);
}
