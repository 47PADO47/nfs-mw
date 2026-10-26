use blackbox_carparts::CarTypeInfo;

use super::*;
use crate::pursuit::HeatRow;

/// A stock car type with the given type name and folder, as the car table lists it.
fn stock(type_name: &str, folder: &str, manufacturer: &str) -> CarTypeInfo {
    CarTypeInfo {
        type_name: type_name.to_owned(),
        base_model_name: type_name.to_ascii_lowercase(),
        geometry_filename: format!("CARS\\{folder}\\GEOMETRY.BIN"),
        manufacturer: manufacturer.to_owned(),
        type_name_hash: 0,
        index: 0,
        usage_type: 0,
        default_skin_number: 0,
        skinnable: false,
        default_base_paint: 0,
    }
}

/// A fake car table: two stock cars.
fn table() -> Vec<CarTypeInfo> {
    vec![stock("BMWM3GTR", "BMWM3GTR", "BMW"), stock("CAMARO", "CAMARO", "CHEVROLET")]
}

fn spec(folder: &str) -> CarSpec {
    CarSpec {
        folder: folder.to_owned(),
        owner: "example.pack".to_owned(),
        name: None,
        manufacturer: None,
        player: true,
        cop_heat: None,
        geometry_in_mod: true,
        stats: CarStats::default(),
        engine_sound: None,
        car_effects: None,
    }
}

fn reasons(rejected: &[Rejected]) -> Vec<(&str, Reason)> {
    rejected.iter().map(|r| (r.folder.as_str(), r.reason.clone())).collect()
}

#[test]
fn a_mod_car_is_a_table_entry_of_its_own_with_its_own_geometry_and_brand() {
    let mut request = spec("MYCAR");
    request.manufacturer = Some("Acme".to_owned());
    request.name = Some("My Car".to_owned());
    let (cars, rejected) = register(&table(), &[request]);
    assert!(rejected.is_empty());
    let car = &cars[0];
    assert_eq!(car.car_type.type_name, "MYCAR", "nothing is taken from a car of the game");
    assert_eq!(car.car_type.base_model_name, "MYCAR");
    assert_eq!(car.car_type.type_name_hash, blackbox_hash::bstring_hash("MYCAR"));
    assert_eq!(car.car_type.index, 2, "after the two stock cars");
    assert_eq!(car.car_type.geometry_filename, "CARS\\MYCAR\\GEOMETRY.BIN");
    assert_eq!(car.car_type.manufacturer, "Acme");
    assert_eq!(car.name.as_deref(), Some("My Car"));
    assert_eq!(car.owner, "example.pack");
}

#[test]
fn a_car_without_geometry_in_the_mod_is_refused() {
    let mut request = spec("MYCAR");
    request.geometry_in_mod = false;
    let (cars, rejected) = register(&table(), &[request]);
    assert!(cars.is_empty());
    assert_eq!(reasons(&rejected), [("MYCAR", Reason::NoGeometry)]);
}

#[test]
fn a_folder_that_is_a_stock_car_or_taken_by_an_earlier_mod_is_refused() {
    let specs = [spec("CAMARO"), spec("MYCAR"), spec("MYCAR")];
    let (cars, rejected) = register(&table(), &specs);
    assert_eq!(reasons(&rejected), [("CAMARO", Reason::NameTaken), ("MYCAR", Reason::NameTaken)]);
    assert_eq!(cars.len(), 1);
}

#[test]
fn mod_cars_get_the_next_free_table_indices() {
    let specs = [spec("MYCAR"), spec("MYCAR2")];
    let (cars, rejected) = register(&table(), &specs);
    assert!(rejected.is_empty(), "{rejected:?}");
    assert_eq!((cars[0].car_type.index, cars[1].car_type.index), (2, 3));
    assert_eq!(cars[1].car_type.type_name, "MYCAR2");
}

#[test]
fn a_cop_heat_level_the_game_lacks_is_refused() {
    let mut request = spec("MYCAR");
    request.cop_heat = Some(vec![1, 11]);
    let (cars, rejected) = register(&table(), &[request]);
    assert!(cars.is_empty());
    assert_eq!(reasons(&rejected), [("MYCAR", Reason::NoSuchHeat(11))]);
}

#[test]
fn a_cop_only_mod_car_is_not_a_player_car_and_a_stock_car_is() {
    let mut request = spec("MYCOP");
    request.player = false;
    request.cop_heat = Some(vec![1]);
    let (cars, _) = register(&table(), &[request]);
    assert!(!player_in(&cars, "MYCOP"));
    assert!(player_in(&cars, "CAMARO"));
    assert!(player_in(&cars, "unknown folder"));
}

#[test]
fn the_admitted_car_is_found_by_its_folder_in_the_table() {
    let (cars, _) = register(&table(), &[spec("MYCAR")]);
    let mut types = table();
    types.extend(cars.iter().map(|c| c.car_type.clone()));
    let found = find_type(&types, "MYCAR").expect("the mod car");
    assert_eq!(found.type_name, "MYCAR");
    assert_eq!(find_type(&types, "CAMARO").map(|t| t.geometry_filename.as_str()), Some("CARS\\CAMARO\\GEOMETRY.BIN"));
}

#[test]
fn the_garage_takes_a_mod_cars_name_and_hides_a_cop_only_car() {
    let mut named = spec("MYCAR");
    named.name = Some("My Car".to_owned());
    let mut cop = spec("MYCOP");
    cop.player = false;
    cop.cop_heat = Some(vec![1]);
    let (cars, _) = register(&table(), &[named, cop]);
    assert_eq!(display_name_in(&cars, "mycar"), Some("My Car"));
    assert_eq!(display_name_in(&cars, "CAMARO"), None, "a stock car keeps the language table's name");
    assert!(player_in(&cars, "MYCAR"));
    assert!(!player_in(&cars, "MYCOP"), "the garage hides a cop-only car");
}

#[test]
fn a_cop_row_gets_the_mod_car_in_its_heat_levels_only() {
    let mut request = spec("MYCOP");
    request.player = false;
    request.cop_heat = Some(vec![2, 3]);
    let (cars, _) = register(&table(), &[request]);
    let mut row = HeatRow {
        cops: Vec::new(),
        patrol_cars: 0,
        full_engagement_cops: 0,
        time_between_first_four_spawns: 10.0,
        time_between_cop_spawns: 5.0,
    };
    add_cop_rows(&cars, 1, &mut row);
    assert!(row.cops.is_empty());
    add_cop_rows(&cars, 3, &mut row);
    assert_eq!(row.cops, [CopRecord { name: "mycop".to_owned(), count: COP_COUNT, chance: COP_CHANCE }]);
}

/// The only test in this binary that publishes: the list is process-wide.
#[test]
fn published_cars_accumulate_and_a_folder_is_added_once() {
    let (first, _) = register(&table(), &[spec("PUBTEST")]);
    publish(first.clone());
    assert_eq!(published(), first.as_slice());
    assert_eq!(car_types().last().map(|t| t.geometry_filename.as_str()), Some("CARS\\PUBTEST\\GEOMETRY.BIN"));
    let (second, _) = register(&table(), &[spec("PUBLATER")]);
    publish([first.clone(), second.clone()].concat());
    assert_eq!(published().len(), 2, "the repeated folder is not added again");
    assert_eq!(published()[1], second[0]);
    publish(Vec::new());
    assert_eq!(published().len(), 2);
}

#[test]
fn a_mod_car_keeps_its_own_brand_and_tuning() {
    let mut first = spec("MYCAR");
    first.manufacturer = Some("Acme".to_owned());
    first.stats = CarStats { mass_kg: 1300.0, ..CarStats::default() };
    let (cars, rejected) = register(&table(), &[first, spec("MYCAR2")]);
    assert!(rejected.is_empty(), "{rejected:?}");
    assert_eq!(find_in(&cars, "mycar").and_then(|car| car.manufacturer.as_deref()), Some("Acme"));
    assert_eq!(find_in(&cars, "MYCAR2").and_then(|car| car.manufacturer.as_deref()), None);
    assert_eq!(find_in(&cars, "mycar").map(|car| car.stats.mass_kg), Some(1300.0));
    assert_eq!(find_in(&cars, "MYCAR2").map(|car| car.stats.mass_kg), Some(CarStats::default().mass_kg));
    assert!(find_in(&cars, "CAMARO").is_none(), "a stock car is not a mod car");
}
