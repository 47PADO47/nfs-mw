//! A car's engine sound set resolves to files that exist, with the values the data holds.

use blackbox_attrib::Database;
use blackbox_ginsu::GinsuTables;
use nfsmw_data::sound::{CarSound, EngineGroup, EngineSound, SoundUpgrades, car_sound};

use crate::install;

fn attributes() -> Option<(game_install::GameDir, Database)> {
    let dir = install()?;
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    Some((dir, db))
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_m3_gtr_resolves_to_the_cerbera_engine() {
    let Some((dir, db)) = attributes() else { return };
    let car = car_sound(&db, "BMWM3GTR", SoundUpgrades::default()).unwrap();
    let engine = &car.engine;
    assert_eq!(engine.name, "tvr_cerb");
    assert_eq!(engine.accel_loop, "GIN_TVR_Cerbera.gin");
    assert_eq!(engine.decel_loop, "GIN_TVR_Cerbera_DCL.gin");
    assert_eq!(engine.bank_main, "CAR_66_ENG_MB_EE.abk");
    assert_eq!(engine.banks_aux, ["CAR_66_ENG_MB_SPU.abk"]);
    assert_eq!(engine.sweet_banks, ["SWTN_CAR_66_MB.abk", "CAR_WHINE_00.abk"]);
    assert_eq!((engine.car_id, engine.group), (66, EngineGroup::V8));
    assert_eq!((engine.min_rpm, engine.max_rpm), (1500.0, 7784.0));
    assert!(engine.has_transmission_loop && engine.has_ginsu());
    assert_eq!(engine.decel_window.min_rpm, 2079.0);
    assert_eq!(engine.decel_window.max_rpm, 7570.0);
    assert_eq!(engine.mix.accel_loop_volume, 25500);
    assert_eq!(engine.mix.aems.steady, 1.0);
    assert_eq!(engine.mix.aems.large, 0.5);
    assert_eq!(engine.mix.ginsu.steady, 0.34);
    assert!((engine.rpm_map[1] - 0.4136).abs() < 1e-3 && (engine.rpm_map[2] - 0.6893).abs() < 1e-3);
    assert_eq!(car.engine_level, 0);
    assert!(car.turbo.is_none(), "the default turbo set means no turbo sound");
    assert_eq!(car.shift.bank, "GEAR_MED_Lev3.abk");
    assert_eq!(car.shift.up_disengage_fall.len(), 2);
    assert_eq!((car.shift.up_disengage_fall[0].rpm, car.shift.up_disengage_fall[0].time_ms), (1500, 210));
    assert_eq!((car.shift.up_disengage_fall[1].rpm, car.shift.up_disengage_fall[1].time_ms), (1200, 210));
    let engage = car.shift.up_engage.expect("an up-shift engage stage");
    assert_eq!((engage.rpm, engage.time_ms), (800, 300));
    assert_eq!(car.banks.nitrous_bank(), Some("Nitrous_00_MB.abk"));
    assert_eq!(car.banks.skid_bank(), Some("SKID_BIG_MB.abk"));
    assert!(car.accel_transition.peak_ms > 0);
    for file in car.files() {
        assert!(dir.exists(&file), "{file} is missing");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn a_turbo_car_has_a_turbo_sound() {
    let Some((dir, db)) = attributes() else { return };
    let car = car_sound(&db, "skylinezt", SoundUpgrades::default()).unwrap();
    let turbo = car.turbo.expect("the Skyline's turbo set");
    assert_eq!(turbo.bank, "TURBO_TUN_SML_1_MB.abk");
    assert!(turbo.charge_time > 0.0 && turbo.spool_volume > 0);
    assert!(dir.exists(&format!("SOUND/TURBO/{}", turbo.bank)));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn upgrades_pick_later_sets() {
    let Some((_, db)) = attributes() else { return };
    let stock = car_sound(&db, "clk500", SoundUpgrades::default()).unwrap();
    let maxed = car_sound(&db, "clk500", SoundUpgrades { engine: 4, transmission: 4, induction: 4 }).unwrap();
    assert_ne!(stock.engine.name, maxed.engine.name, "the engine sound changes with the engine upgrade");
    assert_eq!(stock.engine_level, 0);
    assert!(maxed.engine_level > 0);
    assert_ne!(stock.shift.name, maxed.shift.name);
    assert!(stock.turbo.is_none() && maxed.turbo.is_some(), "the supercharger sound arrives with the upgrade");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_car_with_an_engine_set_resolves_to_files_that_exist() {
    let Some((dir, db)) = attributes() else { return };
    let mut checked = 0;
    let mut with_loops = 0;
    for pvehicle in db.collections_of("pvehicle") {
        let Some(name) = pvehicle.name() else { continue };
        if pvehicle.get("engineaudio").is_none() {
            continue;
        }
        for upgrades in [SoundUpgrades::default(), SoundUpgrades { engine: 4, transmission: 4, induction: 4 }] {
            let car: CarSound = car_sound(&db, name, upgrades).unwrap_or_else(|e| panic!("{name}: {e:#}"));
            for file in car.files() {
                assert!(dir.exists(&file), "{name}: {file} is missing");
            }
            assert!(car.engine.min_rpm <= car.engine.max_rpm, "{name}");
            assert!(car.shift.up_engage.is_some(), "{name}: no up-shift engage stage");
            checked += 1;
        }
        let car = car_sound(&db, name, SoundUpgrades::default()).unwrap();
        if car.engine.has_ginsu() {
            with_loops += 1;
        }
    }
    assert!(checked >= 200, "only {checked} sets checked");
    assert!(with_loops >= 60, "only {with_loops} cars have a Ginsu loop");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_gin_tables_agree_with_the_engine_ranges() {
    let Some((dir, db)) = attributes() else { return };
    let mut checked = 0;
    for engine in db.collections_of("engineaudio") {
        let engine = EngineSound::from_collection(engine);
        if !engine.has_ginsu() {
            continue;
        }
        for (file, is_accel) in [(&engine.accel_loop, true), (&engine.decel_loop, false)] {
            let bytes = dir.read(&format!("SOUND/ENGINE/{file}")).unwrap_or_else(|e| panic!("{file}: {e}"));
            let (tables, payload_at) = GinsuTables::parse(&bytes).unwrap_or_else(|e| panic!("{file}: {e}"));
            assert_eq!(bytes.len() - payload_at, tables.xas_payload_len(), "{file}: payload size");
            assert_eq!(tables.seg_count(), 50, "{file}");
            let rising = tables.freq_pos().first() < tables.freq_pos().last();
            assert_eq!(rising, is_accel, "{file}: accelerate files rise, decelerate files fall");
            if is_accel {
                let ratio = tables.max_frequency() / engine.max_rpm;
                assert!(
                    (0.8..=1.4).contains(&ratio),
                    "{}: MaxRPM {} against {}",
                    engine.name,
                    engine.max_rpm,
                    tables.max_frequency()
                );
            }
            checked += 1;
        }
    }
    assert!(checked >= 120, "only {checked} loops checked");
}
