use super::*;

const PROFILE: &str = r#"
format = 1
[[layer]]
texture = "TIRESMOKE/MW2012/PUFF.PNG"
blend = "alpha"
grid = 2
random_start = true
fps = 0
rate = 18.0
rate_per_speed = 2.0
life = 1.0
life_variance = 0.25
size = [0.5, 1.5, 2.5]
keys = [0.0, 0.15, 1.0]
colors = [[200, 205, 220, 0], [200, 205, 220, 80], [150, 155, 170, 0]]
[[layer]]
texture = "TIRESMOKE/MW2012/SMOKE.PNG"
"#;

#[test]
fn layers_parse_in_file_order_with_the_effect_defaults() {
    let profile = parse(PROFILE).unwrap();
    assert_eq!(profile.layers.len(), 2);
    let puff = &profile.layers[0];
    assert_eq!((puff.additive, puff.grid, puff.fps, puff.rate, puff.rate_per_speed), (false, 2, 0, 18.0, 2.0));
    assert_eq!((puff.life, puff.life_variance, puff.random_start), (1.0, 0.25, true));
    assert_eq!(puff.colors[1], [200, 205, 220, 80]);
    let plain = &profile.layers[1];
    assert!(plain.additive && plain.grid == 0, "the defaults of an effect");
    assert_eq!(plain.keys.len(), 4);
}

#[test]
fn a_profile_needs_the_format_and_a_layer() {
    let wrong = parse("format = 2\n[[layer]]\ntexture = \"A.PNG\"");
    assert!(matches!(wrong, Err(ContentError::Field { field: "format", .. })));
    assert!(matches!(parse("format = 1"), Err(ContentError::Field { field: "layer", .. })));
    assert!(matches!(parse("[[layer]]\ntexture = \"A.PNG\""), Err(ContentError::Parse(_))), "format is required");
}

#[test]
fn a_layer_has_no_moment_and_the_keys_are_checked() {
    let layer = |extra: &str| parse(&format!("format = 1\n[[layer]]\ntexture = \"A.PNG\"\n{extra}"));
    assert!(layer("").is_ok());
    assert!(matches!(layer("on = \"burn\""), Err(ContentError::Field { field: "layer.on", .. })));
    assert!(matches!(layer("rate = 9999.0"), Err(ContentError::Field { field: "effect.rate", .. })));
    assert!(matches!(layer("blend = \"x\""), Err(ContentError::Field { field: "effect.blend", .. })));
    assert!(matches!(layer("what = 1"), Err(ContentError::Parse(_))));
    let bad_texture = "format = 1\n[[layer]]\ntexture = \"../A.PNG\"";
    assert!(matches!(parse(bad_texture), Err(ContentError::Field { field: "effect.texture", .. })));
}

#[test]
fn at_most_eight_layers() {
    let layers = |n: usize| format!("format = 1\n{}", "[[layer]]\ntexture = \"A.PNG\"\n".repeat(n));
    assert!(parse(&layers(MAX_LAYERS)).is_ok());
    assert!(matches!(parse(&layers(MAX_LAYERS + 1)), Err(ContentError::Field { field: "layer", .. })));
}

#[test]
fn the_profile_path_is_the_tire_smoke_folder_of_the_value() {
    assert_eq!(tire_smoke_profile_path("mw2012"), "TIRESMOKE/MW2012/profile.toml");
    assert_eq!(TIRESMOKE_FOLDER, "TIRESMOKE");
}
