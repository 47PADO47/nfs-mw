//! The pool filter, the random pick and the brief's text: pure, no game data needed.

use nfsmw_career::carlot::stats::CarStats;
use nfsmw_data::career_cars::{CarClass, CatalogueCar};
use nfsmw_data::gameplay::RaceType;

use super::pool::{self, PoolCar, Rng, UPCOMING};
use super::text::{self, Unlock};
use crate::frontend::quick_race::Track;

fn car(name: &str, rank: u8) -> PoolCar {
    PoolCar {
        name: name.into(),
        folder: name.to_ascii_uppercase(),
        rank,
        handle: 1,
        manufacturer: Some(1),
        stats: None,
    }
}

fn track(name: &str, bin: Option<u8>) -> Track {
    Track { bin, ..Track::sample(name) }
}

#[test]
fn the_pool_reaches_two_positions_past_the_stage() {
    assert_eq!(UPCOMING, 2);
    // The prologue: ranks 16 (available), 15 and 14 (the two upcoming) are in; 13 is not.
    assert!(pool::car_offered(16, 16));
    assert!(pool::car_offered(15, 16));
    assert!(pool::car_offered(14, 16));
    assert!(!pool::car_offered(13, 16));
    // Stage 10: rank 8 is the lowest.
    assert!(pool::car_offered(8, 10));
    assert!(!pool::car_offered(7, 10));
    // The last rival: every rank is in, and the stage cannot go below zero.
    assert!(pool::car_offered(1, 1));
    assert!(pool::car_offered(0, 1));
    assert!(pool::car_offered(0, 0));
}

#[test]
fn a_car_is_the_players_when_its_rank_is_at_least_the_stage() {
    assert!(pool::car_unlocked(16, 16));
    assert!(pool::car_unlocked(12, 12));
    assert!(!pool::car_unlocked(14, 16));
}

#[test]
fn races_follow_the_cars_rule_by_bin_and_races_outside_the_bins_are_never_in() {
    assert!(pool::race_offered(Some(15), 16));
    assert!(pool::race_offered(Some(14), 16));
    assert!(!pool::race_offered(Some(13), 16));
    assert!(pool::race_offered(Some(3), 5));
    assert!(!pool::race_offered(Some(2), 5));
    assert!(!pool::race_offered(None, 1));
    assert!(!pool::race_offered(None, 16));
}

#[test]
fn a_catalogue_car_needs_a_price_a_car_class_and_a_folder() {
    let entry = |name: &str, price: i32, for_sale: bool, class: CarClass| CatalogueCar {
        key: 1,
        name: Some(name.into()),
        price,
        for_sale,
        unlock_rank: Some(12),
        class: Some(class),
        manufacturer: Some(19),
    };
    let catalogue = [
        entry("cobaltss", 26_000, true, CarClass::NorthAmerica),
        entry("bmw", 0, true, CarClass::Europe),
        entry("sl500", 75_000, false, CarClass::Europe),
        entry("vehicles", 5, true, CarClass::Root),
        entry("carrera_gt", 280_000, true, CarClass::Europe),
        entry("viper", 98_000, true, CarClass::NorthAmerica),
    ];
    let folders: Vec<String> = ["COBALTSS", "BMW", "SL500", "CARRERAGT"].map(String::from).into();
    let cars = pool::cars_of(&catalogue, &folders, |_| None);
    let names: Vec<(&str, &str)> = cars.iter().map(|c| (c.name.as_str(), c.folder.as_str())).collect();
    assert_eq!(names, [("cobaltss", "COBALTSS"), ("carrera_gt", "CARRERAGT")]);
    assert_eq!(cars[0].manufacturer, Some(19));
}

#[test]
fn a_car_with_no_physics_is_not_offered_once_the_physics_are_read() {
    let stats = CarStats { top_speed: 80.0, handling: 1.0, time_to_100: 9.0 };
    let with = PoolCar { stats: Some(stats), ..car("cobaltss", 12) };
    let without = PoolCar { stats: None, ..car("levin", 12) };
    let cars = vec![with.clone(), without.clone()];
    assert_eq!(
        pool::drivable(cars.clone(), true),
        std::slice::from_ref(&with),
        "levin has no pvehicle, so it cannot be driven"
    );
    assert_eq!(pool::drivable(cars, false), [with, without], "without the physics database nothing is known");
}

#[test]
fn a_front_end_name_finds_its_folder_with_or_without_underscores_and_suffix() {
    let folders: Vec<String> = ["A3", "A4", "CARRERAGT", "CORVETTE", "COBALTSS"].map(String::from).into();
    assert_eq!(pool::folder_for("cobaltss", &folders).as_deref(), Some("COBALTSS"));
    assert_eq!(pool::folder_for("carrera_gt", &folders).as_deref(), Some("CARRERAGT"));
    assert_eq!(pool::folder_for("a4_32", &folders).as_deref(), Some("A4"));
    assert_eq!(pool::folder_for("a3_20t", &folders).as_deref(), Some("A3"));
    assert_eq!(pool::folder_for("corvette_car", &folders).as_deref(), Some("CORVETTE"));
    assert_eq!(pool::folder_for("viper", &folders), None);
}

#[test]
fn the_same_seed_gives_the_same_numbers_and_they_stay_in_range() {
    let mut a = Rng::new(7);
    let mut b = Rng::new(7);
    let mut c = Rng::new(8);
    let run = |rng: &mut Rng| (0..16).map(|_| rng.below(10)).collect::<Vec<_>>();
    let first = run(&mut a);
    assert_eq!(first, run(&mut b));
    assert_ne!(first, run(&mut c));
    assert!(first.iter().all(|n| *n < 10));
    assert_eq!(Rng::new(0).below(0), 0);
}

#[test]
fn the_pick_stays_inside_the_stage_and_covers_the_whole_pool() {
    let cars = [car("a", 16), car("b", 15), car("c", 14), car("d", 13), car("e", 2)];
    let races = [track("r16", None), track("r15", Some(15)), track("r14", Some(14)), track("r5", Some(5))];
    let mut rng = Rng::new(1);
    let mut seen_cars = std::collections::BTreeSet::new();
    let mut seen_races = std::collections::BTreeSet::new();
    for _ in 0..400 {
        let pick = pool::pick(&cars, &races, 16, &mut rng, None).expect("a pick");
        seen_cars.insert(pick.car);
        seen_races.insert(pick.race);
    }
    assert_eq!(seen_cars.into_iter().collect::<Vec<_>>(), [0, 1, 2], "ranks 16, 15 and 14 only");
    assert_eq!(seen_races.into_iter().collect::<Vec<_>>(), [1, 2], "bins 15 and 14 only, not the race with no bin");
}

#[test]
fn randomize_does_not_repeat_the_pair_when_there_is_a_choice() {
    let cars = [car("a", 16), car("b", 16)];
    let races = [track("x", Some(15))];
    let mut rng = Rng::new(3);
    let mut last = pool::pick(&cars, &races, 16, &mut rng, None);
    for _ in 0..50 {
        let next = pool::pick(&cars, &races, 16, &mut rng, last);
        assert_ne!(next, last);
        last = next;
    }
    // One pair only: it is picked again rather than looping.
    let one = pool::pick(&cars[..1], &races, 16, &mut rng, last).expect("a pick");
    assert_eq!(pool::pick(&cars[..1], &races, 16, &mut rng, Some(one)), Some(one));
}

#[test]
fn nothing_is_picked_without_a_car_or_a_race_in_range() {
    let cars = [car("e", 2)];
    let races = [track("r", Some(15))];
    let mut rng = Rng::new(1);
    assert_eq!(pool::pick(&cars, &races, 16, &mut rng, None), None);
    assert_eq!(pool::pick(&[], &races, 16, &mut rng, None), None);
    assert_eq!(pool::pick(&cars, &[], 1, &mut rng, None), None);
}

#[test]
fn the_length_reads_in_miles_or_kilometres_with_one_decimal() {
    assert_eq!(text::length_text(11_105.0, false, "Miles"), "6.9 Miles");
    assert_eq!(text::length_text(11_105.0, true, "KMs"), "11.1 KMs");
    assert_eq!(text::length_text(804.67, false, "Miles"), "0.5 Miles");
    assert_eq!(text::length_text(0.0, true, "KMs"), "0.0 KMs");
}

#[test]
fn the_unit_words_are_the_games_labels() {
    assert_ne!(text::unit_label(true), text::unit_label(false));
}

#[test]
fn each_race_kind_has_its_own_icon_and_name() {
    let kinds = [
        RaceType::Sprint,
        RaceType::Circuit,
        RaceType::Drag,
        RaceType::Knockout,
        RaceType::Tollbooth,
        RaceType::SpeedTrap,
    ];
    let icons: std::collections::BTreeSet<u32> = kinds.iter().map(|k| text::mode_icon(*k)).collect();
    let labels: std::collections::BTreeSet<u32> = kinds.iter().map(|k| text::mode_label(*k)).collect();
    assert_eq!((icons.len(), labels.len()), (6, 6));
    assert_eq!(text::mode_icon(RaceType::Sprint), 0x2521_E5EB, "the package's own Mode_Icon_Sprint");
}

#[test]
fn a_track_label_is_the_event_id_with_underscores() {
    assert_eq!(text::track_label_key("10.2.1"), "TRACKNAME_10_2_1");
}

#[test]
fn the_logos_are_named_after_the_manufacturer_and_the_front_end_name() {
    let mut cobalt = car("cobaltss", 16);
    cobalt.manufacturer = Some(19);
    assert_eq!(text::manufacturer_texture(&cobalt), "CARSELECT_MANUFACTURER_CHEVROLET");
    assert_eq!(text::model_texture(&cobalt), "SECONDARY_LOGO_cobaltss");
    cobalt.manufacturer = Some(7);
    assert_eq!(text::manufacturer_texture(&cobalt), "CARSELECT_MANUFACTURER_GENERAL_MOTORS");
    cobalt.manufacturer = None;
    assert_eq!(text::manufacturer_texture(&cobalt), "GENERIC_LOGO_128");
    assert_eq!(text::model_texture(&cobalt), "GENERIC_LOGO_256");
    assert_eq!(text::manufacturer(0), None);
    assert_eq!(text::manufacturer(1), Some("BMW"));
    assert_eq!(text::manufacturer(25), Some("CORVETTE"));
    assert_eq!(text::manufacturer(26), None);
}

#[test]
fn the_unlock_line_follows_the_handle_then_the_rank() {
    // A car the profile has hides the line, whatever its handle.
    assert_eq!(text::unlock_of(&car("a", 12), 12), Unlock::None);
    let with_handle = |handle: u32, rank: u8| PoolCar { handle, ..car("x", rank) };
    // The bonus cars name their own condition (the sentence table in `docs/provenance/quick-play.md`).
    assert_eq!(text::unlock_of(&with_handle(0x2CF3_70F0, 1), 5), Unlock::Label(0xBD8B_AC94));
    assert_eq!(text::unlock_of(&with_handle(0x03A9_4520, 1), 5), Unlock::Label(0xBD8B_AC93));
    assert_eq!(text::unlock_of(&with_handle(0x2CF3_85B2, 1), 5), Unlock::Label(0xBD8B_AC92));
    assert_eq!(text::unlock_of(&with_handle(0xCB6A_AF2F, 1), 5), Unlock::Label(0xBD8B_AC91));
    // A rival's car names the rival (0x965F is member 2, 0x13624E member 10, 0x136253 member 15).
    assert_eq!(text::unlock_of(&with_handle(0x0000_965F, 1), 5), Unlock::Rival(2));
    assert_eq!(text::unlock_of(&with_handle(0x0013_624E, 1), 12), Unlock::Rival(10));
    assert_eq!(text::unlock_of(&with_handle(0x0013_6253, 1), 16), Unlock::Rival(15));
    // Any other locked car is opened by the member after its rank.
    assert_eq!(text::unlock_of(&car("a3", 14), 16), Unlock::Rival(15));
    assert_eq!(text::unlock_of(&car("b", 9), 10), Unlock::Rival(10));
    // Rank 15 at the prologue has no member to name.
    assert_eq!(text::unlock_of(&car("c", 15), 16), Unlock::None);
}

#[test]
fn the_handle_tables_match_the_exe() {
    assert_eq!(text::handle_label(0x0013_624D), None);
    assert_eq!(text::handle_label(0x0013_624E), Some(text::RIVAL_LABEL));
    assert_eq!(text::handle_label(0x0000_9666), Some(text::RIVAL_LABEL));
    assert_eq!(text::handle_label(0x0000_9667), None);
    assert_eq!(text::handle_member(0x0000_9666), Some(9));
    assert_eq!(text::handle_member(0x0013_624F), Some(11));
    assert_eq!(text::handle_member(0x2CF3_70F0), None);
}

#[test]
fn the_rival_sentence_fills_the_alias_and_the_rank() {
    let line = text::fill_sentence("Unlock this car by beating %s, Blacklist member %d.", "MING", 15);
    assert_eq!(line, "Unlock this car by beating MING, Blacklist member 15.");
    assert_eq!(text::alias_key(5), "BLACKLIST_RIVAL_05_AKA");
    assert_eq!(text::alias_key(15), "BLACKLIST_RIVAL_15_AKA");
}
