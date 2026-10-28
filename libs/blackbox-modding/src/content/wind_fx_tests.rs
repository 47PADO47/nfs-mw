use super::*;

const PROFILE: &str = r#"
format = 1
volume = 0.8

[[layer]]
file = "WIND/MW2012/W182159.WAV"
volume = 0.25
rate = 1.3
gain = [[0.0, 0.0], [45.0, 0.0], [65.0, 1.0], [90.0, 0.0]]

[[layer]]
file = "WIND/MW2012/W182182.WAV"
gain = [[0.0, 1.0], [100.0, 1.0]]
"#;

#[test]
fn layers_parse_in_file_order_with_their_defaults() {
    let profile = parse(PROFILE).unwrap();
    assert_eq!((profile.volume, profile.layers.len()), (0.8, 2));
    let first = &profile.layers[0];
    assert_eq!((first.file.as_str(), first.volume, first.rate), ("WIND/MW2012/W182159.WAV", 0.25, 1.3));
    assert_eq!(first.gain[2], (65.0, 1.0));
    let second = &profile.layers[1];
    assert_eq!((second.volume, second.rate), (1.0, 1.0), "the defaults");
    assert_eq!(parse("format = 1\n[[layer]]\nfile = \"A.WAV\"\ngain = [[0.0, 1.0], [1.0, 1.0]]").unwrap().volume, 1.0);
}

#[test]
fn the_level_follows_the_curve_and_stays_flat_outside_it() {
    let layer = &parse(PROFILE).unwrap().layers[0];
    assert_eq!(layer.level_at(10.0), 0.0);
    assert!((layer.level_at(55.0) - 0.125).abs() < 1e-6, "half way up from 45 to 65 mph, times the layer's 0.25");
    assert_eq!(layer.level_at(65.0), 0.25);
    assert_eq!(layer.level_at(200.0), 0.0, "flat after the last point");
    assert_eq!(WindLayer { file: String::new(), volume: 1.0, rate: 1.0, gain: Vec::new() }.level_at(5.0), 0.0);
}

#[test]
fn the_profile_lives_in_the_wind_folder() {
    assert_eq!(wind_profile_path("mw2012"), "WIND/MW2012/profile.toml");
    assert_eq!(WIND_FOLDER, "WIND");
}

#[test]
fn a_profile_needs_the_format_and_a_layer() {
    let wrong = parse("format = 2\n[[layer]]\nfile = \"A.WAV\"\ngain = [[0.0, 1.0], [1.0, 1.0]]");
    assert!(matches!(wrong, Err(ContentError::Field { field: "format", .. })));
    assert!(matches!(parse("format = 1"), Err(ContentError::Field { field: "layer", .. })));
    assert!(matches!(
        parse("[[layer]]\nfile = \"A.WAV\"\ngain = [[0.0, 1.0], [1.0, 1.0]]"),
        Err(ContentError::Parse(_))
    ));
}

#[test]
fn bad_layers_are_refused() {
    let layer =
        |extra: &str| format!("format = 1\n[[layer]]\nfile = \"A.WAV\"\ngain = [[0.0, 1.0], [1.0, 1.0]]\n{extra}");
    assert!(parse(&layer("")).is_ok());
    for (what, text) in [
        ("a rate under 0.25", layer("rate = 0.1")),
        ("a rate over 4", layer("rate = 5.0")),
        ("a volume over 8", layer("volume = 9.0")),
        ("a key the schema does not know", layer("pan = 1.0")),
    ] {
        assert!(parse(&text).is_err(), "{what}");
    }
    for (what, file, gain) in [
        ("a path that is not a WAV", "A.MP3", "[[0.0, 1.0], [1.0, 1.0]]"),
        ("a path with ..", "../A.WAV", "[[0.0, 1.0], [1.0, 1.0]]"),
        ("one point", "A.WAV", "[[0.0, 1.0]]"),
        ("speeds that do not rise", "A.WAV", "[[5.0, 1.0], [5.0, 1.0]]"),
        ("a level over 8", "A.WAV", "[[0.0, 9.0], [1.0, 1.0]]"),
        ("a negative level", "A.WAV", "[[0.0, -1.0], [1.0, 1.0]]"),
    ] {
        let text = format!("format = 1\n[[layer]]\nfile = \"{file}\"\ngain = {gain}\n");
        assert!(parse(&text).is_err(), "{what}");
    }
    assert!(parse("format = 1\nvolume = 9.0\n[[layer]]\nfile = \"A.WAV\"\ngain = [[0.0, 1.0], [1.0, 1.0]]").is_err());
}

#[test]
fn seventeen_layers_are_too_many() {
    let one = "[[layer]]\nfile = \"A.WAV\"\ngain = [[0.0, 1.0], [1.0, 1.0]]\n";
    assert!(parse(&format!("format = 1\n{}", one.repeat(16))).is_ok());
    assert!(parse(&format!("format = 1\n{}", one.repeat(17))).is_err());
}
