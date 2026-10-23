use blackbox_text::label_hash;
use nfsmw_career::carlot::{LotCar, LotClass};
use nfsmw_career::settings::CareerSettings;
use nfsmw_career::stable::{CarRecord, CareerRecord, Stable};

use super::*;
use crate::frontend::career_screens::car_lot_bars::{StripFill, strip_fills};

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
    assert_eq!(Tab::Owned.title(), 0x5415_C3F1, "the Career label");
}

#[test]
fn a_purchase_the_cash_cannot_cover_is_refused_with_the_no_cash_notice() {
    let car = lot("cobaltss", 0xC0, 26_000);
    let refusal = buy_refusal(&settings(25_999), &Stable::empty(), &car, &Limits::SPEC);
    assert_eq!(refusal, Some(Text::Game(label_hash("CAR_LOT_NO_CASH"))));
}

#[test]
fn a_purchase_with_the_cash_for_it_may_be_asked() {
    let car = lot("cobaltss", 0xC0, 26_000);
    assert_eq!(buy_refusal(&settings(26_000), &Stable::empty(), &car, &Limits::SPEC), None);
}

#[test]
fn a_purchase_with_no_free_career_record_is_refused_with_the_no_room_notice() {
    let car = lot("cobaltss", 0xC0, 26_000);
    let mut stable = Stable::empty();
    for record in stable.careers.iter_mut() {
        *record = CareerRecord::new(0);
    }
    let refusal = buy_refusal(&settings(100_000), &stable, &car, &Limits::SPEC);
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

/// The right edge of a strip's lit part, in the bar's own x (the left edge is the bar's -60).
fn lit_right_edge(fill: &StripFill) -> f32 {
    fill.centre_x + fill.width / 2.0
}

#[test]
fn an_empty_bar_hides_every_strip() {
    let fills = strip_fills(0.0, &TOP_SPEED_STRIPS);
    assert!(fills.iter().all(|fill| !fill.shown && fill.lit_uv == 0.0));
}

#[test]
fn a_full_bar_lights_both_strips_to_the_end_of_the_full_strip() {
    let fills = strip_fills(1.0, &TOP_SPEED_STRIPS);
    assert_eq!(fills[0].lit_uv, 2.5);
    assert_eq!(fills[1].lit_uv, 5.0);
    assert!(fills.iter().all(|fill| fill.shown));
    assert_eq!(lit_right_edge(&fills[1]), 100.0, "the full strip ends at -60 + 160");
}

#[test]
fn a_half_bar_on_the_top_speed_strips_lights_the_first_80_pixels_of_both() {
    let fills = strip_fills(0.5, &TOP_SPEED_STRIPS);
    assert_eq!(fills[0].lit_uv, 2.5);
    assert_eq!(fills[0].width, 80.0);
    assert_eq!(lit_right_edge(&fills[0]), 20.0, "-60 + 80");
    assert_eq!(lit_right_edge(&fills[1]), 20.0, "the full strip stops at the same edge");
}

#[test]
fn a_fraction_is_split_over_the_strips_and_each_lit_part_keeps_its_scale() {
    let fills = strip_fills(0.9, &HANDLING_STRIPS);
    assert!((fills[0].lit_uv - 3.75).abs() < 1e-5, "the short strip is full");
    assert!((fills[1].lit_uv - 4.5).abs() < 1e-5, "the full strip lights the rest");
    assert!((fills[1].width - 144.0).abs() < 1e-3, "4.5 uv at 32 px per uv");
    assert!((lit_right_edge(&fills[1]) - 84.0).abs() < 1e-3, "0.9 of 160 px from -60");
}

#[test]
fn an_acceleration_quarter_bar_lights_a_part_of_the_short_strip_only() {
    let fills = strip_fills(0.25, &ACCELERATION_STRIPS);
    assert!((fills[0].lit_uv - 1.25).abs() < 1e-5);
    assert!((fills[0].width - 40.0).abs() < 1e-3);
    assert!((fills[0].centre_x + 40.0).abs() < 1e-3, "left edge stays at -60");
    assert!((lit_right_edge(&fills[1]) + 20.0).abs() < 1e-3, "-60 + 40: the full strip stops at the same edge");
}

#[test]
fn a_fraction_outside_zero_to_one_is_clamped() {
    assert_eq!(strip_fills(1.7, &TOP_SPEED_STRIPS), strip_fills(1.0, &TOP_SPEED_STRIPS));
    assert_eq!(strip_fills(-0.3, &TOP_SPEED_STRIPS), strip_fills(0.0, &TOP_SPEED_STRIPS));
}

#[test]
fn a_bar_of_no_strips_has_no_fills() {
    assert!(strip_fills(0.5, &[]).is_empty());
}

#[test]
fn only_the_first_car_purchase_starts_the_first_drive() {
    let first = CarLotAction::Buy { frontend_key: 0xC0, delivery: Delivery::Deliver, first_drive: true };
    let plain = CarLotAction::Buy { frontend_key: 0xC0, delivery: Delivery::Deliver, first_drive: false };
    assert!(first.starts_first_drive());
    assert!(!plain.starts_first_drive());
    assert!(!CarLotAction::Sell { handle: 1 }.starts_first_drive());
}

#[test]
fn the_first_car_is_affordable_with_the_prologue_prize_money() {
    // Three prologue wins pay 10,000 each (`story-flow-2.md` F-13): 30,000 in all.
    let car = lot("prize", 0xC1, 30_000);
    let refusal = buy_refusal(&settings(29_999), &Stable::empty(), &car, &Limits::SPEC);
    assert_eq!(refusal, Some(Text::Game(label_hash("CAR_LOT_NO_CASH"))));
    assert_eq!(buy_refusal(&settings(30_000), &Stable::empty(), &car, &Limits::SPEC), None);
}

/// A stable holding `count` owned career cars, each with its own career record.
fn owned_garage(count: usize) -> Stable {
    let mut stable = Stable::empty();
    for slot in 0..count {
        stable.cars[slot] = CarRecord {
            handle: 0x100 + slot as u32,
            frontend_key: 0xC0,
            vehicle_key: 0xC1,
            flags: CarRecord::CLASS_MASK | CarRecord::FLAG_OWNED,
            career_index: slot as u8,
            ..CarRecord::free()
        };
        stable.careers[slot] = CareerRecord::new(slot as u8);
    }
    stable
}

#[test]
fn ten_owned_cars_fill_a_single_players_garage() {
    let car = lot("cobaltss", 0xC0, 26_000);
    let stable = owned_garage(10);
    assert_eq!(carlot::owned_count(&stable), 10);
    let refusal = buy_refusal(&settings(100_000), &stable, &car, &Limits::for_players(1));
    assert_eq!(refusal, Some(Text::Game(label_hash("CAR_LOT_NO_ROOM"))));
}

#[test]
fn a_two_player_garage_takes_an_eleventh_car() {
    let car = lot("cobaltss", 0xC0, 26_000);
    let stable = owned_garage(10);
    assert_eq!(buy_refusal(&settings(100_000), &stable, &car, &Limits::for_players(2)), None);
}

#[test]
fn the_lots_rows_keep_the_greyed_cars_and_price_the_owned_ones_at_a_sale() {
    // The shared car pool is not read in a test, so no row is dropped as undrivable.
    let cars = [lot("cobaltss", 0xC0, 26_000), LotCar { unlock_rank: 7, ..lot("elise", 0xC1, 48_000) }];
    let rows = super::super::car_lot_look::lot_rows(&cars, 16);
    assert_eq!(
        rows.iter().map(|row| (row.name.as_str(), row.locked)).collect::<Vec<_>>(),
        [("cobaltss", false), ("elise", true)]
    );
    let owned = super::super::car_lot_look::owned_rows(&owned_garage(1), &[lot("cobaltss", 0xC0, 26_000)]);
    assert_eq!(owned.len(), 1);
    assert_eq!(owned[0].price, nfsmw_career::economy::sale_proceeds(26_000));
}

#[test]
fn the_showcase_remembers_where_the_lot_was_left_and_gives_it_back_once() {
    use super::super::car_lot_look::{ShowcaseCar, set_showcase, showcased, take_return};
    use crate::frontend::look::CarLook;
    let car = ShowcaseCar { name: "gti".into(), look: CarLook::named("GTI", None), tab: Tab::Owned, cursor: 3 };
    set_showcase(car.clone());
    assert_eq!(showcased(), Some(car));
    assert_eq!(take_return(), Some((Tab::Owned, 3)));
    assert_eq!(take_return(), None, "taken once");
}
