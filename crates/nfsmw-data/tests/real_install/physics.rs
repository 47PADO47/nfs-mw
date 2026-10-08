//! A car's physics data reads into the vehicle library's parameter structs with sane values.

use blackbox_attrib::Database;
use nfsmw_data::car::physics::{self, Fields};

use crate::install;

fn attributes() -> Option<Database> {
    let dir = install()?;
    Some(Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin"))
}

fn link<'a>(db: &'a Database, car: &str, field: &str) -> Fields<'a> {
    let pvehicle = db.collection("pvehicle", car).unwrap_or_else(|| panic!("no pvehicle {car}"));
    Fields(pvehicle.follow(field).unwrap_or_else(|| panic!("{car}: no {field} link")))
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_m3_gtr_powertrain_matches_the_data() {
    let Some(db) = attributes() else { return };
    let engine = physics::engine(link(&db, "bmwm3gtr", "engine"));
    assert_eq!(engine.torque.len(), 9);
    assert_eq!((engine.idle, engine.red_line, engine.max_rpm), (800.0, 8500.0, 9500.0));
    assert_eq!(engine.torque[4], 283.0);
    assert_eq!(engine.speed_limiter, [0.0, 0.0], "no governor");
    let t = physics::transmission(link(&db, "bmwm3gtr", "transmission"));
    assert_eq!(t.gear_ratio.len(), 8, "reverse, neutral and six forward gears");
    assert_eq!(t.gear_ratio[1], 0.0);
    assert_eq!(t.final_gear, 3.85);
    assert!((0.0..=1.0).contains(&t.torque_split));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_car_has_a_sane_engine_and_gearbox() {
    let Some(db) = attributes() else { return };
    let mut checked = 0;
    for car in db.collections_of("pvehicle") {
        let Some(name) = car.name() else { continue };
        let (Some(engine), Some(trans)) = (car.follow("engine"), car.follow("transmission")) else { continue };
        let (engine, trans) = (physics::engine(Fields(engine)), physics::transmission(Fields(trans)));
        if engine.torque.len() < 2 {
            continue; // not a real car (a template or a prop)
        }
        assert!(engine.idle > 300.0 && engine.idle < engine.red_line, "{name}: idle {}", engine.idle);
        assert!((4000.0..=14000.0).contains(&engine.red_line), "{name}: redline {}", engine.red_line);
        assert!(engine.torque.iter().all(|&t| t > 0.0 && t < 2000.0), "{name}: torque {:?}", engine.torque);
        let forward = trans.gear_ratio.iter().skip(2).filter(|&&r| r > 0.0).count();
        assert!((3..=7).contains(&forward), "{name}: {forward} forward gears");
        assert!(trans.final_gear > 1.0 && trans.final_gear < 8.0, "{name}: final {}", trans.final_gear);
        checked += 1;
    }
    assert!(checked >= 50, "only {checked} cars checked");
}
