//! The shop's sections for a vehicle with components, and the golden lists of a car without any (the original game's categories).

use super::visual::{rider_pages, shop_sells_gear};
use super::*;
use crate::garage::menu_fixture::shop_model;
use crate::garage::options::{Inputs, VehicleOptions};
use crate::garage::{Car, Selection};
use nfsmw_data::car::customization::{Catalog, Choice};
use nfsmw_data::car::physics::{PerformanceCatalog, PerformanceCategory};

fn labels(page: Page, model: &Model) -> Vec<String> {
    items(page, model).into_iter().map(|item| item.label).collect()
}

fn parts(backroom: bool) -> Page {
    Page::Categories { section: Section::Parts, backroom }
}

fn performance(backroom: bool) -> Page {
    Page::Categories { section: Section::Performance, backroom }
}

#[test]
fn a_bike_lists_its_own_slots_in_the_sections_by_their_names() {
    let model = shop_model();
    assert_eq!(labels(parts(false), &model), ["Exhaust", "Rims", "Road mirrors"]);
    assert_eq!(labels(performance(false), &model), ["Engine"]);
    // Every entry opens the choices of the row that keeps it: the rims no longer go through the brand pages.
    let entries = items(parts(false), &model);
    let next: Vec<Option<Page>> = entries.iter().map(|item| item.next).collect();
    assert_eq!(
        next,
        [
            Some(Page::Choices { row: Row::Part(Part::Spoiler), backroom: false }),
            Some(Page::Choices { row: Row::Part(Part::Wheels), backroom: false }),
            Some(Page::Choices { row: Row::Slot(3), backroom: false }),
        ]
    );
    assert!(entries.iter().all(|item| item.enabled));
    assert_eq!(labels(Page::Main, &model), ["Parts", "Performance", "Visual"]);
}

#[test]
fn a_slot_with_nothing_to_choose_is_not_listed_and_a_section_without_slots_is_not_offered() {
    let mut model = shop_model();
    model.cars[0].performance.categories[PerformanceCategory::Engine as usize].max_level = 0;
    // The engine ladder is the bike's only performance slot: without a stage to choose the section is gone.
    assert!(labels(performance(false), &model).is_empty());
    assert_eq!(labels(Page::Main, &model), ["Parts", "Visual"]);
}

#[test]
fn the_backroom_pays_with_markers_so_it_lists_only_the_slots_a_marker_buys() {
    let mut model = shop_model();
    // The career holds a spoiler marker and a rims marker (kinds 10 and 11): the page lists what they buy and the mirrors never.
    model.shop_marker_counts[10] = 1;
    model.shop_marker_counts[11] = 1;
    assert_eq!(labels(parts(true), &model), ["Exhaust", "Rims"], "mirrors have no marker");
    let engine = &slots::section(Section::Performance, &model, true).unwrap()[0];
    assert_eq!(engine.next, Some(Page::Choices { row: Row::Junkman(PerformanceCategory::Engine), backroom: true }));
}

#[test]
fn a_refused_choice_is_listed_but_cannot_be_picked_and_says_why() {
    let model = shop_model();
    let choices = items(Page::Choices { row: Row::Part(Part::Spoiler), backroom: false }, &model);
    let names: Vec<&str> = choices.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(names, ["Stock exhaust", "Race exhaust", "Loud exhaust"]);
    assert_eq!(choices.iter().map(|item| item.enabled).collect::<Vec<_>>(), [true, true, false]);
    assert!(choices[2].blocked.as_deref().is_some_and(|reason| reason.contains("perf_engine")));
    assert!(choices[0].blocked.is_none() && choices[1].blocked.is_none());
}

#[test]
fn the_title_of_a_slots_page_is_the_slots_name() {
    let model = shop_model();
    let exhaust = Page::Choices { row: Row::Part(Part::Spoiler), backroom: false };
    assert_eq!(exhaust.component_title(&model).as_deref(), Some("Exhaust"));
    let mirrors = Page::Choices { row: Row::Slot(3), backroom: false };
    assert_eq!(mirrors.component_title(&model).as_deref(), Some("Road mirrors"));
    assert_eq!(Page::Main.component_title(&model), None);
}

#[test]
fn the_visual_section_leaves_out_the_pages_a_bike_has_nothing_behind() {
    let model = shop_model();
    let visual = labels(Page::Categories { section: Section::Visual, backroom: false }, &model);
    // Paint is always there; a bike with no vinyls, decals, tint or gauge skins lists none of them.
    assert_eq!(visual, ["Paint"]);
}

fn part(hash: u32, label: &str) -> Choice {
    Choice { hash, label: label.into(), language_hash: None, carbon: false }
}

/// A car of the original game's kind: categories only, no components.
fn plain() -> Model {
    let mut catalog = Catalog::default();
    catalog.choices[Part::Spoiler as usize] = vec![part(1, "Wing")];
    catalog.choices[Part::Wheels as usize] = vec![part(2, "Rim")];
    let mut performance = PerformanceCatalog::default();
    performance.categories[PerformanceCategory::Engine as usize].max_level = 3;
    performance.categories[PerformanceCategory::Brakes as usize].max_level = 2;
    let car = Car {
        folder: "PLAINCAR".into(),
        name: "Plain car".into(),
        performance,
        parts: Some(catalog.into()),
        ..Default::default()
    };
    let draft = Selection { car: "PLAINCAR".into(), ..Default::default() };
    Model { cars: vec![car], applied: draft.clone(), draft, ..Default::default() }
}

#[test]
fn a_car_of_the_original_game_keeps_the_fixed_categories_greyed_where_it_has_none() {
    let model = plain();
    let shown = |page: Page| -> Vec<(String, bool)> {
        items(page, &model).into_iter().map(|item| (item.label, item.enabled)).collect()
    };
    // The original's order and words, an empty category dimmed and not dropped.
    let expected = [("Body kit", false), ("Spoiler", true), ("Rims", true), ("Hood", false), ("Roof scoop", false)];
    assert_eq!(shown(parts(false)), expected.map(|(label, enabled)| (label.to_owned(), enabled)));
    let performance = shown(performance(false));
    assert_eq!(performance.len(), 7, "all seven, in the original's order");
    assert_eq!(performance.iter().filter(|(_, enabled)| *enabled).count(), 2);
    // The rims keep their brand pages, the spoilers their groups.
    let entries = items(parts(false), &model);
    assert_eq!(entries[2].next, Some(Page::RimBrands { backroom: false }));
    assert!(matches!(entries[1].next, Some(Page::Spoilers { .. })));
    // The rows of such a car take the catalogue's own labels and the plain hash: nothing came from components.
    let (labels, at) = Row::Part(Part::Spoiler).options(&model);
    assert_eq!((labels.as_slice(), at), (["Stock / remove".to_owned(), "Wing".to_owned()].as_slice(), 0));
    assert_eq!(Page::Choices { row: Row::Part(Part::Spoiler), backroom: false }.component_title(&model), None);
}

#[test]
fn the_fixed_categories_are_what_the_vehicle_options_list_for_a_car_without_components() {
    // The same categories are the options' fallback model: a category is listed there exactly when the shop enables it.
    let model = plain();
    let car = &model.cars[0];
    let options = VehicleOptions::build(&Inputs {
        folder: &car.folder,
        name: &car.name,
        catalog: car.parts.as_deref(),
        parts: &model.draft.parts,
        color: None,
        paintable: false,
        original_color: None,
        performance: Some((&car.performance, model.draft.performance)),
    });
    let keys = [
        (Part::Body, "body"),
        (Part::Spoiler, "spoiler"),
        (Part::Wheels, "rims"),
        (Part::Hood, "hood"),
        (Part::Roof, "roof"),
    ];
    for (category, key) in keys {
        let enabled = items(parts(false), &model).iter().any(|item| item.label == category.label() && item.enabled);
        assert_eq!(options.slot(key).is_some(), enabled, "{key}");
    }
    assert!(options.slot("engine").is_some() && options.slot("brakes").is_some() && options.slot("nitrous").is_none());
    assert!(VehicleOptions::of_draft(&model).is_none(), "the shop's rows are the category rows for such a car");
}

#[test]
fn the_riders_gear_is_for_sale_in_a_bikes_career_shop_only() {
    for bike in [false, true] {
        for shop in [false, true] {
            for backroom in [false, true] {
                for setting in [false, true] {
                    let sold = shop_sells_gear(bike, shop, backroom, setting);
                    assert_eq!(sold, bike && shop && !backroom && setting, "{bike} {shop} {backroom} {setting}");
                }
            }
        }
    }
    // No shop, no gear page: My Cars and the free roam garage list none, whatever catalogue is loaded.
    let mut model = shop_model();
    model.shop = false;
    assert!(rider_pages(&model).is_empty());
}
