use blackbox_text::label_hash;
use nfsmw_career::carlot::{LotCar, LotClass};
use nfsmw_career::settings::CareerSettings;
use nfsmw_career::stable::{CarRecord, CareerRecord, Stable};

use super::*;

fn lot(name: &str, key: u32, price: u32) -> LotCar {
    LotCar {
        name: name.to_string(),
        handle: key + 1,
        frontend_key: key,
        vehicle_key: key + 2,
        price,
        unlock_rank: 16,
        for_sale: true,
        class: LotClass::Europe,
    }
}

fn settings(cash: u32) -> CareerSettings {
    let mut s = CareerSettings::new_career();
    s.cash = cash;
    s
}

#[test]
fn the_highlight_moves_one_row_and_stops_at_the_ends() {
    assert_eq!(step(0, 3, true), 1);
    assert_eq!(step(2, 3, true), 2, "the last row stays put");
    assert_eq!(step(1, 3, false), 0);
    assert_eq!(step(0, 3, false), 0, "the first row stays put");
}

#[test]
fn an_empty_list_keeps_the_highlight_at_the_top() {
    assert_eq!(step(0, 0, true), 0);
    assert_eq!(step(0, 0, false), 0);
}

#[test]
fn the_two_tabs_swap() {
    assert_eq!(Tab::Lot.other(), Tab::Owned);
    assert_eq!(Tab::Owned.other(), Tab::Lot);
}

#[test]
fn each_tab_has_its_own_title_label() {
    assert_ne!(Tab::Lot.title(), Tab::Owned.title());
    assert_eq!(Tab::Lot.title(), 0xB9D7_1959, "the Car Lot label");
}

#[test]
fn a_purchase_the_cash_cannot_cover_is_refused_with_the_no_cash_notice() {
    let car = lot("cobaltss", 0xC0, 26_000);
    let refusal = buy_refusal(&settings(25_999), &Stable::empty(), &car);
    assert_eq!(refusal, Some(Text::Game(label_hash("CAR_LOT_NO_CASH"))));
}

#[test]
fn a_purchase_with_the_cash_for_it_may_be_asked() {
    let car = lot("cobaltss", 0xC0, 26_000);
    assert_eq!(buy_refusal(&settings(26_000), &Stable::empty(), &car), None);
}

#[test]
fn a_purchase_with_no_free_career_record_is_refused_with_the_no_room_notice() {
    let car = lot("cobaltss", 0xC0, 26_000);
    let mut stable = Stable::empty();
    for record in stable.careers.iter_mut() {
        *record = CareerRecord::new(0);
    }
    let refusal = buy_refusal(&settings(100_000), &stable, &car);
    assert_eq!(refusal, Some(Text::Game(label_hash("CAR_LOT_NO_ROOM"))));
}

#[test]
fn the_first_customization_record_nothing_uses_is_zero_in_an_empty_stable() {
    assert_eq!(free_customization(&Stable::empty()), Some(0));
}

#[test]
fn the_customization_record_of_a_live_car_is_skipped() {
    let mut stable = Stable::empty();
    let mut car = CarRecord::free();
    car.handle = 0x10;
    car.customization_index = 0;
    stable.cars[0] = car;
    assert_eq!(free_customization(&stable), Some(1));
}

#[test]
fn the_lot_and_owned_keys_never_mix() {
    assert_ne!(Key::Lot(0x10), Key::Owned(0x10));
}

#[test]
fn only_the_car_lot_screen_name_is_made() {
    assert!(make("car_select.fng", Args::default()).is_some());
    assert!(make("main_menu_sub.fng", Args::default()).is_none());
}
