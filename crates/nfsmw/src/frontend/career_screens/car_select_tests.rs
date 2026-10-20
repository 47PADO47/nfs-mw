use std::sync::Arc;

use nfsmw_career::carlot::{LotCar, LotClass};
use nfsmw_career::economy::aftermath::{CarSelectInfo, CarStatus};
use nfsmw_career::settings::NewCareer;
use nfsmw_career::stable::{CarRecord, ImpoundState};

use super::super::car_select_view::{LABEL_RELEASE, LABEL_USE_MARKER, Pip, Value, header};
use super::*;
use crate::career::CareerData;

const KEY: u32 = 0xC0;
const HANDLE: u32 = 7;

fn info(record: &CareerRecord, cost: u32) -> CarSelectInfo {
    car_select_info(record, cost)
}

fn bin(number: u8) -> nfsmw_career::blacklist::BinDef {
    nfsmw_career::blacklist::BinDef {
        number,
        races: Vec::new(),
        races_needed: 0,
        challenges_needed: 0,
        bounty_needed: 0,
        rival_events: Vec::new(),
    }
}

/// A career with one owned car, the lot's 35 000 car.
fn career() -> Career {
    let lot = LotCar {
        name: "tt".to_owned(),
        handle: 1,
        frontend_key: KEY,
        vehicle_key: 2,
        price: 35_000,
        unlock_rank: 16,
        for_sale: true,
        class: LotClass::Europe,
    };
    let data = CareerData {
        bins: (1..=15).map(bin).collect(),
        milestones: Vec::new(),
        traps: Vec::new(),
        cars: vec![lot],
        stats: vec![None],
        heat: None,
        ranks: None,
        gameplay: Default::default(),
    };
    let mut career = Career::new("mwo", Arc::new(data), &NewCareer::default()).unwrap();
    career.stable.cars[0] = CarRecord {
        handle: HANDLE,
        frontend_key: KEY,
        flags: 0xF_0000 | CarRecord::FLAG_OWNED,
        career_index: 0,
        customization_index: 0,
        ..CarRecord::free()
    };
    career.stable.careers[0] = CareerRecord::new(0);
    career.settings.car_handle = HANDLE;
    career
}

#[test]
fn a_free_car_shows_its_strikes_and_the_new_one_animates() {
    let mut record = CareerRecord::new(0);
    record.strikes = 2;
    record.bounty = 5_000;
    record.unserved[0] = 2;
    let head = header(&info(&record, 35_000), CarSelectEntry::NewStrike, 900, false);
    assert_eq!(head.pips, [Pip::Show, Pip::Animate, Pip::Hide, Pip::Hide, Pip::Hide]);
    assert_eq!(head.position, 1);
    assert!(!head.show_pip_four && !head.show_pip_five && !head.stamp);
    assert_eq!((head.value_a, head.value_b), (Value::Money(5_000), Value::Money(300)));
    let plain = header(&info(&record, 35_000), CarSelectEntry::Plain, 900, false);
    assert_eq!(plain.pips, [Pip::Show, Pip::Show, Pip::Hide, Pip::Hide, Pip::Hide]);
}

#[test]
fn a_raised_limit_moves_the_pips_and_adds_groups() {
    let mut record = CareerRecord::new(0);
    record.strike_limit = 5;
    record.strikes = 4;
    let head = header(&info(&record, 0), CarSelectEntry::Plain, 0, false);
    assert_eq!(head.position, 3);
    assert!(head.show_pip_four && head.show_pip_five);
    assert_eq!(head.pips, [Pip::Show, Pip::Show, Pip::Show, Pip::Show, Pip::Hide]);
    record.strike_limit = 4;
    assert_eq!(header(&info(&record, 0), CarSelectEntry::Plain, 0, false).position, 2);
}

#[test]
fn an_impounded_car_shows_the_stamp_and_a_pending_release() {
    let mut record = CareerRecord::new(0);
    record.enter_impound(ImpoundState::LimitReached);
    let head = header(&info(&record, 35_000), CarSelectEntry::Impounded, 50_000, false);
    assert!(head.stamp && head.stamp_animates);
    assert_eq!(head.value_a, Value::Label(0xaefe_dad9), "Pending while it counts down");
    assert_eq!(head.value_b, Value::Money(50_000), "the cash line");
    assert_eq!(head.accept, 0x2b65_a216, "plain Accept: nothing to release with");
    let later = header(&info(&record, 35_000), CarSelectEntry::Plain, 50_000, false);
    assert!(later.stamp && !later.stamp_animates, "the animation plays once");
}

#[test]
fn the_accept_label_follows_what_can_release_the_car() {
    let mut record = CareerRecord::new(0);
    record.enter_impound(ImpoundState::LimitReached);
    record.countdown = 1;
    record.win_tick();
    assert_eq!(info(&record, 35_000).status, CarStatus::Releasable);
    let pay = header(&info(&record, 35_000), CarSelectEntry::Plain, 8_750, false);
    assert_eq!((pay.accept, pay.value_a), (LABEL_RELEASE, Value::Money(8_750)));
    let poor = header(&info(&record, 35_000), CarSelectEntry::Plain, 100, true);
    assert_eq!(poor.accept, LABEL_USE_MARKER);
    let rich_with_marker = header(&info(&record, 35_000), CarSelectEntry::Plain, 9_000, true);
    assert_eq!(rich_with_marker.accept, LABEL_RELEASE, "cash comes first");
}

#[test]
fn the_cost_of_a_car_comes_from_the_lot_catalogue() {
    let career = career();
    assert_eq!(cost_of(&career, HANDLE), 35_000);
    assert_eq!(cost_of(&career, 99), 0);
}

#[test]
fn the_rows_are_the_owned_cars() {
    let career = career();
    let rows = items(&career, Tab::Owned);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key, Key::Owned(HANDLE));
}

#[test]
fn a_cash_release_spends_the_fee_and_frees_the_car() {
    let mut career = career();
    career.settings.cash = 10_000;
    let record = career.stable.career_for_handle_mut(HANDLE).unwrap();
    record.enter_impound(ImpoundState::LimitReached);
    record.countdown = 1;
    record.win_tick();
    assert_eq!(release(&mut career, HANDLE, false), Ok(()));
    assert_eq!(career.settings.cash, 1_250);
    assert!(career.stable.career_for_handle(HANDLE).unwrap().is_free_to_drive());
}

#[test]
fn a_poor_player_cannot_release_by_cash_and_the_car_stays_impounded() {
    let mut career = career();
    career.settings.cash = 5;
    let record = career.stable.career_for_handle_mut(HANDLE).unwrap();
    record.enter_impound(ImpoundState::CashConfiscated);
    assert!(release(&mut career, HANDLE, false).is_err());
    assert_eq!(career.stable.career_for_handle(HANDLE).unwrap().state, 3);
    assert_eq!(career.settings.cash, 5);
}

#[test]
fn a_marker_release_consumes_the_marker_while_the_car_still_counts_down() {
    let mut career = career();
    career.markers.add(MarkerKind::ImpoundRelease, 1);
    career.stable.career_for_handle_mut(HANDLE).unwrap().enter_impound(ImpoundState::LimitReached);
    assert_eq!(release(&mut career, HANDLE, true), Ok(()));
    assert!(career.stable.career_for_handle(HANDLE).unwrap().is_free_to_drive());
    assert_eq!(career.markers.count(MarkerKind::ImpoundRelease), 0);
    assert!(release(&mut career, HANDLE, true).is_err(), "no marker left");
}

#[test]
fn the_strike_marker_raises_the_limit_and_is_consumed() {
    let mut career = career();
    career.markers.add(MarkerKind::ExtraStrike, 1);
    assert_eq!(use_strike_marker(&mut career, HANDLE), Ok(()));
    assert_eq!(career.stable.career_for_handle(HANDLE).unwrap().strike_limit, 4);
    assert_eq!(career.markers.count(MarkerKind::ExtraStrike), 0);
    assert!(use_strike_marker(&mut career, HANDLE).is_err());
}

#[test]
fn the_signature_changes_with_what_the_screen_draws() {
    let mut career = career();
    let rows = items(&career, Tab::Owned);
    let before = signature(&career, &rows, Some(HANDLE));
    career.settings.cash += 1;
    assert_ne!(signature(&career, &rows, Some(HANDLE)), before, "cash");
    let before = signature(&career, &rows, Some(HANDLE));
    career.stable.career_for_handle_mut(HANDLE).unwrap().enter_impound(ImpoundState::LimitReached);
    assert_ne!(signature(&career, &rows, Some(HANDLE)), before, "impound state");
    assert_eq!(signature(&career, &rows, Some(HANDLE)), signature(&career, &rows, Some(HANDLE)));
}

#[test]
fn the_message_labels_are_the_original_strings() {
    use blackbox_text::label_hash;
    assert_eq!(label_hash("MW_BUSTED_STRIKE_LIMIT"), 0x78f0_e298);
    assert_eq!(label_hash("MW_BUSTED_INSUFFICIENT_FUNDS"), 0x1ecf_fa6e);
    assert_eq!(label_hash("CS_IMPOUND_MUST_CHOOSE_NEW_CAR"), TEXT_MUST_CHOOSE);
}
