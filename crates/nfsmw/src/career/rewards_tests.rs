use std::sync::Arc;

use blackbox_attrib::vlt_hash;
use nfsmw_career::blacklist::{BinDef, KIND_CASH_BONUS, KIND_PINK_SLIP};
use nfsmw_career::carlot::{LotCar, LotClass};
use nfsmw_career::economy::markers::MarkerKind;
use nfsmw_career::settings::NewCareer;
use nfsmw_career::stable::CarRecord;

use super::*;
use crate::career::CareerData;

/// A bin with no races and no rival, only its number. The Blacklist needs bins 1 to 15 once each.
fn bin(number: u8) -> BinDef {
    BinDef {
        number,
        races: Vec::new(),
        races_needed: 0,
        challenges_needed: 0,
        bounty_needed: 0,
        rival_events: Vec::new(),
    }
}

/// A car of the car lot. Its handle is the name hash, as the install's preset cars have it.
fn lot_car(name: &str) -> LotCar {
    LotCar {
        name: name.to_string(),
        handle: vlt_hash(name),
        frontend_key: 0xC0,
        vehicle_key: vlt_hash(name),
        price: 26_000,
        unlock_rank: 16,
        for_sale: true,
        class: LotClass::Europe,
    }
}

/// A career table with the 15 bins and the given car lot.
fn career(cars: Vec<LotCar>) -> Career {
    let data = CareerData {
        bins: (1..=15).map(bin).collect(),
        milestones: Vec::new(),
        traps: Vec::new(),
        cars,
        stats: Vec::new(),
        heat: Default::default(),
        ranks: Default::default(),
        gameplay: Default::default(),
    };
    Career::new("mwo", Arc::new(data), &NewCareer::default()).unwrap()
}

fn cash(value: i32) -> Reward {
    Reward { kind: Some(KIND_CASH_BONUS), value, ..Reward::default() }
}

fn pink_slip(preset: &str) -> Reward {
    Reward { kind: Some(KIND_PINK_SLIP), preset: Some(preset.to_string()), ..Reward::default() }
}

fn marker(kind: u8) -> Reward {
    Reward { kind: Some(kind), ..Reward::default() }
}

fn part(number: u32) -> Reward {
    Reward { part: Some(number), ..Reward::default() }
}

fn owned_cars(career: &Career) -> usize {
    career.stable.cars.iter().filter(|car| !car.is_free()).count()
}

#[test]
fn cash_is_added_at_once_and_is_not_stored_as_a_marker() {
    let mut c = career(Vec::new());
    let before = c.settings.cash;
    let effects = grant_rewards(&mut c, &[cash(500)], false);
    assert_eq!(effects, [RewardEffect::CashAdded(500)]);
    assert_eq!(c.settings.cash, before + 500);
    assert_eq!(MarkerInventory::from_table(&c.settings.inventory[..]).unwrap().count_entries(0x13, 0), 0);
}

#[test]
fn an_out_of_jail_free_marker_is_stored_and_read_back_into_the_markers() {
    let mut c = career(Vec::new());
    let effects = grant_rewards(&mut c, &[marker(0x11)], false);
    assert_eq!(effects, [RewardEffect::MarkerAdded { kind: 0x11 }]);
    assert_eq!(c.markers.count(MarkerKind::JailFree), 1);
    assert_eq!(MarkerInventory::from_table(&c.settings.inventory[..]).unwrap().count(MarkerKind::JailFree), 1);
}

#[test]
fn an_impound_strike_and_an_impound_release_are_stored_as_their_kinds() {
    let mut c = career(Vec::new());
    grant_rewards(&mut c, &[marker(0x14), marker(0x15)], false);
    assert_eq!(c.markers.count(MarkerKind::ExtraStrike), 1);
    assert_eq!(c.markers.count(MarkerKind::ImpoundRelease), 1);
}

#[test]
fn a_part_unlock_is_stored_with_its_part_number_as_the_type() {
    let mut c = career(Vec::new());
    let effects = grant_rewards(&mut c, &[part(8)], false);
    assert_eq!(effects, [RewardEffect::PartUnlocked { part: 8 }]);
    assert_eq!(MarkerInventory::from_table(&c.settings.inventory[..]).unwrap().count_entries(8, 0), 1);
}

#[test]
fn a_pink_slip_adds_the_preset_car_to_the_stable_once() {
    let mut c = career(vec![lot_car("BL05")]);
    let effects = grant_rewards(&mut c, &[pink_slip("BL05")], false);
    assert_eq!(effects, [RewardEffect::AddCar { preset: "BL05".to_string() }]);
    let car = c.stable.car_by_handle(vlt_hash("BL05")).copied().expect("the preset car is in the stable");
    assert_eq!(car.flags, CarRecord::CLASS_MASK | CarRecord::FLAG_OWNED);
    assert_eq!(owned_cars(&c), 1);

    let again = grant_rewards(&mut c, &[pink_slip("BL05")], false);
    assert_eq!(again, [RewardEffect::CarAlreadyOwned { preset: "BL05".to_string() }]);
    assert_eq!(owned_cars(&c), 1, "a second pink slip for the same car adds nothing");
}

#[test]
fn a_pink_slip_for_a_car_the_car_lot_does_not_list_adds_no_car() {
    let mut c = career(Vec::new());
    let effects = grant_rewards(&mut c, &[pink_slip("BL05")], false);
    assert_eq!(effects, [RewardEffect::AddCar { preset: "BL05".to_string() }]);
    assert_eq!(owned_cars(&c), 0);
}

#[test]
fn deferred_rewards_are_queued_and_neither_paid_nor_stored() {
    let mut c = career(vec![lot_car("BL05")]);
    let before = c.settings.cash;
    let effects = grant_rewards(&mut c, &[cash(300), pink_slip("BL05"), marker(0x11)], true);
    assert_eq!(effects.len(), 3);
    assert!(effects.iter().all(|effect| matches!(effect, RewardEffect::Queued(_))));
    assert_eq!(c.popups.entries().len(), 3);
    assert_eq!(c.settings.cash, before);
    assert_eq!(owned_cars(&c), 0);
    assert_eq!(c.markers.count(MarkerKind::JailFree), 0);
}
