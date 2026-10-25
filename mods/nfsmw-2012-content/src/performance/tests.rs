use super::*;

fn cars(text: &str) -> BTreeMap<String, Change> {
    toml::from_str(text).unwrap()
}

#[test]
fn the_converted_cars_have_nitrous_and_drive_like_their_clone_by_default() {
    let performance = Performance::default();
    assert!(performance.nitrous);
    assert_eq!((performance.power, performance.mass, performance.grip, performance.brakes), (1.0, 1.0, 1.0, 1.0));
}

#[test]
fn a_car_table_changes_only_the_keys_it_names() {
    let general = Performance { power: 1.1, ..Performance::default() };
    let cars = cars("[porsche]\nmass = 0.9\n[POR911CARR12]\nnitrous = false\n");
    let tuning = Tuning { general, cars: &cars };
    let porsche = tuning.for_car("POR911GT208", "Porsche 911 GT2 2008");
    assert_eq!((porsche.power, porsche.mass, porsche.nitrous), (1.1, 0.9, true));
    let carrera = tuning.for_car("POR911CARR12", "Porsche 911 Carrera S 2012");
    assert_eq!((carrera.power, carrera.mass, carrera.nitrous), (1.1, 0.9, false));
    let other = tuning.for_car("BMW1M12", "BMW 1M 2012");
    assert_eq!(other, general);
}

#[test]
fn the_manifest_table_is_valid_toml_with_every_key() {
    let table = Performance { power: 1.25, nitrous: false, ..Performance::default() }.manifest_table();
    let parsed: toml::Table = toml::from_str(&table).unwrap();
    let performance = parsed["performance"].as_table().unwrap();
    assert_eq!(performance.len(), 6);
    assert_eq!(performance["power"].as_float(), Some(1.25));
    assert_eq!(performance["nitrous"].as_bool(), Some(false));
}

#[test]
fn numbers_outside_the_games_ranges_name_their_key() {
    let general = Performance { power: 9.0, ..Performance::default() };
    let none = BTreeMap::new();
    let error = Tuning { general, cars: &none }.check().unwrap_err().to_string();
    assert!(error.contains("[performance]") && error.contains("power"), "{error}");
    let cars = cars("[BMW1M12]\ngrip = 0.0\n");
    let error = Tuning { general: Performance::default(), cars: &cars }.check().unwrap_err().to_string();
    assert!(error.contains("[car.BMW1M12]") && error.contains("grip"), "{error}");
}

#[test]
fn an_unknown_key_is_an_error() {
    assert!(toml::from_str::<BTreeMap<String, Change>>("[x]\ntorque = 2.0\n").is_err());
    assert!(toml::from_str::<Performance>("torque = 2.0").is_err());
}

#[test]
fn the_settings_example_of_the_docs_parses_and_tunes_the_cars_it_names() {
    let config = crate::config::Config::parse(
        "source = 'D:/2012'\ncars = \"all\"\n\n[performance]\npower = 1.05\nnitrous = true\n\n[car.POR911CARR12]\npower = 1.2\nmass = 0.95\n\n[car.\"Aston Martin\"]\ngrip = 1.05\n",
    )
    .unwrap();
    let carrera = config.tuning().for_car("POR911CARR12", "Porsche 911 Carrera S 2012");
    assert_eq!((carrera.power, carrera.mass, carrera.grip), (1.2, 0.95, 1.0));
    let aston = config.tuning().for_car("AST1", "Aston Martin DB9 2008");
    assert_eq!((aston.power, aston.grip), (1.05, 1.05));
}

#[test]
fn a_setting_outside_its_range_stops_the_conversion_before_it_starts() {
    let error = crate::config::Config::parse("[performance]\ngrip = 7.0\n").unwrap_err();
    assert!(format!("{error:#}").contains("grip"), "{error:#}");
}
