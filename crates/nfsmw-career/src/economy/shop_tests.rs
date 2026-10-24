use super::*;

fn settings(cash: u32) -> CareerSettings {
    let mut settings = CareerSettings::new_career();
    settings.cash = cash;
    settings.shop_unlocks[8 * 8] = 2;
    settings
}

fn engine(level: u32, cost: u32) -> Item {
    Item { entity: 8, level, cost, trade_in: 0, marker: None }
}

#[test]
fn preview_never_changes_cash_inventory_or_unlocks() {
    let settings = settings(1000);
    let markers = MarkerInventory::empty();
    let before = settings.clone();
    assert_eq!(quote(&settings, &markers, &[engine(2, 500)]).unwrap().total, 500);
    assert_eq!(settings, before);
    assert_eq!(markers, MarkerInventory::empty());
}

#[test]
fn locked_and_unaffordable_carts_fail_without_charging() {
    assert!(matches!(
        quote(&settings(1000), &MarkerInventory::empty(), &[engine(3, 1)]),
        Err(ShopError::Locked { .. })
    ));
    assert!(matches!(
        quote(&settings(499), &MarkerInventory::empty(), &[engine(2, 500)]),
        Err(ShopError::NotEnoughCash { .. })
    ));
}

#[test]
fn checkout_rechecks_markers_and_cash_and_commits_atomically() {
    let mut settings = settings(1000);
    let mut markers = MarkerInventory::empty();
    markers.add_entries(8, 0, 1);
    let item = Item { marker: Some((8, 0)), ..engine(3, 9999) };
    let cart = quote(&settings, &markers, &[item, engine(1, 250)]).unwrap();
    assert_eq!(cart.total, 250);
    let before = settings.clone();
    markers.take_entry(8, 0);
    assert!(checkout(&mut settings, &mut markers, &cart).is_err());
    assert_eq!(settings, before);
    markers.add_entries(8, 0, 1);
    checkout(&mut settings, &mut markers, &cart).unwrap();
    assert_eq!(settings.cash, 750);
    assert_eq!(markers.count_entries(8, 0), 0);
}

#[test]
fn one_marker_cannot_pay_for_two_items_and_totals_never_wrap() {
    let mut markers = MarkerInventory::empty();
    markers.add_entries(8, 0, 1);
    let item = Item { marker: Some((8, 0)), ..engine(3, 0) };
    assert!(matches!(quote(&settings(1000), &markers, &[item, item]), Err(ShopError::MissingMarker { .. })));
    assert!(matches!(
        quote(&settings(u32::MAX), &markers, &[engine(1, u32::MAX), engine(2, 1)]),
        Err(ShopError::Overflow)
    ));
    assert_eq!(markers.count_entries(8, 0), 1);
}

#[test]
fn stock_restoration_and_completed_career_are_available() {
    assert_eq!(quote(&settings(0), &MarkerInventory::empty(), &[engine(0, 0)]).unwrap().total, 0);
    let mut settings = settings(1000);
    settings.flags.insert(Flags::CAREER_COMPLETE);
    assert!(quote(&settings, &MarkerInventory::empty(), &[engine(4, 100)]).is_ok());
    assert_eq!(unlocked_level(&settings, 999, false), None);
}

#[test]
fn cheaper_model_parts_refund_the_net_credit_once() {
    let mut settings = settings(100);
    let mut markers = MarkerInventory::empty();
    let replacement = Item { cost: 200, trade_in: 1000, ..engine(1, 0) };
    let cart = quote(&settings, &markers, &[replacement, engine(2, 300)]).unwrap();
    assert_eq!((cart.total, cart.refund), (0, 500));
    checkout(&mut settings, &mut markers, &cart).unwrap();
    assert_eq!(settings.cash, 600);
}
