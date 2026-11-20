use super::*;
use crate::settings::{Partial, Percent, Settings};

fn modded_settings() -> Settings {
    Settings {
        traffic_lights: true,
        traffic: 30,
        cop_share: Percent(10),
        physics_rate_hz: 144,
        game_speed: 2.0,
        no_rev_limiter: true,
        ..Settings::from(Partial::default())
    }
}

fn header(options: ParityOptions, settings: &Settings, ini: Vec<IniFile>, extra: ExtraOptions) -> RunHeader {
    let tuning = WorldTuning::from(settings);
    let testing = Overrides { police_off: true, heat: 4, ..Overrides::default() };
    let world = WorldState {
        track: "L2RA",
        player_car: Some("BMWM3GTR"),
        transmission: Some(Transmission::Manual),
        rules: SessionRules::default(),
        tuning: &tuning,
        traffic: TrafficSettings::from(settings),
        traffic_manual: Some(12),
        rain: (true, 0.5),
        testing: &testing,
    };
    let mods = ModsRecord { mode: "all", loaded: vec!["one.mod".into(), "two.mod".into()] };
    RunHeader::new(options, ini, extra, Some(mods), Some(world))
}

#[test]
fn the_header_records_the_run_and_the_world_as_it_runs() {
    let options = ParityOptions { seed: Some(42), stock_rules: false, traffic: None };
    let extra = ExtraOptions {
        heat_override: Some(true),
        heat_min: Some(3.0),
        speeding_limit_mps: Some(50.0),
        ..ExtraOptions::default()
    };
    let json = serde_json::to_value(header(options, &modded_settings(), Vec::new(), extra)).unwrap();
    assert_eq!((&json["seed"], &json["stock_rules"]), (&serde_json::json!(42), &serde_json::json!(false)));
    assert_eq!(json["mods"], serde_json::json!({"mode": "all", "loaded": ["one.mod", "two.mod"]}));
    assert_eq!(json["extra_options"]["heat_bounds"], serde_json::json!([3.0, 10.0]));
    assert_eq!(json["extra_options"]["speeding_limit_mps"], 50.0);
    assert_eq!(json["extra_options"]["reckless_limit_mps"], serde_json::Value::Null);
    let world = &json["world"];
    assert_eq!((&world["track"], &world["player_car"]), (&serde_json::json!("L2RA"), &serde_json::json!("BMWM3GTR")));
    assert_eq!(world["transmission"], "manual");
    assert_eq!((&world["physics_rate_hz"], &world["game_speed"]), (&serde_json::json!(144), &serde_json::json!(2.0)));
    assert_eq!(world["animation_speed"], 1.0);
    assert_eq!(world["no_rev_limiter"], true);
    assert_eq!(world["car_scale"], 1.0);
    assert_eq!((&world["always_rain"], &world["rain_amount"]), (&serde_json::json!(true), &serde_json::json!(0.5)));
    let traffic = &world["traffic"];
    assert_eq!((&traffic["cars"], &traffic["cop_share"]), (&serde_json::json!(30), &serde_json::json!(10)));
    assert_eq!((&traffic["manual"], &traffic["lights"]), (&serde_json::json!(12), &serde_json::json!(true)));
    assert_eq!(traffic["policy"]["spawn_min"], 300);
    assert_eq!(traffic["policy"]["keep_on_minimap"], false);
    assert_eq!(world["pursuit_options"]["bounty_fixes"], true);
    assert_eq!(world["pursuit_options"]["enhanced"], false);
    assert_eq!(world["game_moments"]["camera"], true);
    assert_eq!(world["speedbreaker"]["dilation"]["target"], 0.25);
    assert_eq!(world["session_rules"]["views"], "local_camera");
    assert_eq!(world["session_rules"]["time_scale_allowed"], true);
    assert_eq!(world["session_rules"]["seed"], serde_json::Value::Null);
    assert_eq!(
        (&world["testing"]["police_off"], &world["testing"]["heat"]),
        (&serde_json::json!(true), &serde_json::json!(4))
    );
}

#[test]
fn every_setting_of_the_stock_list_is_in_the_header() {
    let mut forced = modded_settings();
    stock::force_settings(&mut forced);
    let options = ParityOptions { seed: None, stock_rules: true, traffic: None };
    let json = serde_json::to_value(header(options, &forced, Vec::new(), ExtraOptions::default())).unwrap();
    assert_eq!((&json["seed"], &json["stock_rules"]), (&serde_json::Value::Null, &serde_json::json!(true)));
    let world = json["world"].as_object().expect("the world's part");
    for key in [
        "physics_rate_hz",
        "game_speed",
        "animation_speed",
        "no_rev_limiter",
        "car_scale",
        "rain_amount",
        "traffic",
        "pursuit_options",
        "game_moments",
        "speedbreaker",
        "testing",
    ] {
        assert!(world.contains_key(key), "{key}");
    }
    let world = &json["world"];
    assert_eq!((&world["physics_rate_hz"], &world["game_speed"]), (&serde_json::json!(60), &serde_json::json!(1.0)));
    assert_eq!(
        (&world["traffic"]["lights"], &world["traffic"]["cars"]),
        (&serde_json::json!(false), &serde_json::json!(20))
    );
    assert_eq!(world["traffic"]["cop_share"], 10, "not forced: the original's share is not stated");
    assert_eq!(world["pursuit_options"]["bounty_fixes"], false);
    assert_eq!(world["no_rev_limiter"], false);
}

#[test]
fn a_trace_outside_a_world_has_no_world_part() {
    let header = RunHeader::new(ParityOptions::default(), Vec::new(), ExtraOptions::default(), None, None);
    let json = serde_json::to_value(header).unwrap();
    assert!(json.get("world").is_none());
    assert_eq!(json["mods"], serde_json::Value::Null);
    assert_eq!(json["extra_options"]["heat_bounds"], serde_json::Value::Null);
}

#[test]
fn the_ini_files_are_listed_as_present_and_applied() {
    let dir = std::env::temp_dir().join(format!("nfsmw-run-header-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("NFSMWBartenderSettings.ini"), "[General]\n").unwrap();
    let find = |files: &[IniFile], name: &str| files.iter().find(|file| file.name == name).cloned().unwrap();
    let read = ini_files(Some(&dir), false);
    assert_eq!(read.len(), 5);
    let bartender = find(&read, "NFSMWBartenderSettings.ini");
    assert!(bartender.present && bartender.applied);
    let helicopter = find(&read, "NFSMWHelicopterSettings.ini");
    assert!(!helicopter.present && !helicopter.applied);
    let stock_run = find(&ini_files(Some(&dir), true), "NFSMWBartenderSettings.ini");
    assert!(stock_run.present && !stock_run.applied, "a stock run sees the file and does not read it");
    assert!(ini_files(None, false).iter().all(|file| !file.present && !file.applied));
    std::fs::remove_dir_all(dir).unwrap();
}
