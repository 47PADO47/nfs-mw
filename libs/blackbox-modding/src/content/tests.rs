use super::*;

fn car(text: &str) -> Result<CarManifest, ContentError> {
    parse_car("fastcoupe", text)
}

#[test]
fn a_minimal_car_is_a_player_car_with_no_name_or_brand() {
    let manifest = car("").unwrap();
    assert_eq!(manifest.id, "FASTCOUPE");
    assert_eq!(manifest.role, Role::Player);
    assert!(manifest.heat.is_empty());
    assert_eq!((manifest.name, manifest.manufacturer), (None, None));
}

#[test]
fn name_and_brand_are_trimmed() {
    let manifest = car("name = \"  Fast Coupe \"\nmanufacturer = \"Acme\"\n").unwrap();
    assert_eq!(manifest.name.as_deref(), Some("Fast Coupe"));
    assert_eq!(manifest.manufacturer.as_deref(), Some("Acme"));
}

#[test]
fn a_cop_takes_the_heat_levels_sorted() {
    let manifest = car("role = \"cop\"\nheat = [4, 2, 3]\n").unwrap();
    assert_eq!(manifest.role, Role::Cop);
    assert!(!manifest.role.is_player());
    assert_eq!(manifest.heat, vec![2, 3, 4]);
}

#[test]
fn a_cop_without_a_heat_list_gets_an_empty_list_for_every_level() {
    let manifest = car("role = \"both\"\n").unwrap();
    assert!(manifest.role.is_player() && manifest.role.is_cop());
    assert!(manifest.heat.is_empty());
}

#[test]
fn heat_without_a_cop_role_is_refused() {
    assert_eq!(car("heat = [1]\n"), Err(ContentError::HeatWithoutCop));
}

#[test]
fn heat_levels_start_at_one_and_do_not_repeat() {
    assert_eq!(car("role = \"cop\"\nheat = [0, 1]\n"), Err(ContentError::InvalidHeat(0)));
    assert_eq!(car("role = \"cop\"\nheat = [2, 1, 2]\n"), Err(ContentError::RepeatedHeat(2)));
    assert_eq!(car("role = \"cop\"\nheat = []\n"), Err(ContentError::EmptyHeat));
}

#[test]
fn an_unknown_key_refuses_the_car_with_its_name() {
    let error = car("[override]\n\"engine.torque_scale\" = 1.1\n").unwrap_err();
    let ContentError::Parse(message) = error else { panic!("expected a parse error") };
    assert!(message.contains("override"), "{message}");
}

#[test]
fn an_unknown_role_is_refused() {
    assert!(matches!(car("role = \"racer\"\n"), Err(ContentError::Parse(_))));
}

#[test]
fn the_clone_key_is_gone_and_names_the_key_when_given() {
    let error = car("clone = \"CAMARO\"\n").unwrap_err();
    assert!(error.to_string().contains("clone"), "{error}");
}

#[test]
fn a_bad_file_name_is_refused() {
    assert_eq!(parse_car("fast.coupe", ""), Err(ContentError::InvalidId("fast.coupe".into())));
    let long = "x".repeat(33);
    assert_eq!(parse_car(&long, ""), Err(ContentError::InvalidId(long)));
}

#[test]
fn an_empty_or_control_text_field_is_refused() {
    assert_eq!(car("name = \"   \"\n"), Err(ContentError::InvalidName("   ".into())));
    assert_eq!(
        car("manufacturer = \"Ac\\tme\"\n"),
        Err(ContentError::InvalidManufacturer("Ac\tme".into()))
    );
}

#[test]
fn a_mod_without_a_music_table_has_no_music_section() {
    assert_eq!(parse_music("[mod]\nid = \"example.pack\"\nname = \"Pack\"\n"), Ok(None));
}

#[test]
fn the_music_table_defaults_to_every_context_and_not_always() {
    let section = parse_music("[mod]\nid = \"x\"\n\n[music]\n").unwrap().unwrap();
    assert_eq!(section, MusicSection { contexts: vec!["menu".into(), "free_roam".into()], always: false });
}

#[test]
fn the_music_table_takes_its_contexts_and_always() {
    let section = parse_music("[music]\ncontexts = [\"free_roam\"]\nalways = true\n").unwrap().unwrap();
    assert_eq!(section, MusicSection { contexts: vec!["free_roam".into()], always: true });
}

#[test]
fn bad_music_contexts_are_refused() {
    assert_eq!(parse_music("[music]\ncontexts = []\n"), Err(ContentError::NoMusicContext));
    assert_eq!(parse_music("[music]\ncontexts = [\"Menu\"]\n"), Err(ContentError::InvalidMusicContext("Menu".into())));
    assert_eq!(
        parse_music("[music]\ncontexts = [\"menu\", \"menu\"]\n"),
        Err(ContentError::RepeatedMusicContext("menu".into()))
    );
    assert!(matches!(parse_music("[music]\nlooping = true\n"), Err(ContentError::Parse(_))));
}

#[test]
fn a_car_without_a_stats_table_has_the_generic_stats() {
    let manifest = car("")
    .unwrap();
    assert_eq!(manifest.stats, Stats::default());
}

const STATS: &str = "[stats]
mass_kg = 1295.0
front_weight_pct = 38.4
wheelbase_m = 2.45
front_tire_radius_m = 0.34
idle_rpm = 1000.0
redline_rpm = 7800.0
torque_curve = [[1000.0, 360.0], [5600.0, 440.0], [7800.0, 390.0]]
gear_ratios = [3.91, 2.29, 1.55, 1.3, 1.08, 0.88, 0.71]
final_drive = 3.44
drive = \"rwd\"
top_speed_kph = 304.0
front_tire_grip = 1.5
front_brake_torque_nm = 1300.0
nitrous = true
nitrous_boost = 0.9
";

#[test]
fn the_stats_table_gives_absolute_values_and_keeps_the_generic_rest() {
    let stats = car(STATS).unwrap().stats;
    assert_eq!((stats.mass_kg, stats.front_weight_pct, stats.wheelbase_m), (1295.0, 38.4, 2.45));
    assert_eq!((stats.idle_rpm, stats.redline_rpm), (1000.0, 7800.0));
    assert_eq!(stats.torque_curve, vec![[1000.0, 360.0], [5600.0, 440.0], [7800.0, 390.0]]);
    assert_eq!(stats.gear_ratios.len(), 7);
    assert_eq!((stats.final_drive, stats.drive, stats.top_speed_kph), (3.44, Drive::Rear, Some(304.0)));
    assert_eq!((stats.tire_radius_m[0], stats.tire_grip[0], stats.brake_torque_nm[0]), (0.34, 1.5, 1300.0));
    assert!(stats.nitrous && stats.nitrous_boost == 0.9);
    // Keys left out are the generic car's.
    let generic = Stats::default();
    assert_eq!((stats.tire_radius_m[1], stats.tire_grip[1], stats.brake_torque_nm[1]), (0.33, 1.2, 950.0));
    assert_eq!(stats.damping_ratio, generic.damping_ratio);
}

#[test]
fn a_stats_number_outside_its_range_is_refused_with_its_name() {
    for (key, bad) in [("mass_kg", "5.0"), ("front_tire_grip", "9.0"), ("final_drive", "0.0"), ("idle_rpm", "-1.0")] {
        let error = car(&format!(
            "[stats]
{key} = {bad}
"
        ))
        .unwrap_err();
        assert!(matches!(&error, ContentError::Field { field, .. } if *field == format!("stats.{key}")), "{error}");
    }
}

#[test]
fn stats_that_contradict_each_other_are_refused() {
    let redline = car("[stats]
idle_rpm = 2000.0
redline_rpm = 1800.0
")
    .unwrap_err();
    assert!(matches!(redline, ContentError::Field { field: "stats.redline_rpm", .. }), "{redline}");
    let curve = car("[stats]
torque_curve = [[3000.0, 300.0], [2000.0, 320.0]]
")
    .unwrap_err();
    assert!(matches!(curve, ContentError::Field { field: "stats.torque_curve", .. }), "{curve}");
    let gears = car("[stats]
gear_ratios = [1.0, 2.0]
")
    .unwrap_err();
    assert!(matches!(gears, ContentError::Field { field: "stats.gear_ratios", .. }), "{gears}");
}

#[test]
fn an_unknown_stats_key_is_refused_and_named() {
    let error = car("[stats]
torque = 2.0
")
    .unwrap_err();
    assert!(error.to_string().contains("torque"), "{error}");
    // The old multipliers are gone: a car has no clone to be a multiple of.
    let error = car("[performance]
power = 1.1
")
    .unwrap_err();
    assert!(error.to_string().contains("performance"), "{error}");
}

const ENGINE: &str = "[engine_sound]\nidle_rpm = 820.0\nredline_rpm = 7500.0\nlayers = [\n  { file = \"SOUND/MODENGINE/MYCAR/HIGH.WAV\", rpm = 5000.0 },\n  { file = \"SOUND/MODENGINE/MYCAR/IDLE.WAV\", rpm = 820.0 },\n]\n";

#[test]
fn a_car_has_no_engine_sound_unless_it_gives_one() {
    assert_eq!(car("").unwrap().engine_sound, None);
}

#[test]
fn the_engine_sound_table_gives_layers_slowest_first() {
    let sound = car(ENGINE).unwrap().engine_sound.unwrap();
    assert_eq!((sound.idle_rpm, sound.redline_rpm), (820.0, 7500.0));
    assert_eq!(sound.layers.iter().map(|layer| layer.rpm).collect::<Vec<_>>(), [820.0, 5000.0]);
    assert_eq!(sound.layers[0].file, "SOUND/MODENGINE/MYCAR/IDLE.WAV");
}

#[test]
fn a_bad_engine_sound_is_refused() {
    for (edit, from, to) in [
        ("a redline under the idle", "redline_rpm = 7500.0", "redline_rpm = 500.0"),
        ("a path with ..", "SOUND/MODENGINE/MYCAR/IDLE.WAV", "SOUND/../IDLE.WAV"),
        ("a file that is not a WAV", "SOUND/MODENGINE/MYCAR/HIGH.WAV", "SOUND/MODENGINE/MYCAR/HIGH.MP3"),
        ("two layers at one rpm", "rpm = 5000.0", "rpm = 820.0"),
        ("an absolute path", "SOUND/MODENGINE/MYCAR/IDLE.WAV", "/SOUND/IDLE.WAV"),
    ] {
        assert!(car(&ENGINE.replace(from, to)).is_err(), "{edit}");
    }
    assert!(car("[engine_sound]\nidle_rpm = 800.0\nredline_rpm = 7000.0\nlayers = []\n").is_err());
}
