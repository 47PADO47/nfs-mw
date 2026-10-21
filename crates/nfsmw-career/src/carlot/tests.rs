use super::*;
use crate::settings::{CareerSettings, Flags};
use crate::stable::{CAREER_SLOTS, CarRecord, CareerRecord, ImpoundState, NO_INDEX, Stable};

fn lot(name: &str, handle: u32, key: u32, price: u32, rank: u8, class: LotClass) -> LotCar {
    LotCar {
        name: name.to_string(),
        handle,
        frontend_key: key,
        vehicle_key: key + 1,
        price,
        unlock_rank: rank,
        for_sale: true,
        class,
    }
}

/// Cobalt SS (rank 16, NA), A3 (rank 14, EU), Elise (rank 7, EU), a traffic car (class 3) and a car not for sale.
fn catalogue() -> Vec<LotCar> {
    vec![
        lot("cobaltss", 0x10, 0xC0, 26_000, 16, LotClass::NorthAmerica),
        lot("a3_20t", 0x11, 0xC1, 32_000, 14, LotClass::Europe),
        lot("elise", 0x12, 0xC2, 48_000, 7, LotClass::Europe),
        lot("traffic", 0x13, 0xC3, 5_000, 16, LotClass::Vehicles),
        LotCar { for_sale: false, ..lot("hidden", 0x14, 0xC4, 1, 16, LotClass::Japan) },
    ]
}

fn settings(stage: u8, cash: u32) -> CareerSettings {
    let mut s = CareerSettings::new_career();
    s.stage = stage;
    s.cash = cash;
    s
}

/// An owned career car in `slot` with career record `career` (the record is created as new).
fn put_car(stable: &mut Stable, slot: usize, handle: u32, career: u8, key: u32) {
    stable.cars[slot] = CarRecord {
        handle,
        frontend_key: key,
        vehicle_key: key + 1,
        flags: CarRecord::CLASS_MASK | CarRecord::FLAG_OWNED,
        career_index: career,
        ..CarRecord::free()
    };
    stable.careers[usize::from(career)] = CareerRecord::new(career);
}

#[test]
fn class_bits_and_the_lot_flags_follow_the_spec() {
    assert_eq!(LotClass::from_value(2), LotClass::Japan);
    assert_eq!(LotClass::Europe.flag_bit(), Some(0x2_0000));
    assert_eq!(LotClass::Vehicles.flag_bit(), Some(0x8_0000));
    assert_eq!(LotClass::Other(9).flag_bit(), None);
    let cat = catalogue();
    assert_eq!(cat[0].lot_flags(), 0x1_0000 | catalogue::FLAG_IN_LOT);
    assert_eq!(cat[3].lot_flags(), 0x8_0000 | catalogue::FLAG_NOT_FOR_SALE);
    assert_eq!(cat[4].lot_flags(), 0x4_0000 | catalogue::FLAG_NOT_FOR_SALE);
}

#[test]
fn a_car_is_available_while_the_stage_is_at_most_its_rank() {
    let cat = catalogue();
    let a3 = &cat[1];
    assert!(a3.is_available(14));
    assert!(!a3.is_available(15));
    assert_eq!(a3.state_at(15), LotState::Locked);
    assert_eq!(a3.state_at(14), LotState::Buyable);
    assert_eq!(cat[3].state_at(16), LotState::NotOffered, "vehicles class");
    assert_eq!(cat[4].state_at(16), LotState::NotOffered, "not for sale");
}

#[test]
fn the_lot_lists_locked_cars_and_drops_the_ones_not_offered() {
    let cat = catalogue();
    let names: Vec<(&str, LotState)> = listed(&cat, 15).into_iter().map(|(c, s)| (c.name.as_str(), s)).collect();
    assert_eq!(
        names,
        vec![("cobaltss", LotState::Buyable), ("a3_20t", LotState::Locked), ("elise", LotState::Locked),]
    );
    assert_eq!(by_frontend_key(&cat, 0xC1).map(|c| c.price), Some(32_000));
    assert!(by_frontend_key(&cat, 0x99).is_none());
}

#[test]
fn a_rival_car_appears_once_its_rival_is_beaten() {
    assert!(rival_car_available(15, 14));
    assert!(!rival_car_available(15, 15));
    assert!(rival_car_available(2, 1));
    assert!(!rival_car_available(2, 2));
    assert!(!rival_car_available(1, 0), "rank 1 is not in the list");
    assert!(!rival_car_available(16, 1), "no such rival");
}

#[test]
fn bonus_presets_follow_their_settings_bit() {
    assert_eq!(bonus_flag(0xCB6A_AF2F), Some(Flags::EVENT_19_8_69_DONE));
    assert_eq!(bonus_unlocked(0xCB6A_AF2F, Flags::EMPTY), Some(false));
    assert_eq!(bonus_unlocked(0xCB6A_AF2F, Flags::EVENT_19_8_69_DONE), Some(true));
    assert_eq!(bonus_unlocked(0x2CF3_85B2, Flags::CHALLENGE_SERIES_DONE), Some(true));
    assert_eq!(bonus_unlocked(0x1234_5678, Flags::EMPTY), None, "not a bonus preset");
}

#[test]
fn the_prologue_prompt_is_a_plain_confirm() {
    assert_eq!(prompt_for(16), PurchasePrompt::Confirm);
    assert_eq!(prompt_for(15), PurchasePrompt::DriveOrDeliver);
}

#[test]
fn owned_count_skips_marked_and_unclassed_records() {
    let mut stable = Stable::empty();
    put_car(&mut stable, 0, 0x10, 0, 0xC0);
    put_car(&mut stable, 1, 0x11, 1, 0xC1);
    stable.cars[1].flags |= FLAG_MARKED;
    stable.cars[2] = CarRecord { handle: 0x12, flags: CarRecord::FLAG_OWNED, ..CarRecord::free() };
    assert_eq!(owned_count(&stable), 1);
    stable.cars[3] = CarRecord { handle: 0x13, flags: CarRecord::CLASS_MASK | FLAG_CUSTOM, ..CarRecord::free() };
    assert_eq!(custom_count(&stable), 1);
    assert_eq!(Limits::SPEC, Limits { owned_cars: 10, career_records: 25, customs: 20 });
}

#[test]
fn a_drive_now_purchase_charges_cash_and_becomes_the_current_car() {
    let cat = catalogue();
    let mut s = settings(16, 30_000);
    let mut stable = Stable::empty();
    let bought = buy(&mut s, &mut stable, &cat[0], Delivery::DriveNow, 3, &Limits::SPEC);
    assert_eq!(bought, Ok(Purchase { handle: 0x10, car_slot: 0, career_index: 0, price: 26_000 }));
    assert_eq!(s.cash, 4_000);
    assert_eq!(s.car_handle, 0x10);
    let car = stable.cars[0];
    assert_eq!((car.handle, car.frontend_key, car.vehicle_key), (0x10, 0xC0, 0xC1));
    assert_eq!(
        (car.flags, car.customization_index, car.career_index),
        (CarRecord::CLASS_MASK | CarRecord::FLAG_OWNED, 3, 0)
    );
    assert!(!stable.careers[0].is_unused());
    assert_eq!(stable.careers[0].strike_limit, 3);
    assert_eq!(owned_count(&stable), 1);
}

#[test]
fn a_delivered_purchase_keeps_the_current_car() {
    let cat = catalogue();
    let mut s = settings(15, 40_000);
    s.car_handle = 0x99;
    let mut stable = Stable::empty();
    let bought = buy(&mut s, &mut stable, &cat[0], Delivery::Deliver, 0, &Limits::SPEC);
    assert!(bought.is_ok());
    assert_eq!(s.car_handle, 0x99);
    assert_eq!(s.cash, 14_000);
    assert_eq!(stable.cars[0].handle, 0x10);
}

#[test]
fn a_purchase_needs_the_price_in_cash_and_changes_nothing_when_refused() {
    let cat = catalogue();
    let mut s = settings(16, 25_999);
    let handle_before = s.car_handle;
    let mut stable = Stable::empty();
    let before = stable.clone();
    let refused = buy(&mut s, &mut stable, &cat[0], Delivery::DriveNow, 0, &Limits::SPEC);
    assert_eq!(refused, Err(PurchaseError::NotEnoughCash));
    assert_eq!((s.cash, s.car_handle), (25_999, handle_before));
    assert_eq!(stable, before);

    s.cash = 26_000;
    assert!(buy(&mut s, &mut stable, &cat[0], Delivery::DriveNow, 0, &Limits::SPEC).is_ok());
    assert_eq!(s.cash, 0, "an exact price is enough");
}

#[test]
fn the_garage_refuses_the_eleventh_owned_car() {
    let cat = catalogue();
    let mut stable = Stable::empty();
    for slot in 0..9 {
        put_car(&mut stable, slot, 0x100 + slot as u32, slot as u8, 0xC0);
    }
    let mut s = settings(16, 100_000);
    assert!(
        buy(&mut s, &mut stable, &cat[0], Delivery::DriveNow, 0, &Limits::SPEC).is_ok(),
        "the tenth car is allowed"
    );
    assert_eq!(owned_count(&stable), 10);
    let refused = buy(&mut s, &mut stable, &cat[0], Delivery::DriveNow, 0, &Limits::SPEC);
    assert_eq!(refused, Err(PurchaseError::GarageFull));
}

#[test]
fn a_purchase_is_refused_when_all_career_records_are_in_use() {
    let cat = catalogue();
    let mut stable = Stable::empty();
    // Used career records that belong to no owned car, so the garage limit does not trigger first.
    for (index, record) in stable.careers.iter_mut().enumerate().take(CAREER_SLOTS) {
        *record = CareerRecord::new(index as u8);
    }
    let mut s = settings(16, 100_000);
    let refused = buy(&mut s, &mut stable, &cat[0], Delivery::DriveNow, 0, &Limits::SPEC);
    assert_eq!(refused, Err(PurchaseError::NoCareerSlot));
    assert_eq!(s.cash, 100_000);
}

#[test]
fn a_locked_or_unlisted_car_cannot_be_bought() {
    let cat = catalogue();
    let mut s = settings(15, 1_000_000);
    let mut stable = Stable::empty();
    assert_eq!(buy(&mut s, &mut stable, &cat[1], Delivery::DriveNow, 0, &Limits::SPEC), Err(PurchaseError::Locked));
    assert_eq!(buy(&mut s, &mut stable, &cat[3], Delivery::DriveNow, 0, &Limits::SPEC), Err(PurchaseError::NotOffered));
    assert_eq!(buy(&mut s, &mut stable, &cat[4], Delivery::DriveNow, 0, &Limits::SPEC), Err(PurchaseError::NotOffered));
    assert_eq!(s.cash, 1_000_000);
}

#[test]
fn the_sale_pays_half_the_price_and_clears_the_car() {
    let cat = catalogue();
    let mut stable = Stable::empty();
    put_car(&mut stable, 0, 0x10, 0, 0xC0);
    put_car(&mut stable, 1, 0x11, 1, 0xC1);
    stable.careers[0].bounty = 500;
    stable.careers[0].evaded = 2;
    let mut s = settings(15, 100);
    s.car_handle = 0x11;
    let sold = sell(&mut s, &mut stable, &cat, 0x10, true);
    assert_eq!(sold, Ok(Sale { handle: 0x10, proceeds: 13_000, new_current: None }));
    assert_eq!(s.cash, 13_100);
    assert_eq!(s.car_handle, 0x11, "the current car did not change");
    assert!(stable.car_by_handle(0x10).is_none());
    assert!(stable.careers[0].is_unused());
    assert_eq!((stable.sold.bounty, stable.sold.evaded), (500, 2));
    assert_eq!(owned_count(&stable), 1);
}

#[test]
fn selling_the_current_car_moves_the_current_car_to_the_next_free_one() {
    let cat = catalogue();
    let mut stable = Stable::empty();
    put_car(&mut stable, 0, 0x10, 0, 0xC0);
    put_car(&mut stable, 1, 0x11, 1, 0xC1);
    let mut s = settings(15, 0);
    s.car_handle = 0x10;
    assert_eq!(replacement_current(&stable, 0x10), Some(0x11));
    let sold = sell(&mut s, &mut stable, &cat, 0x10, true).expect("sale");
    assert_eq!(sold.new_current, Some(0x11));
    assert_eq!(s.car_handle, 0x11);
}

#[test]
fn a_sale_is_refused_outside_the_safe_house_or_after_stage_15() {
    let cat = catalogue();
    let mut stable = Stable::empty();
    put_car(&mut stable, 0, 0x10, 0, 0xC0);
    put_car(&mut stable, 1, 0x11, 1, 0xC1);
    let mut s = settings(15, 0);
    assert_eq!(sell(&mut s, &mut stable, &cat, 0x10, false), Err(SaleError::NotAllowed));
    s.stage = 16;
    assert_eq!(sell(&mut s, &mut stable, &cat, 0x10, true), Err(SaleError::NotAllowed));
    assert!(stable.car_by_handle(0x10).is_some(), "nothing was sold");
}

#[test]
fn a_sale_is_refused_for_an_impounded_car_and_for_the_last_free_car() {
    let cat = catalogue();
    let mut stable = Stable::empty();
    put_car(&mut stable, 0, 0x10, 0, 0xC0);
    put_car(&mut stable, 1, 0x11, 1, 0xC1);
    stable.careers[0].set_impound_state(ImpoundState::LimitReached);
    let mut s = settings(15, 0);
    assert_eq!(sell(&mut s, &mut stable, &cat, 0x10, true), Err(SaleError::Impounded));
    stable.careers[0].set_impound_state(ImpoundState::Free);
    stable.careers[1].set_impound_state(ImpoundState::LimitReached);
    assert_eq!(sell(&mut s, &mut stable, &cat, 0x10, true), Err(SaleError::LastCar));
}

#[test]
fn a_sale_is_refused_without_a_career_record_or_a_price() {
    let cat = catalogue();
    let mut stable = Stable::empty();
    put_car(&mut stable, 0, 0x10, 0, 0xC0);
    put_car(&mut stable, 1, 0x11, 1, 0xC1);
    let mut s = settings(15, 0);
    assert_eq!(sell(&mut s, &mut stable, &cat, 0x55, true), Err(SaleError::NoCareerRecord));
    stable.cars[1].career_index = NO_INDEX;
    assert_eq!(sell(&mut s, &mut stable, &cat, 0x11, true), Err(SaleError::NoCareerRecord));
    stable.cars[1].career_index = 1;
    assert_eq!(sell(&mut s, &mut stable, &[], 0x10, true), Err(SaleError::NoPrice));
}

#[test]
fn a_preset_car_is_added_without_taking_cash_or_checking_the_price() {
    let cat = catalogue();
    let mut stable = Stable::empty();
    let added = add_preset_car(&mut stable, &cat[0], 4);
    assert_eq!(added, Ok(Purchase { handle: 0x10, car_slot: 0, career_index: 0, price: 0 }));
    let car = stable.cars[0];
    assert_eq!((car.handle, car.frontend_key, car.vehicle_key), (0x10, 0xC0, 0xC1));
    assert_eq!(
        (car.flags, car.customization_index, car.career_index),
        (CarRecord::CLASS_MASK | CarRecord::FLAG_OWNED, 4, 0)
    );
    assert!(!stable.careers[0].is_unused());
    assert_eq!(owned_count(&stable), 1);
}

#[test]
fn a_preset_car_the_stable_already_holds_is_not_added_twice() {
    let cat = catalogue();
    let mut stable = Stable::empty();
    assert!(add_preset_car(&mut stable, &cat[0], 4).is_ok());
    let before = stable.clone();
    assert_eq!(add_preset_car(&mut stable, &cat[0], 5), Err(PurchaseError::AlreadyOwned));
    assert_eq!(stable, before, "a refused preset car writes nothing");
}

#[test]
fn a_preset_car_needs_a_free_career_record_and_car_slot() {
    let cat = catalogue();
    let mut stable = Stable::empty();
    for index in 0..CAREER_SLOTS {
        stable.careers[index] = CareerRecord::new(index as u8);
    }
    let before = stable.clone();
    assert_eq!(add_preset_car(&mut stable, &cat[0], 4), Err(PurchaseError::NoCareerSlot));
    assert_eq!(stable, before, "a refused preset car writes nothing");
}

#[test]
fn the_free_customization_record_is_the_first_one_no_car_uses() {
    let mut stable = Stable::empty();
    assert_eq!(stable.free_customization(), Some(0));
    put_car(&mut stable, 0, 0x10, 0, 0xC0);
    stable.cars[0].customization_index = 0;
    assert_eq!(stable.free_customization(), Some(1));
}
