//! The cop waves read from the install match the tables in `docs/specs/ai-pursuit-heat.md` §2.7.

use blackbox_attrib::Database;
use nfsmw_data::pursuit::{ai_vehicle, heat_row, patrol_speeds_mph};

use crate::install;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_helicopter_record_has_no_car_links() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    let heli = db.collection("pvehicle", "copheli").expect("the copheli record");
    for link in ["chassis", "tires", "brakes"] {
        assert!(heli.follow(link).is_none(), "copheli has a {link} link");
    }
    let car = db.collection("pvehicle", "bmwm3gtr").expect("the bmwm3gtr record");
    assert!(car.follow("chassis").is_some(), "a car has a chassis link");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_cop_waves_match_the_documented_table() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    let wave = |heat| {
        let row = heat_row(&db, heat).expect("a row");
        row.cops.iter().map(|c| (c.name.clone(), c.count, c.chance)).collect::<Vec<_>>()
    };
    assert_eq!(wave(1), vec![("copmidsize".to_owned(), 4, 100)]);
    assert_eq!(wave(3), vec![("copgto".to_owned(), 7, 100)]);
    assert_eq!(wave(4), vec![("copgtoghost".to_owned(), 8, 100), ("copheli".to_owned(), 1, 50)]);
    assert_eq!(wave(6)[0], ("copsportghost".to_owned(), 8, 80));
    assert_eq!(heat_row(&db, 1).unwrap().full_engagement_cops, 5);
    assert_eq!(heat_row(&db, 5).unwrap().full_engagement_cops, 25);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn cop_cars_have_their_documented_speed_multipliers() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    // docs/specs/ai-driver-control.md section 10.
    let sport = ai_vehicle(&db, "copsport");
    assert_eq!((sport.max_speed_kmh, sport.acceleration_multiplier, sport.top_speed_multiplier), (400.0, 1.5, 2.0));
    let midsize = ai_vehicle(&db, "copmidsize");
    assert_eq!(
        (midsize.max_speed_kmh, midsize.acceleration_multiplier, midsize.top_speed_multiplier),
        (280.0, 1.05, 1.6)
    );
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_patrol_speeds_are_the_documented_defaults() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    assert_eq!(patrol_speeds_mph(&db), (50.0, 71.0));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_siren_rows_are_read_for_every_heat_level() {
    use nfsmw_data::pursuit_siren::siren_data;
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    let data = siren_data(&db);
    assert_eq!(data.rows.len(), 10);
    for (level, row) in data.rows.iter().enumerate() {
        println!("heat {}: {row:?}", level + 1);
        assert!(row.wail_period > 0.0 && row.max_yelp_time > 0.0, "heat {} has no wail/yelp times", level + 1);
        assert!(row.scream_period > 0.0 && row.max_scream_time > 0.0, "heat {} has no scream times", level + 1);
        assert!(row.init_min_period > 0.0, "heat {} has no opening gap", level + 1);
    }
    println!("intensity: heat {} or {} s", data.high_intensity_heat, data.high_intensity_secs);
    assert_eq!(data.high_intensity_heat, 2);
    assert_eq!(data.high_intensity_secs, 150.0);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_backup_rows_match_the_documented_table() {
    let Some(dir) = install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    let rows = nfsmw_data::pursuit::backup_rows(&db);
    assert_eq!(rows.len(), 10);
    let radius: Vec<f32> = rows.iter().map(|r| r.engagement_radius).collect();
    let trigger: Vec<u32> = rows.iter().map(|r| r.trigger).collect();
    let timer: Vec<f32> = rows.iter().map(|r| r.call_timer).collect();
    assert_eq!(radius, [50.0, 50.0, 50.0, 50.0, 50.0, 50.0, 50.0, 200.0, 50.0, 200.0]);
    assert_eq!(trigger, [2, 2, 2, 2, 2, 2, 2, 3, 5, 5]);
    assert_eq!(timer, [180.0, 120.0, 120.0, 120.0, 90.0, 90.0, 90.0, 60.0, 45.0, 60.0]);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn all_heat_rows_preserve_context_composition_and_execution_tuning() {
    use nfsmw_data::pursuit::{PursuitTable, ground_cops};
    let dir = install().expect("set NFSMW_GAME_DIR to verify installed pursuit context rows");
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin");
    let expected = [
        (PursuitTable::OpenWorld, [4, 5, 7, 8, 10, 10, 6, 8, 4, 1]),
        (PursuitTable::Race, [2, 3, 4, 5, 5, 6, 8, 8, 8, 8]),
    ];
    for (table, counts) in expected {
        assert_selected_context_fields(&db, table);
        let waves = table.rows(&db).expect("all ten wave rows");
        let backups = table.backups(&db).expect("all ten backup rows");
        let formations = table.formations(&db).expect("all ten formation rows");
        let tuning = table.tuning(&db).expect("all ten tuning rows");
        let support = table.support(&db).expect("all ten support rows");
        assert_eq!(waves.len(), 10);
        assert_eq!(formations.len(), 10);
        assert_eq!(tuning.len(), 10);
        assert_eq!(support.len(), 10);
        assert_eq!(table.growth(&db).len(), 10);
        for (index, wave) in waves.iter().enumerate() {
            let count: u32 = ground_cops(wave).map(|cop| cop.count).sum();
            assert_eq!(count, counts[index], "{table:?} heat {}", index + 1);
            assert!(backups[index].engagement_radius > 0.0);
            assert!(tuning[index].front_los > 0.0 && tuning[index].evade_timeout > 0.0);
            assert!(support[index].minimum_delay >= 0.0);
            assert!(!formations[index].records.is_empty());
            if table == PursuitTable::Race {
                assert_eq!(formations[index].stagger_time, 10.0);
                assert_eq!(formations[index].collapse_speed, 15.0);
                assert_eq!(tuning[index].speed_reaction, 0.0);
                assert_eq!(
                    tuning[index].rear_los,
                    match index < 7 {
                        true => 51.0,
                        false => 101.0,
                    }
                );
                assert_eq!(tuning[index].heli_gap, 120.0);
            }
        }
    }
    let waves = PursuitTable::OpenWorld.rows(&db).unwrap();
    assert_ne!(
        waves[9].full_engagement_cops,
        ground_cops(&waves[9]).map(|cop| cop.count).sum::<u32>(),
        "the engagement budget is separate from wave composition and active cop cap"
    );
}

/// Report the exact context, heat and field before a table-level `Option` loses that information.
fn assert_selected_context_fields(db: &Database, table: nfsmw_data::pursuit::PursuitTable) {
    use blackbox_attrib::Value;
    use nfsmw_data::pursuit::PursuitTable;
    let escalation = db.collection("pursuitescalation", "default").expect("pursuitescalation/default");
    let (levels, support) = match table {
        PursuitTable::OpenWorld => ("heattable", "supporttable"),
        PursuitTable::Race => ("racetable", "supportracetable"),
    };
    let floats = [
        "StaggerFormationTime",
        "BoxinTightness",
        "BoxinDuration",
        "RollingBlockTightness",
        "RollingBlockDuration",
        "CollapseAggression",
        "CollapseSpeed",
        "CollapseOuterRadius",
        "frontLOSdistance",
        "rearLOSdistance",
        "heliLOSdistance",
        "SpeedReactionTime",
        "evadetimeout",
        "HiddenZoneTimeMultiplier",
        "HeliFuelTime",
        "TimeBetweenHeliActive",
        "SearchModeHeliSpawnChance",
        "roadblockprobability",
        "roadblockspikechance",
        "SearchModeRoadblockRadius",
        "SearchModeRoadblockChance",
        "SearchModeCityMPH",
        "SearchModeHwyMPH",
        "BustSpeed",
    ];
    for index in 0..10 {
        let row = db.resolve(escalation.get_at(levels, index).unwrap().as_ref_spec().unwrap()).unwrap();
        for field in floats {
            assert!(
                row.get_f32(field).is_some(),
                "{table:?} heat {} ({:08x}) {field}: expected Float, got {:?}",
                index + 1,
                row.key(),
                row.get(field)
            );
        }
        for field in ["CollapseInnerRadius", "MaxCopsCollapsing"] {
            assert!(
                row.get_i32(field).is_some(),
                "{table:?} heat {} ({:08x}) {field}: expected Int32, got {:?}",
                index + 1,
                row.key(),
                row.get(field)
            );
        }
        let formations = row.get("CopFormations").and_then(Value::as_array).expect("CopFormations array");
        assert!(!formations.is_empty(), "{table:?} heat {} has no formations", index + 1);
        for record in formations {
            assert!(
                matches!(record, Value::Raw { bytes, .. } if bytes.len() == 12),
                "{table:?} heat {} CopFormations: {record:?}",
                index + 1
            );
        }
        let row = db.resolve(escalation.get_at(support, index).unwrap().as_ref_spec().unwrap()).unwrap();
        assert!(
            row.get_f32("MinimumSupportDelay").is_some(),
            "{table:?} heat {} MinimumSupportDelay: {:?}",
            index + 1,
            row.get("MinimumSupportDelay")
        );
        for (field, width) in [("AirSupportOptions", 12), ("HeavySupportOptions", 16), ("LeaderSupportOptions", 20)] {
            if let Some(value) = row.get(field) {
                let records = value
                    .as_array()
                    .unwrap_or_else(|| panic!("{table:?} heat {} {field}: expected array, got {value:?}", index + 1));
                for record in records {
                    assert!(
                        matches!(record, Value::Raw { bytes, .. } if bytes.len() == width),
                        "{table:?} heat {} {field}: expected {width} bytes, got {record:?}",
                        index + 1
                    );
                }
            }
        }
    }
}
