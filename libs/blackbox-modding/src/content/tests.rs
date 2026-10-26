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
fn a_car_without_a_performance_table_has_no_change() {
    let manifest = car("").unwrap();
    assert!(manifest.performance.is_stock());
}

#[test]
fn the_performance_table_sets_multipliers_and_factory_nitrous() {
    let manifest = car(
        "[performance]\npower = 1.1\nmass = 0.9\ngrip = 1.05\nbrakes = 1.2\nnitrous = true\nnitrous_power = 1.5\n",
    )
    .unwrap();
    let p = manifest.performance;
    assert_eq!((p.power, p.mass, p.grip, p.brakes, p.nitrous, p.nitrous_power), (1.1, 0.9, 1.05, 1.2, true, 1.5));
    assert!(!p.is_stock());
}

#[test]
fn a_performance_number_outside_its_range_is_refused_with_its_name() {
    let error = car("[performance]\npower = 9.0\n").unwrap_err();
    assert!(matches!(error, ContentError::Field { field: "performance.power", .. }), "{error}");
    let error = car("[performance]\nmass = 0.0\n").unwrap_err();
    assert!(matches!(error, ContentError::Field { field: "performance.mass", .. }), "{error}");
}

#[test]
fn an_unknown_performance_key_is_refused() {
    let error = car("[performance]\ntorque = 2.0\n").unwrap_err();
    assert!(error.to_string().contains("torque"), "{error}");
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
