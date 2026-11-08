use std::collections::BTreeMap;
use std::sync::Arc;

use blackbox_components::{Catalog as Components, VehicleDef};
use nfsmw_data::car::customization::{Catalog, Category, Choice as Part, Selection};
use nfsmw_data::car::mods::components::{self, select};
use nfsmw_data::car::physics::{PerformanceCatalog, PerformanceCategory, PerformancePackage};

use super::*;

fn part(hash: u32, label: &str) -> Part {
    Part { hash, label: label.to_owned(), language_hash: None, carbon: false }
}

/// A car of the original game's kind: categories only.
fn plain_catalog() -> Catalog {
    let mut catalog = Catalog::default();
    catalog.choices[Category::Spoiler as usize] = vec![part(0xAA01, "Wing One"), part(0xAA02, "Wing Two")];
    catalog.choices[Category::Wheels as usize] =
        vec![part(0xBB01, "Rim One"), part(0xBB02, "Rim Two"), part(0xBB03, "Rim Three")];
    catalog.choices[Category::Vinyl as usize] = vec![part(0xCC01, "Flames")];
    catalog.paintable_wheels.insert(0xBB02);
    catalog.choices[Category::RimPaint as usize] = vec![part(0xDD01, "Gold")];
    catalog
}

fn plain<'a>(catalog: &'a Catalog, parts: &'a Selection, levels: &'a PerformanceCatalog) -> Inputs<'a> {
    Inputs {
        folder: "PLAINCAR",
        name: "Plain car",
        catalog: Some(catalog),
        parts,
        color: None,
        paintable: true,
        original_color: Some([1, 2, 3]),
        performance: Some((levels, PerformancePackage::default())),
    }
}

fn levels(engine: u8, brakes: u8) -> PerformanceCatalog {
    let mut levels = PerformanceCatalog::default();
    levels.categories[PerformanceCategory::Engine as usize].max_level = engine;
    levels.categories[PerformanceCategory::Brakes as usize].max_level = brakes;
    levels
}

#[test]
fn a_car_without_components_lists_its_categories_with_the_garages_words() {
    let (catalog, parts, levels) = (plain_catalog(), Selection::default(), levels(3, 2));
    let options = VehicleOptions::build(&plain(&catalog, &parts, &levels));
    let ids: Vec<&str> = options.slots.iter().map(|slot| slot.id.as_str()).collect();
    // Empty categories (body kit, hood, roof, paint, tint) are not listed; the levels follow the categories.
    assert_eq!(ids, ["spoiler", "rims", "rimpaint", "vinyl", "engine", "brakes"]);
    let spoiler = options.slot("spoiler").unwrap();
    assert_eq!((spoiler.label.as_str(), spoiler.group.as_str(), spoiler.components), ("Spoiler", "parts", false));
    assert_eq!(options.slot("vinyl").unwrap().label, "Vinyl / livery");
    assert_eq!(options.slot("vinyl").unwrap().group, "appearance");
    let engine = options.slot("engine").unwrap();
    assert_eq!(engine.group, "performance");
    assert_eq!(
        engine.choices.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
        ["Original equipment", "Stage 1", "Stage 2", "Stage 3"]
    );
    assert!(options.paintwork.is_some());
    assert!(options.missing.is_empty(), "only a bike lacks something");
}

#[test]
fn the_installed_choice_is_read_from_the_selection() {
    let (catalog, mut parts, levels) = (plain_catalog(), Selection::default(), levels(3, 0));
    parts.parts[Category::Wheels as usize] = Some(0xBB03);
    let options = VehicleOptions::build(&plain(&catalog, &parts, &levels));
    assert_eq!(options.slot("rims").unwrap().installed().label, "Rim Three");
    assert_eq!(options.slot("spoiler").unwrap().installed().label, "Stock");
    assert!(options.slot("rims").unwrap().changed() && !options.slot("spoiler").unwrap().changed());
    let mut performance = PerformancePackage::default();
    performance.levels[PerformanceCategory::Engine as usize] = 9;
    let options = VehicleOptions::build(&Inputs {
        performance: Some((&levels, performance)),
        ..plain(&catalog, &parts, &levels)
    });
    assert_eq!(options.slot("engine").unwrap().current, 3, "a level above the car's last is the last");
}

fn edit<'a>(parts: &'a mut Selection, color: &'a mut Option<[u8; 3]>, package: &'a mut PerformancePackage) -> Edit<'a> {
    Edit { parts, color, performance: package }
}

#[test]
fn a_choice_is_set_by_number_name_hash_or_stock() {
    let (catalog, mut parts, levels) = (plain_catalog(), Selection::default(), levels(3, 0));
    let options = VehicleOptions::build(&plain(&catalog, &parts, &levels));
    let (mut color, mut package) = (None, PerformancePackage::default());
    let mut set = |slot: &str, choice: &str| {
        let change = Change::Set { slot: slot.into(), choice: choice.into() };
        options.apply(Some(&catalog), &mut edit(&mut parts, &mut color, &mut package), &change)
    };
    assert_eq!(set("spoiler", "2").unwrap(), "Spoiler: Wing Two");
    assert_eq!(set("Rims", "rim thr").unwrap(), "Rims: Rim Three");
    assert_eq!(set("rims", "0000BB01").unwrap(), "Rims: Rim One", "a hash as the `parts` command prints it");
    assert_eq!(set("engine", "2").unwrap(), "Engine: Stage 2");
    assert!(set("spoiler", "3").is_err() && set("spoiler", "nothing like it").is_err());
    assert!(set("rims", "rim").unwrap_err().contains("several"), "an ambiguous name lists the candidates");
    assert!(set("hood", "1").unwrap_err().contains("no slot"), "an empty category is not a slot");
    assert_eq!(parts.parts[Category::Spoiler as usize], Some(0xAA02));
    assert_eq!(parts.parts[Category::Wheels as usize], Some(0xBB01));
    assert_eq!(package.levels[PerformanceCategory::Engine as usize], 2);
    let mut set = |slot: &str, choice: &str| {
        let change = Change::Set { slot: slot.into(), choice: choice.into() };
        options.apply(Some(&catalog), &mut edit(&mut parts, &mut color, &mut package), &change)
    };
    set("spoiler", "stock").unwrap();
    assert_eq!(parts.parts[Category::Spoiler as usize], None);
}

#[test]
fn fitting_other_rims_takes_the_rim_paint_off() {
    let (catalog, mut parts, levels) = (plain_catalog(), Selection::default(), levels(0, 0));
    let options = VehicleOptions::build(&plain(&catalog, &parts, &levels));
    let (mut color, mut package) = (None, PerformancePackage::default());
    let mut set = |slot: &str, choice: &str| {
        let change = Change::Set { slot: slot.into(), choice: choice.into() };
        options.apply(Some(&catalog), &mut edit(&mut parts, &mut color, &mut package), &change)
    };
    assert!(set("rimpaint", "1").is_err(), "rim paint needs aftermarket rims: the catalogue's rule refuses it");
    set("rims", "2").unwrap();
    set("rimpaint", "1").unwrap();
    set("rims", "1").unwrap();
    assert_eq!(parts.parts[Category::RimPaint as usize], None);
}

#[test]
fn a_refused_change_leaves_the_selection_alone() {
    let (catalog, mut parts, levels) = (plain_catalog(), Selection::default(), levels(0, 0));
    let options = VehicleOptions::build(&plain(&catalog, &parts, &levels));
    let (mut color, mut package) = (None, PerformancePackage::default());
    let change = Change::Set { slot: "rimpaint".into(), choice: "1".into() };
    let before = parts.clone();
    assert!(options.apply(Some(&catalog), &mut edit(&mut parts, &mut color, &mut package), &change).is_err());
    assert_eq!(parts, before);
}

#[test]
fn reset_and_colour() {
    let (catalog, mut parts, levels) = (plain_catalog(), Selection::default(), levels(3, 0));
    let options = VehicleOptions::build(&plain(&catalog, &parts, &levels));
    let (mut color, mut package) = (None, PerformancePackage::default());
    let mut run =
        |change: Change| options.apply(Some(&catalog), &mut edit(&mut parts, &mut color, &mut package), &change);
    run(Change::Pick { slot: "spoiler".into(), index: 1 }).unwrap();
    run(Change::Pick { slot: "engine".into(), index: 3 }).unwrap();
    run(Change::Color(Some([9, 8, 7]))).unwrap();
    run(Change::Reset).unwrap();
    assert!(parts.is_stock() && color.is_none() && package == PerformancePackage::default());
    let fixed = VehicleOptions { paintwork: None, ..options.clone() };
    let mut parts = Selection::default();
    let error = fixed
        .apply(None, &mut edit(&mut parts, &mut color, &mut package), &Change::Color(Some([1, 1, 1])))
        .unwrap_err();
    assert!(error.contains("fixed livery"));
}

const CATALOG: &str = r#"
[[component]]
id = "t:exhaust.stock"
name = "Stock exhaust"
[[component]]
id = "t:exhaust.race"
name = "Race exhaust"
price = 900
[[component]]
id = "t:exhaust.loud"
name = "Loud exhaust"
when = [{ kind = "tier", slot = "perf_engine", at_least = 1 }]
[[component]]
id = "t:engine.stock"
[[component]]
id = "t:engine.1"
name = "Engine kit"
tier = 1
[[component]]
id = "t:engine.2"
name = "Engine kit 2"
tier = 2
[[component]]
id = "t:mirrors.on"
name = "Mirrors"
[[component]]
id = "t:plate.on"
name = "Plate holder"
"#;

const VEHICLE: &str = r#"
id = "optbike"
[[slot]]
id = "exhaust"
group = "looks"
accepts = ["t:exhaust.stock", "t:exhaust.race", "t:exhaust.loud"]
default = "t:exhaust.stock"
[[slot]]
id = "perf_engine"
group = "performance"
accepts = ["t:engine.stock", "t:engine.1", "t:engine.2"]
default = "t:engine.stock"
[[slot]]
id = "road_mirrors"
group = "road"
accepts = ["t:mirrors.on"]
default = "t:mirrors.on"
optional = true
[[slot]]
id = "road_plate"
group = "road"
accepts = ["t:plate.on"]
default = "t:plate.on"
optional = true
"#;

fn publish_bike() {
    let catalog: Components = toml::from_str(CATALOG).unwrap();
    let vehicle: VehicleDef = toml::from_str(VEHICLE).unwrap();
    vehicle.check(&catalog).unwrap();
    components::publish(
        "OPTBIKE",
        components::Components { vehicle, catalog: Arc::new(catalog), parts: BTreeMap::new() },
    );
    nfsmw_data::car::mods::bike::publish("OPTBIKE", blackbox_vehicle::bike::BikeSpec::example());
}

fn bike_catalog() -> Catalog {
    let mut catalog = Catalog::default();
    // The garage's catalogue lists the exhaust's components under Spoiler, by the hash of the car and the component.
    catalog.choices[Category::Spoiler as usize] = ["t:exhaust.race", "t:exhaust.loud"]
        .map(|id| part(select::choice_hash("OPTBIKE", id), "from the catalogue"))
        .to_vec();
    catalog
}

fn bike<'a>(catalog: &'a Catalog, parts: &'a Selection, levels: &'a PerformanceCatalog) -> Inputs<'a> {
    Inputs {
        folder: "OPTBIKE",
        name: "Option bike",
        catalog: Some(catalog),
        parts,
        color: None,
        paintable: false,
        original_color: None,
        performance: Some((levels, PerformancePackage::default())),
    }
}

#[test]
fn a_bikes_slots_are_its_components_with_their_names_groups_and_prices() {
    publish_bike();
    let (catalog, parts) = (bike_catalog(), Selection::default());
    let options = VehicleOptions::build(&bike(&catalog, &parts, &levels(0, 0)));
    // The component slots in the order of the definition. The engine ladder comes from the components, not from the car's levels.
    let ids: Vec<&str> = options.slots.iter().map(|slot| slot.id.as_str()).collect();
    assert_eq!(ids, ["exhaust", "road_mirrors", "road_plate"]);
    assert!(options.bike && options.paintwork.is_none());
    let exhaust = options.slot("exhaust").unwrap();
    assert_eq!((exhaust.label.as_str(), exhaust.group.as_str(), exhaust.components), ("Exhaust", "looks", true));
    let labels: Vec<&str> = exhaust.choices.iter().map(|choice| choice.label.as_str()).collect();
    assert_eq!(
        labels,
        ["Stock exhaust", "Race exhaust", "Loud exhaust"],
        "the component's own names, not the catalogue's"
    );
    assert_eq!(exhaust.choices[0].id, "t:exhaust.stock");
    assert_eq!(exhaust.choices[1].id, "t:exhaust.race");
    assert_eq!(exhaust.choices[1].price, Some(900));
    assert_eq!(exhaust.choices[0].price, None, "a free component has no price");
}

#[test]
fn a_ladder_is_named_by_its_components_and_a_rule_blocks_a_choice() {
    publish_bike();
    let (catalog, parts) = (bike_catalog(), Selection::default());
    let levels = levels(0, 0);
    let mut engine = levels.clone();
    engine.categories[PerformanceCategory::Engine as usize].max_level = 2;
    let options = VehicleOptions::build(&bike(&catalog, &parts, &engine));
    let slot = options.slot("perf_engine").unwrap();
    assert_eq!((slot.label.as_str(), slot.group.as_str()), ("Engine", "performance"));
    let labels: Vec<&str> = slot.choices.iter().map(|choice| choice.label.as_str()).collect();
    assert_eq!(labels, ["Original equipment", "Engine kit", "Engine kit 2"]);
    // The loud exhaust needs the first engine kit: blocked until then.
    let exhaust = options.slot("exhaust").unwrap();
    assert!(exhaust.choices[2].blocked.is_some() && exhaust.choices[1].blocked.is_none());
    let (mut parts, mut color, mut package) = (Selection::default(), None, PerformancePackage::default());
    let loud = Change::Set { slot: "exhaust".into(), choice: "t:exhaust.loud".into() };
    let error = options.apply(Some(&catalog), &mut edit(&mut parts, &mut color, &mut package), &loud).unwrap_err();
    assert!(error.contains("cannot be fitted"), "{error}");
    options
        .apply(
            Some(&catalog),
            &mut edit(&mut parts, &mut color, &mut package),
            &Change::Set { slot: "perf_engine".into(), choice: "1".into() },
        )
        .unwrap();
    assert_eq!(package.levels[PerformanceCategory::Engine as usize], 1);
}

#[test]
fn a_component_slot_the_garage_has_no_category_for_is_kept_for_the_session() {
    publish_bike();
    let (catalog, parts, levels) = (bike_catalog(), Selection::default(), levels(0, 0));
    let options = VehicleOptions::build(&bike(&catalog, &parts, &levels));
    let mirrors = options.slot("road_mirrors").unwrap();
    assert_eq!(mirrors.store, Store::Session);
    let labels: Vec<&str> = mirrors.choices.iter().map(|choice| choice.label.as_str()).collect();
    assert_eq!(labels, ["Mirrors", "None"], "the default first, then nothing for an optional slot");
    let (mut parts, mut color, mut package) = (Selection::default(), None, PerformancePackage::default());
    let off = Change::Set { slot: "road_mirrors".into(), choice: "none".into() };
    options.apply(Some(&catalog), &mut edit(&mut parts, &mut color, &mut package), &off).unwrap();
    assert_eq!(parts.slots.get("road_mirrors").map(String::as_str), Some(""));
    let again = VehicleOptions::build(&bike(&catalog, &parts, &levels));
    assert_eq!(again.slot("road_mirrors").unwrap().installed().label, "None");
    let stock = Change::Set { slot: "road_mirrors".into(), choice: "stock".into() };
    again.apply(Some(&catalog), &mut edit(&mut parts, &mut color, &mut package), &stock).unwrap();
    assert!(parts.slots.is_empty());
}

#[test]
fn a_group_of_optional_slots_switches_off_and_on_and_a_fixed_group_does_not() {
    publish_bike();
    let (catalog, parts, levels) = (bike_catalog(), Selection::default(), levels(0, 0));
    let options = VehicleOptions::build(&bike(&catalog, &parts, &levels));
    let groups = options.groups();
    let road = groups.iter().find(|group| group.name == "road").unwrap();
    assert_eq!(road.slots, ["road_mirrors", "road_plate"]);
    assert!(road.switchable && road.on);
    assert!(!groups.iter().find(|group| group.name == "looks").unwrap().switchable);
    let (mut parts, mut color, mut package) = (Selection::default(), None, PerformancePackage::default());
    let off = Change::Group { name: "road".into(), on: false };
    options.apply(None, &mut edit(&mut parts, &mut color, &mut package), &off).unwrap();
    assert_eq!(parts.slots.len(), 2);
    let after = VehicleOptions::build(&bike(&catalog, &parts, &levels));
    assert!(!after.groups().iter().find(|group| group.name == "road").unwrap().on);
    after
        .apply(None, &mut edit(&mut parts, &mut color, &mut package), &Change::Group { name: "road".into(), on: true })
        .unwrap();
    assert!(parts.slots.is_empty());
    let looks = Change::Group { name: "looks".into(), on: false };
    let error = options.apply(None, &mut edit(&mut parts, &mut color, &mut package), &looks).unwrap_err();
    assert!(error.contains("cannot be switched"), "{error}");
    assert!(
        options
            .apply(None, &mut edit(&mut parts, &mut color, &mut package), &Change::ResetGroup("nope".into()))
            .is_err()
    );
}

#[test]
fn a_bike_without_road_equipment_says_what_it_would_need() {
    publish_bike();
    let (catalog, parts, levels) = (bike_catalog(), Selection::default(), levels(0, 0));
    let with_road = VehicleOptions::build(&bike(&catalog, &parts, &levels));
    assert!(with_road.missing.is_empty(), "this bike has road slots");
    let vehicle: VehicleDef = toml::from_str(
        "id = \"optbike2\"
[[slot]]
id = \"exhaust\"
accepts = [\"t:exhaust.stock\"]
default = \"t:exhaust.stock\"
",
    )
    .unwrap();
    let catalog_of: Components = toml::from_str(CATALOG).unwrap();
    components::publish(
        "OPTBIKE2",
        components::Components { vehicle, catalog: Arc::new(catalog_of), parts: BTreeMap::new() },
    );
    nfsmw_data::car::mods::bike::publish("OPTBIKE2", blackbox_vehicle::bike::BikeSpec::example());
    let without = VehicleOptions::build(&Inputs { folder: "OPTBIKE2", ..bike(&catalog, &parts, &levels) });
    assert_eq!(without.missing.len(), 1);
    assert!(without.groups_text().contains("missing: road equipment"), "{}", without.groups_text());
}

#[test]
fn the_text_lists_numbered_choices_with_the_installed_one_starred() {
    let (catalog, mut parts, levels) = (plain_catalog(), Selection::default(), levels(1, 0));
    parts.parts[Category::Spoiler as usize] = Some(0xAA02);
    let options = VehicleOptions::build(&plain(&catalog, &parts, &levels));
    let text = options.slots_text();
    assert!(text.contains("spoiler (Spoiler): 0: Stock, 1: Wing One, 2: Wing Two*"), "{text}");
    assert!(text.contains("engine (Engine): 0: Original equipment*, 1: Stage 1"), "{text}");
    assert!(text.contains("body colour: the vehicle's own (own #010203)"), "{text}");
    assert!(options.groups_text().contains("parts: spoiler, rims (1 changed, not switchable)"));
}
