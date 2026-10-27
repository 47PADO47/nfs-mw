use super::*;

fn cars(text: &str) -> BTreeMap<String, Change> {
    toml::from_str(text).unwrap()
}

#[test]
fn the_converted_cars_keep_their_2012_stats_and_nitrous_by_default() {
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

fn stats() -> ImportedStats {
    ImportedStats {
        mass_kg: 1400.0,
        front_weight_pct: 50.0,
        cg_height_m: 0.45,
        wheelbase_m: 2.6,
        track_m: [1.55, 1.55],
        tire_radius_m: [0.34, 0.34],
        tire_width_m: [0.25, 0.28],
        ride_height_m: 0.15,
        idle_rpm: 1000.0,
        redline_rpm: 7000.0,
        torque_curve: vec![[1000.0, 300.0], [4000.0, 400.0], [7000.0, 350.0]],
        gear_ratios: vec![3.5, 2.0, 1.4, 1.0, 0.8],
        reverse_ratio: 3.0,
        final_drive: 3.5,
        drive: "rwd",
        front_torque_share: 0.0,
        diff_lock: [0.0, 0.5],
        top_speed_kph: 280.0,
        tire_grip: [1.5, 1.5],
        brake_torque_nm: [1500.0, 1200.0],
        nitrous_boost: 0.9,
        drift: None,
    }
}

#[test]
fn the_multipliers_scale_the_imported_stats() {
    let mut car = stats();
    Performance { power: 1.25, mass: 0.9, grip: 1.1, brakes: 2.0, nitrous: true, nitrous_power: 0.5 }.apply(&mut car);
    assert_eq!(car.peak_torque_nm(), 500.0);
    assert!((car.mass_kg - 1260.0).abs() < 1e-3);
    assert!((car.tire_grip[0] - 1.65).abs() < 1e-5);
    assert_eq!(car.brake_torque_nm, [3000.0, 2400.0]);
    assert!((car.nitrous_boost - 0.45).abs() < 1e-6);
}

#[test]
fn the_default_multipliers_change_nothing_and_nitrous_can_be_turned_off() {
    let mut car = stats();
    Performance::default().apply(&mut car);
    assert_eq!(car, stats());
    Performance { nitrous: false, ..Performance::default() }.apply(&mut car);
    assert_eq!(car.nitrous_boost, 0.0);
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
