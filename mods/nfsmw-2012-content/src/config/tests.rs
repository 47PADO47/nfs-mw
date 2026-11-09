use super::*;

#[test]
fn the_default_settings_file_parses_to_the_defaults() {
    assert_eq!(Config::parse(DEFAULT_CONFIG).unwrap(), Config::default());
}

#[test]
fn defaults_want_everything() {
    let config = Config::default();
    assert!(config.wants("POR911CARR12", "Porsche 911 Carrera S 2012"));
}

#[test]
fn the_signs_are_on_by_default_and_the_setting_turns_them_off() {
    assert!(Config::default().signs);
    assert!(
        !Config::parse(
            "signs = false
"
        )
        .unwrap()
        .signs
    );
}

#[test]
fn the_bushes_are_on_by_default_and_a_change_converts_again() {
    assert!(Config::default().bushes);
    assert!(
        !Config::parse(
            "bushes = false
"
        )
        .unwrap()
        .bushes
    );
    assert_ne!(
        crate::stamp::of(&Config::default(), Path::new("definitely/not/here")),
        crate::stamp::of(&Config { bushes: false, ..Config::default() }, Path::new("definitely/not/here")),
    );
}

#[test]
fn the_trees_are_on_by_default_and_the_setting_turns_them_off() {
    assert!(Config::default().trees);
    assert!(!Config::parse("trees = false\n").unwrap().trees);
    assert_ne!(
        crate::stamp::of(&Config::default(), Path::new("definitely/not/here")),
        crate::stamp::of(&Config { trees: false, ..Config::default() }, Path::new("definitely/not/here")),
        "a change of the setting converts again"
    );
}

#[test]
fn the_sky_is_on_at_full_size_by_default_and_its_settings_are_checked() {
    let config = Config::default();
    assert!(
        config.sky
            && config.sky_size == 2048
            && !Config::parse(
                "sky = false
"
            )
            .unwrap()
            .sky
    );
    assert_eq!(
        Config::parse(
            "sky_size = 1024
"
        )
        .unwrap()
        .sky_size,
        1024
    );
    assert!(["sky_size = 100", "sky_size = 4096", "sky_size = 128"].into_iter().all(|bad| Config::parse(bad).is_err()));
    let stamp = |config: Config| crate::stamp::of(&config, Path::new("definitely/not/here"));
    assert_ne!(stamp(Config::default()), stamp(Config { sky: false, ..Config::default() }));
    assert_ne!(stamp(Config::default()), stamp(Config { sky_size: 512, ..Config::default() }));
}

#[test]
fn selection_and_exclusion_match_ids_and_name_fragments() {
    let config: Config = toml::from_str("cars = ['porsche', 'LAM']\nexclude = ['GT2']").unwrap();
    assert!(config.wants("POR911CARR12", "Porsche 911 Carrera S 2012"));
    assert!(config.wants("LAMGALLARDO", "Lamborghini Gallardo 2011"));
    assert!(!config.wants("POR911GT208", "Porsche 911 GT2 2008"));
    assert!(!config.wants("BMW1M12", "BMW 1M 2012"));
}

fn supplied(text: &str, has_file: bool, has_main_section: bool) -> Supplied {
    Supplied { text: text.to_owned(), has_file, has_main_section }
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-config-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_hosts_merged_text_is_the_settings() {
    let dir = scratch("merged");
    let path = dir.join(FILE_NAME);
    let (config, created) =
        Config::resolve(Some(&supplied("role = 'cop'\nmax_texture_size = 512\n", false, true)), &path).unwrap();
    assert_eq!((config.role, config.max_texture_size, created), (Role::Cop, 512, false));
    assert!(!path.exists(), "a table in the main config and no file: no template is written");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn with_no_settings_anywhere_the_template_is_written_and_means_the_defaults() {
    let dir = scratch("template");
    let path = dir.join(FILE_NAME);
    let (config, created) = Config::resolve(Some(&supplied("", false, false)), &path).unwrap();
    assert_eq!((config, created), (Config::default(), true));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), DEFAULT_CONFIG);
    let (_, again) = Config::resolve(Some(&supplied("", true, false)), &path).unwrap();
    assert!(!again, "once the file exists nothing is written");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_host_without_the_interface_gets_the_file_alone() {
    let dir = scratch("legacy");
    let path = dir.join(FILE_NAME);
    let (config, created) = Config::resolve(None, &path).unwrap();
    assert_eq!((config, created), (Config::default(), true));
    std::fs::write(&path, "role = 'both'\n").unwrap();
    let (config, created) = Config::resolve(None, &path).unwrap();
    assert_eq!((config.role, created), (Role::Both, false));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_unknown_key_in_either_source_is_an_error() {
    let dir = scratch("typo");
    let path = dir.join(FILE_NAME);
    let error = Config::resolve(Some(&supplied("typo = 1\n", false, true)), &path).unwrap_err();
    assert!(format!("{error:#}").contains("typo"), "{error:#}");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_change_in_either_source_changes_the_import_stamp() {
    use blackbox_modding::host::config::effective;
    let nowhere = Path::new("definitely/not/here");
    let stamp = |file: &str, main: &str| {
        let merged = effective(Some(file), Some(main)).unwrap();
        crate::stamp::of(&Config::parse(&merged.text).unwrap(), nowhere)
    };
    let base = stamp("role = 'player'\n", "max_texture_size = 1024\n");
    assert_ne!(base, stamp("role = 'cop'\n", "max_texture_size = 1024\n"), "the file changed");
    assert_ne!(base, stamp("role = 'player'\n", "max_texture_size = 512\n"), "the main config changed");
    assert_eq!(base, stamp("role = 'player'\n", "max_texture_size = 1024\n"), "nothing changed");
    assert_eq!(
        base,
        stamp("role = 'player'\nmax_texture_size = 1024\n", "role = 'player'\n"),
        "the same effective settings from other sources are the same stamp"
    );
}

#[test]
fn bad_settings_are_refused() {
    assert!(toml::from_str::<Config>("cars = 'some'").is_err());
    assert!(toml::from_str::<Config>("typo = 1").is_err());
    let config: Config = toml::from_str("max_texture_size = 1000").unwrap();
    assert!(config.check().is_err());
}

#[test]
fn the_drivers_are_posed_by_default_and_the_setting_turns_the_poses_off() {
    assert!(Config::default().driver_poses);
    assert!(!Config::parse("driver_poses = false\n").unwrap().driver_poses);
}

#[test]
fn the_cars_stream_by_default_and_the_setting_and_the_decision_change_the_stamp() {
    assert!(Config::default().stream_cars && !Config::default().lazy);
    assert!(!Config::parse("stream_cars = false\n").unwrap().stream_cars);
    assert!(toml::from_str::<Config>("lazy = true").is_err(), "the decision is not a setting");
    let nowhere = Path::new("definitely/not/here");
    let lazy = Config { lazy: true, ..Config::default() };
    assert_ne!(
        crate::stamp::of(&Config::default(), nowhere),
        crate::stamp::of(&lazy, nowhere),
        "stubs and files are different content"
    );
}

#[test]
fn the_effect_variants_are_on_by_default_and_each_setting_turns_one_off() {
    let config = Config::default();
    assert!(config.skid_marks && config.rain_splash && config.rain_drops && config.wet_road);
    let off = Config::parse("skid_marks = false\nrain_splash = false\nwet_road = false\n").unwrap();
    assert!(!off.skid_marks && !off.rain_splash && !off.wet_road && off.rain_drops);
    assert!(!off.features().skid_marks && off.features().trees);
    let here = Path::new("definitely/not/here");
    assert_ne!(
        crate::stamp::of(&config, here),
        crate::stamp::of(&Config { rain_drops: false, ..config.clone() }, here)
    );
}
