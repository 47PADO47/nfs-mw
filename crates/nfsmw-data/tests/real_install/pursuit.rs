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
fn the_siren_rows_are_read_for_every_heat_level() {
    use nfsmw_data::pursuit_siren::siren_data;
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    let data = siren_data(&db);
    assert_eq!(data.rows.len(), 10);
    for (level, row) in data.rows.iter().enumerate() {
        println!("heat {}: {row:?}", level + 1);
        assert!(row.wail_period > 0.0 && row.max_yelp_time > 0.0, "heat {} has no wail/yelp times", level + 1);
        assert!(row.scream_period > 0.0 && row.max_scream_time > 0.0, "heat {} has no scream times", level + 1);
        assert!(row.init_min_period > 0.0, "heat {} has no opening gap", level + 1);
    }
    println!("intensity: heat {} or {} s", data.high_intensity_heat, data.high_intensity_secs);
    assert_eq!(data.high_intensity_heat, 2);
    assert_eq!(data.high_intensity_secs, 150.0);
}
