//! The cop waves read from the install match the tables in `docs/specs/ai-pursuit-heat.md` §2.7.

use blackbox_attrib::Database;
use nfsmw_data::pursuit::{ai_vehicle, heat_row, patrol_speeds_mph};

use crate::install;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_helicopter_record_has_no_car_links() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    let heli = db.collection("pvehicle", "copheli").expect("the copheli record");
    for link in ["chassis", "tires", "brakes"] {
        assert!(heli.follow(link).is_none(), "copheli has a {link} link");
    }
    let car = db.collection("pvehicle", "bmwm3gtr").expect("the bmwm3gtr record");
    assert!(car.follow("chassis").is_some(), "a car has a chassis link");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_cop_waves_match_the_documented_table() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    let wave = |heat| {
        let row = heat_row(&db, heat).expect("a row");
        row.cops.iter().map(|c| (c.name.clone(), c.count, c.chance)).collect::<Vec<_>>()
    };
    assert_eq!(wave(1), vec![("copmidsize".to_owned(), 4, 100)]);
    assert_eq!(wave(3), vec![("copgto".to_owned(), 7, 100)]);
    assert_eq!(wave(4), vec![("copgtoghost".to_owned(), 8, 100), ("copheli".to_owned(), 1, 50)]);
    assert_eq!(wave(6)[0], ("copsportghost".to_owned(), 8, 80));
    assert_eq!(heat_row(&db, 1).unwrap().full_engagement_cops, 5);
    assert_eq!(heat_row(&db, 5).unwrap().full_engagement_cops, 25);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn cop_cars_have_their_documented_speed_multipliers() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    // docs/specs/ai-driver-control.md section 10.
    let sport = ai_vehicle(&db, "copsport");
    assert_eq!((sport.max_speed_kmh, sport.acceleration_multiplier, sport.top_speed_multiplier), (400.0, 1.5, 2.0));
    let midsize = ai_vehicle(&db, "copmidsize");
    assert_eq!(
        (midsize.max_speed_kmh, midsize.acceleration_multiplier, midsize.top_speed_multiplier),
        (280.0, 1.05, 1.6)
    );
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_patrol_speeds_are_the_documented_defaults() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    assert_eq!(patrol_speeds_mph(&db), (50.0, 71.0));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_backup_rows_match_the_documented_table() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    let rows = nfsmw_data::pursuit::backup_rows(&db);
    assert_eq!(rows.len(), 10);
    let radius: Vec<f32> = rows.iter().map(|r| r.engagement_radius).collect();
    let trigger: Vec<u32> = rows.iter().map(|r| r.trigger).collect();
    let timer: Vec<f32> = rows.iter().map(|r| r.call_timer).collect();
    assert_eq!(radius, [50.0, 50.0, 50.0, 50.0, 50.0, 50.0, 50.0, 200.0, 50.0, 200.0]);
    assert_eq!(trigger, [2, 2, 2, 2, 2, 2, 2, 3, 5, 5]);
    assert_eq!(timer, [180.0, 120.0, 120.0, 120.0, 90.0, 90.0, 90.0, 60.0, 45.0, 60.0]);
}
