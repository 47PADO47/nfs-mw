//! The `nfsmw.career` port over a career of its own (a fresh handle, not the shared one).

use std::sync::Arc;

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_DENIED, BB_ERR_NOT_FOUND, BB_ERR_NOT_READY, BB_ERR_UNSUPPORTED};
use nfsmw_career::blacklist::BinDef;
use nfsmw_career::milestones::{MilestoneDef, StatType};
use nfsmw_career::settings::NewCareer;
use nfsmw_career::stable::{CarRecord, CareerRecord};
use nfsmw_modding::host::CareerBackend;

use super::career::CareerPort;
use crate::career::{Career, CareerData, CareerHandle};

const OWNER: &str = "mod.test";

/// The car the player drives in the tests.
const CAR: u32 = 7;

/// A bin with no races and no rival, only its number.
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

/// Bins 1 to 15, and one milestone in bin 1 that pays 500 bounty.
fn data() -> Arc<CareerData> {
    Arc::new(CareerData {
        bins: (1..=15).map(bin).collect(),
        milestones: vec![MilestoneDef {
            key: 0x1234,
            bin: 1,
            stat: StatType::PursuitLength,
            threshold: 10.0,
            bounty: 500,
        }],
        traps: Vec::new(),
        cars: Vec::new(),
        stats: Vec::new(),
        heat: Default::default(),
        ranks: Default::default(),
        gameplay: Default::default(),
    })
}

/// A career with one owned car, [`CAR`], which the player drives, and 1000 cash.
fn career() -> Career {
    let mut career = Career::new("mwo", data(), &NewCareer::default()).unwrap();
    career.stable.cars[0] = CarRecord { handle: CAR, flags: 0xF_0002, career_index: 0, ..CarRecord::free() };
    career.stable.careers[0] = CareerRecord::new(0);
    career.settings.car_handle = CAR;
    career.settings.cash = 1000;
    career
}

/// A port over a fresh handle holding `career`, and the handle.
fn port_with(career: Option<Career>) -> (CareerPort, CareerHandle) {
    let handle = CareerHandle::default();
    if let Some(career) = career {
        handle.set(career);
    }
    (CareerPort::new(handle.clone()), handle)
}

/// The bounty of the car at career record 0.
fn car_bounty(handle: &CareerHandle) -> Option<u32> {
    handle.with(|career| career.stable.careers[0].bounty)
}

#[test]
fn outside_career_mode_every_read_and_write_is_not_ready() {
    let (port, _) = port_with(None);
    assert_eq!(port.cash_get(OWNER), Err(BB_ERR_NOT_READY));
    assert_eq!(port.bounty_get(OWNER), Err(BB_ERR_NOT_READY));
    assert_eq!(port.cash_add(OWNER, 5, "x"), Err(BB_ERR_NOT_READY));
    assert_eq!(port.rival_count(OWNER), Err(BB_ERR_NOT_READY));
    assert_eq!(port.milestone_count(OWNER), Err(BB_ERR_NOT_READY));
    assert_eq!(port.garage_count(OWNER), Err(BB_ERR_NOT_READY));
    assert_eq!(port.stage_get(OWNER), Err(BB_ERR_NOT_READY));
}

#[test]
fn cash_adds_spends_clamped_at_zero_and_sets() {
    let (port, handle) = port_with(Some(career()));
    assert_eq!(port.cash_get(OWNER), Ok(1000));
    assert_eq!(port.cash_add(OWNER, 250, "race"), Ok(()));
    assert_eq!(port.cash_get(OWNER), Ok(1250));
    assert_eq!(port.cash_add(OWNER, -2000, "purchase"), Ok(()));
    assert_eq!(port.cash_get(OWNER), Ok(0), "spending clamps at 0");
    assert_eq!(port.cash_set(OWNER, -1), Err(BB_ERR_ARG));
    assert_eq!(port.cash_set(OWNER, 40), Ok(()));
    assert_eq!(handle.with(|c| c.cash()), Some(40));
}

#[test]
fn bounty_lives_on_the_current_car_and_feeds_the_blacklist() {
    let (port, handle) = port_with(Some(career()));
    assert_eq!(port.bounty_add(OWNER, 300), Ok(()));
    assert_eq!(port.bounty_get(OWNER), Ok(300));
    assert_eq!(car_bounty(&handle), Some(300));
    assert_eq!(handle.with(|c| c.blacklist.bounty()), Some(300));
    assert_eq!(port.bounty_set(OWNER, 100), Ok(()));
    assert_eq!(car_bounty(&handle), Some(100));
    assert_eq!(port.bounty_get(OWNER), Ok(100));
}

#[test]
fn bounty_set_below_the_other_cars_share_is_an_argument_error() {
    let mut career = career();
    career.stable.sold.absorb(&CareerRecord { bounty: 400, ..CareerRecord::new(1) });
    let (port, _) = port_with(Some(career));
    assert_eq!(port.bounty_set(OWNER, 300), Err(BB_ERR_ARG));
    assert_eq!(port.bounty_set(OWNER, 450), Ok(()));
    assert_eq!(port.bounty_get(OWNER), Ok(450));
}

#[test]
fn bounty_needs_a_driven_car() {
    let (port, handle) = port_with(Some(career()));
    handle.with(|c| c.settings.car_handle = 0);
    assert_eq!(port.bounty_add(OWNER, 1), Err(BB_ERR_NOT_FOUND));
}

#[test]
fn the_rap_sheet_reads_the_stable_and_writes_the_stored_sums() {
    let (port, handle) = port_with(Some(career()));
    assert_eq!(port.rap_sheet_set_stat(OWNER, 0, 5), Ok(()));
    assert_eq!(port.rap_sheet_add_stat(OWNER, 0, 2), Ok(()));
    assert_eq!(port.rap_sheet_set_stat(OWNER, 3, 4), Ok(()));
    assert_eq!(handle.with(|c| c.history.sums[0]), Some(7));
    let sheet = port.rap_sheet_get(OWNER).unwrap();
    assert_eq!(sheet.cops_wrecked, 4);
    assert_eq!((sheet.pursuits, sheet.busted, sheet.evaded), (0, 0, 0));
    assert_eq!(sheet.infractions, [0; 8]);
    assert_eq!(port.rap_sheet_set_stat(OWNER, 8, 1), Err(BB_ERR_ARG), "statistic 8 is computed");
}

#[test]
fn the_blacklist_lists_fifteen_rivals_in_rank_order() {
    let (port, _) = port_with(Some(career()));
    assert_eq!(port.rival_count(OWNER), Ok(15));
    let first = port.rival_at(OWNER, 0).unwrap();
    assert_eq!((first.rank, first.locked, first.defeated), (1, 1, 0));
    assert_eq!(port.rival_at(OWNER, 14).unwrap().rank, 15);
    assert_eq!(port.rival_at(OWNER, 15), Err(BB_ERR_NOT_FOUND));
    assert_eq!(port.rival_defeat(OWNER, 1), Err(BB_ERR_UNSUPPORTED));
    assert_eq!(port.rival_set_locked(OWNER, 1, false), Err(BB_ERR_UNSUPPORTED));
}

#[test]
fn a_milestone_completes_once_it_is_reached_and_pays_its_bounty() {
    let (port, handle) = port_with(Some(career()));
    assert_eq!(port.milestone_count(OWNER), Ok(1));
    let before = port.milestone_at(OWNER, 0).unwrap();
    assert_eq!((before.id, before.goal, before.done), (1, 10.0, 0));
    assert_eq!(port.milestone_complete(OWNER, 1), Err(BB_ERR_DENIED), "bin 1 not reached");
    handle.with(|c| c.milestones.enter_bin(1));
    assert_eq!(port.milestone_complete(OWNER, 1), Ok(()));
    assert_eq!(port.milestone_at(OWNER, 0).unwrap().done, 1);
    assert_eq!(car_bounty(&handle), Some(500));
    assert_eq!(port.milestone_complete(OWNER, 1), Ok(()), "a second call changes nothing");
    assert_eq!(car_bounty(&handle), Some(500));
    assert_eq!(port.milestone_complete(OWNER, 2), Err(BB_ERR_NOT_FOUND));
    assert_eq!(port.milestone_progress(OWNER, 1, 1.0), Err(BB_ERR_UNSUPPORTED));
}

#[test]
fn the_stage_is_the_blacklist_bin_and_starts_in_the_prologue() {
    let (port, _) = port_with(Some(career()));
    assert_eq!(port.stage_get(OWNER), Ok(16));
}

#[test]
fn the_garage_lists_the_owned_cars_and_the_driven_car() {
    let (port, handle) = port_with(Some(career()));
    assert_eq!(port.garage_count(OWNER), Ok(1));
    let car = port.garage_at(OWNER, 0).unwrap();
    assert_eq!((car.car, car.current), (u64::from(CAR), 1));
    assert_eq!(port.garage_at(OWNER, 1), Err(BB_ERR_NOT_FOUND));
    assert_eq!(port.current_car_get(OWNER), Ok(u64::from(CAR)));
    assert_eq!(port.current_car_set(OWNER, 9), Err(BB_ERR_NOT_FOUND));
    assert_eq!(port.current_car_set(OWNER, u64::from(CAR)), Ok(()));
    handle.with(|c| c.settings.car_handle = 0);
    assert_eq!(port.current_car_get(OWNER), Ok(0), "no driven car answers 0");
}

#[test]
fn the_calls_without_state_behind_them_stay_unsupported() {
    let (port, _) = port_with(Some(career()));
    assert_eq!(port.give_car(OWNER, 3), Err(BB_ERR_UNSUPPORTED));
    assert_eq!(port.remove_car(OWNER, CAR.into()), Err(BB_ERR_UNSUPPORTED));
    assert_eq!(port.save(OWNER), Err(BB_ERR_UNSUPPORTED));
    assert_eq!(port.slot_get(OWNER), Err(BB_ERR_UNSUPPORTED));
    assert_eq!(port.safehouse_count(OWNER), Err(BB_ERR_UNSUPPORTED));
    assert_eq!(port.shop_count(OWNER), Err(BB_ERR_UNSUPPORTED));
}
