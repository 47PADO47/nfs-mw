//! The shop rows of a vehicle with components: its own lists, its rules, its prices and its saves (`menu_slots`, `pricing`,
//! `presets`), on a bike a test publishes (`menu_fixture`).

use nfsmw_data::car::customization::Category;
use nfsmw_data::car::physics::PerformanceCategory;

use super::menu_fixture::{self, shop_model};
use super::menu_rows::Row;
use super::pricing::components as priced;
use super::{Model, Selection, menu_slots, rider_gear};

fn labels(model: &Model, row: Row) -> (Vec<String>, usize) {
    row.options(model)
}

#[test]
fn a_bikes_rows_list_its_components_by_name_not_the_categorys_words() {
    let model = shop_model();
    let (labels, at) = labels(&model, Row::Part(Category::Spoiler));
    assert_eq!(
        (labels.as_slice(), at),
        (["Stock exhaust", "Race exhaust", "Loud exhaust"].map(String::from).as_slice(), 0)
    );
    assert_eq!(labels_of(&model, Row::Part(Category::Wheels)), ["Stock rims", "Forged rims"]);
    assert_eq!(
        labels_of(&model, Row::Performance(PerformanceCategory::Engine)),
        ["Original equipment", "Engine kit", "Engine kit 2"]
    );
    // A slot only the components know has a row of its own, by its position among the vehicle's slots.
    let mirrors = Row::Slot(3);
    assert_eq!(labels_of(&model, mirrors), ["Mirrors", "Chrome mirrors", "None"]);
    assert_eq!(menu_slots::label(&model, Row::Part(Category::Spoiler)).as_deref(), Some("Exhaust"));
    assert_eq!(menu_slots::label(&model, mirrors).as_deref(), Some("Road mirrors"));
    assert!(mirrors.enabled(&model));
}

fn blocked(model: &Model, row: Row, index: usize) -> Option<String> {
    menu_slots::blocked_all(model, row).into_iter().nth(index).flatten()
}

fn labels_of(model: &Model, row: Row) -> Vec<String> {
    row.options(model).0
}

#[test]
fn a_rule_of_the_vehicle_refuses_a_choice_and_says_why() {
    let mut model = shop_model();
    let exhaust = Row::Part(Category::Spoiler);
    // The loud exhaust needs the engine's first stage: it is blocked, in the data's words, and picking it changes nothing.
    let reason = blocked(&model, exhaust, 2).unwrap();
    assert!(reason.starts_with("Loud exhaust cannot be fitted") && reason.contains("perf_engine"), "{reason}");
    assert_eq!(blocked(&model, exhaust, 1), None);
    exhaust.set(&mut model, 2);
    assert_eq!(model.draft.parts.parts[Category::Spoiler as usize], None);
    // With the stage fitted the same choice is allowed and is kept as the hash of the car and the component.
    Row::Performance(PerformanceCategory::Engine).set(&mut model, 1);
    assert_eq!(blocked(&model, exhaust, 2), None);
    exhaust.set(&mut model, 2);
    assert_eq!(
        model.draft.parts.parts[Category::Spoiler as usize],
        Some(nfsmw_data::car::mods::components::select::choice_hash(menu_fixture::FOLDER, "t:exhaust.loud"))
    );
    // Taking the stage away again is refused while the exhaust needs it.
    Row::Performance(PerformanceCategory::Engine).set(&mut model, 0);
    assert_eq!(model.draft.performance.levels[PerformanceCategory::Engine as usize], 1);
}

#[test]
fn a_component_slot_no_category_holds_is_kept_with_the_selection() {
    let mut model = shop_model();
    let mirrors = Row::Slot(3);
    mirrors.set(&mut model, 1);
    assert_eq!(model.draft.parts.slots.get("road_mirrors").map(String::as_str), Some("t:mirrors.chrome"));
    assert_eq!(mirrors.options(&model).1, 1);
    mirrors.set(&mut model, 2);
    assert_eq!(model.draft.parts.slots.get("road_mirrors").map(String::as_str), Some(""), "emptied on purpose");
    mirrors.set(&mut model, 0);
    assert!(model.draft.parts.slots.is_empty(), "the stock choice is the absence of a pick");
}

#[test]
fn prices_come_from_the_components_and_trade_in_a_quarter_of_the_replaced_one() {
    let mut model = shop_model();
    let (stock, car) = (model.draft.clone(), model.selected().unwrap().clone());
    Row::Part(Category::Spoiler).set(&mut model, 1);
    let item = nfsmw_career::economy::shop::Item { entity: 12, level: 1, cost: 77, trade_in: 0, marker: None };
    let priced_item = priced::adjust(Row::Part(Category::Spoiler), item, &model.draft, &stock, &car, false);
    assert_eq!(
        (priced_item.cost, priced_item.level, priced_item.entity),
        (900, 1, 12),
        "the data's price, the shop's tier"
    );
    // A component with no price keeps the shop's own number; the backroom pays with markers and keeps it too.
    Row::Part(Category::Wheels).set(&mut model, 1);
    let forged = priced::adjust(Row::Part(Category::Wheels), item, &model.draft, &stock, &car, false);
    assert_eq!(forged.cost, 1200);
    assert_eq!(priced::adjust(Row::Part(Category::Spoiler), item, &model.draft, &stock, &car, true).cost, 77);
    // Replacing the race exhaust by stock gives back a quarter of what it cost.
    let (before, mut after) = (model.draft.clone(), model.clone());
    Row::Part(Category::Spoiler).set(&mut after, 0);
    let back = priced::adjust(Row::Part(Category::Spoiler), item, &after.draft, &before, &car, false);
    assert_eq!(back.trade_in, 225);
}

#[test]
fn a_session_slot_is_priced_by_its_component_or_as_a_first_tier_body_part() {
    let mut model = shop_model();
    let (stock, car) = (model.draft.clone(), model.selected().unwrap().clone());
    let mirrors = Row::Slot(3);
    let free = priced::session(mirrors, &model.draft, &stock, &car, || panic!("nothing is fitted")).unwrap();
    assert_eq!((free.cost, free.level), (0, 0));
    mirrors.set(&mut model, 1);
    let chrome = priced::session(mirrors, &model.draft, &stock, &car, || panic!("it has a price")).unwrap();
    assert_eq!((chrome.cost, chrome.level), (250, 1));
    // Taking the mirrors off is free.
    mirrors.set(&mut model, 2);
    assert_eq!(priced::session(mirrors, &model.draft, &stock, &car, || panic!("emptying is free")).unwrap().cost, 0);
}

#[test]
fn a_saved_bike_keeps_its_component_picks_and_drops_the_ones_its_mod_no_longer_has() {
    let mut model = shop_model();
    Row::Slot(3).set(&mut model, 1);
    let text = super::presets::encode(&model.draft).unwrap();
    assert!(text.contains("road_mirrors") && text.contains("t:mirrors.chrome"), "{text}");
    let back = super::presets::decode(&text, &model).unwrap();
    assert_eq!(back.parts.slots, model.draft.parts.slots);
    assert_eq!(back.parts, model.draft.parts);
    let gone = text.replace("t:mirrors.chrome", "t:mirrors.gone");
    assert!(super::presets::decode(&gone, &model).unwrap().parts.slots.is_empty(), "back to stock");
    // A car with no picks writes no table at all: the files of 2005 cars are as they were.
    let plain = super::presets::encode(&Selection { car: menu_fixture::FOLDER.into(), ..Default::default() }).unwrap();
    assert!(!plain.contains("slots"));
}

#[test]
fn the_riders_gear_follows_the_player_not_the_bike() {
    let mut model = shop_model();
    model.cars.push(super::Car { folder: "OTHERBIKE".into(), ..Default::default() });
    model.draft.gear.set(rider_gear::Part::Helmet, "HL_A");
    model.select("OTHERBIKE".into());
    assert_eq!(model.draft.car, "OTHERBIKE");
    assert_eq!(model.draft.gear.get(rider_gear::Part::Helmet), "HL_A", "the next bike is worn by the same rider");
    assert!(model.draft.parts.is_stock(), "while the car's own parts start again");
}
