use std::sync::Arc;

use blackbox_attrib::vlt_hash;
use blackbox_hash::bstring_hash;
use game_install::GameDir;
use nfsmw_career::carlot::{LotCar, LotClass};
use nfsmw_career::settings::NewCareer;
use nfsmw_career::stable::CarRecord;

use super::*;
use crate::career::{Career, CareerData};

fn data(cars: Vec<LotCar>) -> Arc<CareerData> {
    let bin = |number| nfsmw_career::blacklist::BinDef {
        number,
        races: Vec::new(),
        races_needed: 0,
        challenges_needed: 0,
        bounty_needed: 0,
        rival_events: Vec::new(),
    };
    Arc::new(CareerData {
        bins: (1..=15).map(bin).collect(),
        milestones: Vec::new(),
        traps: Vec::new(),
        stats: vec![None; cars.len()],
        cars,
        heat: Default::default(),
        ranks: Default::default(),
        gameplay: Default::default(),
    })
}

fn lot_car(name: &str, frontend_key: u32) -> LotCar {
    LotCar {
        name: name.to_owned(),
        handle: vlt_hash(name),
        frontend_key,
        vehicle_key: vlt_hash(name),
        price: 1000,
        unlock_rank: 16,
        for_sale: true,
        class: LotClass::Europe,
    }
}

/// A loaded profile whose current car is the record `record`, in the first slot of its stable.
fn profile_driving(record: Option<CarRecord>, cars: Vec<LotCar>) -> CareerHandle {
    let mut career = Career::new("mwo", data(cars), &NewCareer::default()).unwrap();
    let Some(record) = record else { return load(career) };
    career.settings.car_handle = record.handle;
    career.stable.cars[0] = record;
    load(career)
}

fn load(career: Career) -> CareerHandle {
    let handle = CareerHandle::default();
    handle.load(career);
    handle
}

fn record(handle: u32, frontend_key: u32, vehicle_key: u32) -> CarRecord {
    CarRecord { handle, frontend_key, vehicle_key, flags: 0xF_0002, career_index: 0, ..CarRecord::free() }
}

#[test]
fn the_default_car_is_the_red_e46_gtr_and_the_career_start_car_is_its_preset() {
    assert_eq!(CarLook::default_car(), CarLook::named("BMWM3GTRE46", None));
    assert_eq!(CarLook::career_start(), CarLook::named("BMWM3GTRE46", Some("M3GTRCAREERSTART")));
}

#[test]
fn the_starter_handle_is_the_name_hash_of_the_starter_preset() {
    // `CarLook::resolve` finds a preset ride by hashing its name against the record's handle.
    assert_eq!(bstring_hash(STARTER_PRESET_NAME), STARTER_CAR_HANDLE);
}

#[test]
fn no_profile_shows_the_default_car() {
    assert_eq!(CarLook::of_profile(&CareerHandle::default()), CarLook::default_car());
}

#[test]
fn a_new_career_shows_the_career_start_preset() {
    let profile = profile_driving(None, Vec::new());
    assert_eq!(CarLook::of_profile(&profile), CarLook::career_start());
}

#[test]
fn a_current_car_missing_from_the_stable_shows_the_default_car() {
    let profile = profile_driving(None, Vec::new());
    profile.with_loaded(|career| career.settings.car_handle = 77);
    assert_eq!(CarLook::of_profile(&profile), CarLook::default_car());
}

#[test]
fn a_bought_car_shows_the_lot_name_of_its_front_end_collection_and_keeps_its_record() {
    let cars = vec![lot_car("gti", 0x7905_4B53), lot_car("tt", 0x193A_8BC8)];
    let profile = profile_driving(Some(record(3, 0x193A_8BC8, 0xD5C4_14E5)), cars);
    let look = CarLook::of_profile(&profile);
    assert_eq!(look.car, "tt");
    assert_eq!(look.preset, None);
    assert_eq!(look.record, Some(CarRecordRef { handle: 3, frontend_key: 0x193A_8BC8, vehicle_key: 0xD5C4_14E5 }));
}

#[test]
fn a_car_the_lot_does_not_list_keeps_only_its_record() {
    let profile = profile_driving(Some(record(4, 0x1111, 0x2222)), Vec::new());
    let look = CarLook::of_profile(&profile);
    assert_eq!(look.car, "");
    assert_eq!(look.record.map(|r| (r.handle, r.vehicle_key)), Some((4, 0x2222)));
}

#[test]
fn a_look_with_no_record_resolves_to_its_own_names() {
    let Some(dir) = install() else { return };
    assert_eq!(CarLook::career_start().resolve(&dir), ("BMWM3GTRE46".to_owned(), Some("M3GTRCAREERSTART".to_owned())));
    assert_eq!(CarLook::named("TT", None).resolve(&dir), ("TT".to_owned(), None));
}

#[test]
fn a_preset_ride_record_resolves_to_its_preset_and_car_type() {
    let Some(dir) = install() else { return };
    let rival = CarRecordRef { handle: bstring_hash("BL15"), frontend_key: 0, vehicle_key: 0 };
    let look = CarLook { car: String::new(), preset: None, record: Some(rival) };
    assert_eq!(look.resolve(&dir), ("GTI".to_owned(), Some("BL15".to_owned())));
}

#[test]
fn a_stock_record_resolves_through_the_hash_of_its_car_type() {
    let Some(dir) = install() else { return };
    let stock = |vehicle_key| CarRecordRef { handle: 9, frontend_key: 0, vehicle_key };
    let look = CarLook { car: String::new(), preset: None, record: Some(stock(vlt_hash("bmwm3gtr"))) };
    assert_eq!(look.resolve(&dir), ("BMWM3GTR".to_owned(), None));
    let unknown = CarLook { car: String::new(), preset: None, record: Some(stock(1)) };
    assert_eq!(unknown.resolve(&dir), (DEFAULT_CAR.to_owned(), None), "an unknown car gives the default");
}

#[test]
fn a_record_with_no_hash_match_falls_back_to_the_lot_name() {
    let Some(dir) = install() else { return };
    let record = CarRecordRef { handle: 9, frontend_key: 1, vehicle_key: 2 };
    let look = CarLook { car: "cobaltss".to_owned(), preset: None, record: Some(record) };
    assert_eq!(look.resolve(&dir), ("COBALTSS".to_owned(), None));
}

fn install() -> Option<GameDir> {
    let dir = std::env::var_os("NFSMW_GAME_DIR")?;
    GameDir::open(dir).ok()
}
