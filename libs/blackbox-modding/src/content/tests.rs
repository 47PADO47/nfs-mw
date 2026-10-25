use super::*;

fn car(text: &str) -> Result<CarManifest, ContentError> {
    parse_car("fastcoupe", text)
}

#[test]
fn a_minimal_car_is_a_player_car_that_keeps_the_clone_name() {
    let manifest = car("clone = \"BMWM3GTR\"\n").unwrap();
    assert_eq!(manifest.id, "FASTCOUPE");
    assert_eq!(manifest.clone, "BMWM3GTR");
    assert_eq!(manifest.role, Role::Player);
    assert!(manifest.heat.is_empty());
    assert_eq!((manifest.name, manifest.manufacturer), (None, None));
}

#[test]
fn name_and_brand_are_trimmed() {
    let manifest = car("clone = \"CORVETTE\"\nname = \"  Fast Coupe \"\nmanufacturer = \"Acme\"\n").unwrap();
    assert_eq!(manifest.name.as_deref(), Some("Fast Coupe"));
    assert_eq!(manifest.manufacturer.as_deref(), Some("Acme"));
}

#[test]
fn a_cop_takes_the_heat_levels_sorted() {
    let manifest = car("clone = \"CAMARO\"\nrole = \"cop\"\nheat = [4, 2, 3]\n").unwrap();
    assert_eq!(manifest.role, Role::Cop);
    assert!(!manifest.role.is_player());
    assert_eq!(manifest.heat, vec![2, 3, 4]);
}

#[test]
fn a_cop_without_a_heat_list_gets_an_empty_list_for_every_level() {
    let manifest = car("clone = \"CAMARO\"\nrole = \"both\"\n").unwrap();
    assert!(manifest.role.is_player() && manifest.role.is_cop());
    assert!(manifest.heat.is_empty());
}

#[test]
fn heat_without_a_cop_role_is_refused() {
    assert_eq!(car("clone = \"CAMARO\"\nheat = [1]\n"), Err(ContentError::HeatWithoutCop));
}

#[test]
fn heat_levels_start_at_one_and_do_not_repeat() {
    assert_eq!(car("clone = \"CAMARO\"\nrole = \"cop\"\nheat = [0, 1]\n"), Err(ContentError::InvalidHeat(0)));
    assert_eq!(car("clone = \"CAMARO\"\nrole = \"cop\"\nheat = [2, 1, 2]\n"), Err(ContentError::RepeatedHeat(2)));
    assert_eq!(car("clone = \"CAMARO\"\nrole = \"cop\"\nheat = []\n"), Err(ContentError::EmptyHeat));
}

#[test]
fn an_unknown_key_refuses_the_car_with_its_name() {
    let error = car("clone = \"CAMARO\"\n[override]\n\"engine.torque_scale\" = 1.1\n").unwrap_err();
    let ContentError::Parse(message) = error else { panic!("expected a parse error") };
    assert!(message.contains("override"), "{message}");
}

#[test]
fn an_unknown_role_is_refused() {
    assert!(matches!(car("clone = \"CAMARO\"\nrole = \"racer\"\n"), Err(ContentError::Parse(_))));
}

#[test]
fn a_missing_clone_is_refused() {
    assert!(matches!(car("name = \"Fast\"\n"), Err(ContentError::Parse(_))));
}

#[test]
fn a_bad_clone_or_file_name_is_refused() {
    assert_eq!(car("clone = \"BMW M3\"\n"), Err(ContentError::InvalidClone("BMW M3".into())));
    assert_eq!(car("clone = \"\"\n"), Err(ContentError::InvalidClone(String::new())));
    assert_eq!(parse_car("fast.coupe", "clone = \"CAMARO\"\n"), Err(ContentError::InvalidId("fast.coupe".into())));
    let long = "x".repeat(33);
    assert_eq!(parse_car(&long, "clone = \"CAMARO\"\n"), Err(ContentError::InvalidId(long)));
}

#[test]
fn an_empty_or_control_text_field_is_refused() {
    assert_eq!(car("clone = \"CAMARO\"\nname = \"   \"\n"), Err(ContentError::InvalidName("   ".into())));
    assert_eq!(
        car("clone = \"CAMARO\"\nmanufacturer = \"Ac\\tme\"\n"),
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
fn a_car_without_a_performance_table_drives_like_its_clone() {
    let manifest = car("clone = \"BMWM3GTR\"\n").unwrap();
    assert!(manifest.performance.is_stock());
}

#[test]
fn the_performance_table_sets_multipliers_and_factory_nitrous() {
    let manifest = car(
        "clone = \"BMWM3GTR\"\n[performance]\npower = 1.1\nmass = 0.9\ngrip = 1.05\nbrakes = 1.2\nnitrous = true\nnitrous_power = 1.5\n",
    )
    .unwrap();
    let p = manifest.performance;
    assert_eq!((p.power, p.mass, p.grip, p.brakes, p.nitrous, p.nitrous_power), (1.1, 0.9, 1.05, 1.2, true, 1.5));
    assert!(!p.is_stock());
}

#[test]
fn a_performance_number_outside_its_range_is_refused_with_its_name() {
    let error = car("clone = \"BMWM3GTR\"\n[performance]\npower = 9.0\n").unwrap_err();
    assert!(matches!(error, ContentError::Field { field: "performance.power", .. }), "{error}");
    let error = car("clone = \"BMWM3GTR\"\n[performance]\nmass = 0.0\n").unwrap_err();
    assert!(matches!(error, ContentError::Field { field: "performance.mass", .. }), "{error}");
}

#[test]
fn an_unknown_performance_key_is_refused() {
    let error = car("clone = \"BMWM3GTR\"\n[performance]\ntorque = 2.0\n").unwrap_err();
    assert!(error.to_string().contains("torque"), "{error}");
}
