use super::*;
use crate::car::{CarTables, physics::PhysicsData};
use blackbox_attrib::Value;

#[test]
#[ignore = "requires an installed game"]
fn installed_performance_packages_match_original_records_for_every_playable_car() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let tables = CarTables::load(&dir).unwrap();
    let mut cars = 0;
    let mut stages = 0;
    let mut junkman = 0;
    let folders = crate::car::list(&dir);
    for car in folders
        .iter()
        .filter_map(|folder| tables.car_type_for_folder(folder))
        .filter(|car| matches!(car.usage_type, 0 | 1))
    {
        let Ok(stock) = data.car(&car.type_name) else { continue };
        if stock.spec.engine.torque.len() < 2 || stock.spec.transmission.gear_ratio.len() < 3 {
            continue;
        }
        let options = PerformanceCatalog::read(data.database(), &car.type_name).unwrap();
        let base = data.car_with_performance(&car.type_name, PerformancePackage::default()).unwrap();
        assert_eq!(format!("{:?}", stock.spec), format!("{:?}", base.spec), "stock {}", car.type_name);
        assert_eq!(stock.spec.aero.airborne_scale, 0.8);
        for category in PerformanceCategory::ALL {
            for level in 0..=options.categories[category as usize].max_level {
                let mut package = PerformancePackage::default();
                package.levels[category as usize] = level;
                let ordinary = data.car_with_performance(&car.type_name, package).unwrap();
                assert!(ordinary.spec.engine.torque.iter().all(|v| v.is_finite()));
                assert!(ordinary.spec.transmission.gear_ratio.iter().all(|v| v.is_finite()));
                if options.junkman_available(category, package) {
                    package.junkman |= 1 << category as u8;
                    let upgraded = data.car_with_performance(&car.type_name, package).unwrap();
                    let again = data.car_with_performance(&car.type_name, package).unwrap();
                    assert_eq!(
                        format!("{:?}", upgraded.spec),
                        format!("{:?}", again.spec),
                        "Junkman must be idempotent"
                    );
                    junkman += 1;
                }
                stages += 1;
            }
        }
        let mut invalid = options.maximum();
        invalid.levels[0] += 1;
        assert!(data.car_with_performance(&car.type_name, invalid).is_err());
        cars += 1;
    }
    assert!(cars >= 40 && stages > 600 && junkman > 100, "{cars} cars / {stages} stages / {junkman} Junkman");
    println!("{cars} cars / {stages} individual stage combinations / {junkman} Junkman combinations");
}

#[test]
#[ignore = "requires an installed game"]
fn installed_stage_curves_nitrous_and_junkman_follow_original_resolution() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let db = data.database();
    let car = "cobaltss";
    let vehicle = db.collection("pvehicle", car).unwrap();
    let linked = |field: &str| match vehicle.get(field).unwrap() {
        Value::Array(items) => items.iter().map(|v| db.resolve(v.as_ref_spec().unwrap()).unwrap()).collect::<Vec<_>>(),
        _ => panic!("installed category must be an array"),
    };
    let stock = data.car(car).unwrap();
    assert!(!stock.spec.nos.present(), "a stock Cobalt must not receive a free nitrous kit");
    let engine = linked("engine");
    let low = super::fields::numbers(engine[0].get("TORQUE"));
    let high = super::fields::numbers(engine[1].get("TORQUE"));
    for stage in 1..=4 {
        let mut package = PerformancePackage::default();
        package.levels[PerformanceCategory::Engine as usize] = stage;
        let actual = data.car_with_performance(car, package).unwrap();
        for (i, torque) in actual.spec.engine.torque.iter().enumerate() {
            let expected = low[i] + (high[i] - low[i]) * stage as f32 / 4.0;
            assert!((torque - expected).abs() < 0.0001);
        }
        package.junkman = 1 << PerformanceCategory::Engine as u8;
        let tuned = data.car_with_performance(car, package).unwrap();
        for (a, b) in actual.spec.engine.torque.iter().zip(&tuned.spec.engine.torque) {
            assert!((b - a * 1.1).abs() < 0.0001);
        }
    }
    let nos = linked("nos");
    let mut package = PerformancePackage::default();
    package.levels[PerformanceCategory::Nitrous as usize] = 1;
    let actual = data.car_with_performance(car, package).unwrap();
    let a = nos[1].get_f32("NOS_CAPACITY").unwrap();
    let b = nos[2].get_f32("NOS_CAPACITY").unwrap();
    assert!((actual.spec.nos.nos_capacity - (a * 2.0 / 3.0 + b / 3.0)).abs() < 0.0001);
    package = PerformancePackage::default();
    package.junkman = 1 << PerformanceCategory::Nitrous as u8;
    assert!(data.car_with_performance(car, package).is_err());
    assert!(data.car("bmwm3gtr").unwrap().spec.nos.present(), "stock equipment can already include NOS");
}
