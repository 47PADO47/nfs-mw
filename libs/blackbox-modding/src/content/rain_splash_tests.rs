use super::*;

const PROFILE: &str = r#"
format = 1
radius = 12.0
density = 1.5
[[layer]]
texture = "RAINSPLASH/MW2012/RING.PNG"
rate = 100.0
life = 0.6
size = [0.1, 0.8]
opacity = 0.5
color = [210, 220, 235]
[[layer]]
texture = "RAINSPLASH/MW2012/BURST.PNG"
kind = "burst"
blend = "additive"
grid = 4
"#;

#[test]
fn layers_parse_in_file_order_with_defaults() {
    let profile = parse(PROFILE).unwrap();
    assert_eq!((profile.radius, profile.density, profile.layers.len()), (12.0, 1.5, 2));
    let ring = &profile.layers[0];
    assert_eq!((ring.kind, ring.additive, ring.grid, ring.rate), (SplashKind::Ring, false, 1, 100.0));
    assert_eq!((ring.size, ring.color, ring.life_variance), ([0.1, 0.8], [210, 220, 235], 0.3));
    let burst = &profile.layers[1];
    assert_eq!((burst.kind, burst.additive, burst.grid, burst.color), (SplashKind::Burst, true, 4, [255; 3]));
}

#[test]
fn a_profile_needs_the_format_and_a_layer() {
    assert!(matches!(
        parse("format = 2\n[[layer]]\ntexture = \"A.PNG\""),
        Err(ContentError::Field { field: "format", .. })
    ));
    assert!(matches!(parse("format = 1"), Err(ContentError::Field { field: "layer", .. })));
    assert!(matches!(parse("[[layer]]\ntexture = \"A.PNG\""), Err(ContentError::Parse(_))));
}

#[test]
fn numbers_paths_and_counts_are_checked() {
    let layer = |extra: &str| parse(&format!("format = 1\n[[layer]]\ntexture = \"A.PNG\"\n{extra}"));
    assert!(layer("").is_ok());
    assert!(matches!(layer("rate = 5000.0"), Err(ContentError::Field { field: "layer.rate", .. })));
    assert!(matches!(layer("life = 0.0"), Err(ContentError::Field { field: "layer.life", .. })));
    assert!(matches!(layer("grid = 9"), Err(ContentError::Field { field: "layer.grid", .. })));
    assert!(matches!(layer("blend = \"x\""), Err(ContentError::Field { field: "layer.blend", .. })));
    assert!(matches!(layer("size = [0.0, 1.0]"), Err(ContentError::Field { field: "layer.size", .. })));
    assert!(matches!(layer("kind = \"x\""), Err(ContentError::Parse(_))));
    assert!(matches!(layer("what = 1"), Err(ContentError::Parse(_))));
    assert!(matches!(
        parse("format = 1\nradius = 99.0\n[[layer]]\ntexture = \"A.PNG\""),
        Err(ContentError::Field { field: "radius", .. })
    ));
    let bad = "format = 1\n[[layer]]\ntexture = \"../A.PNG\"";
    assert!(matches!(parse(bad), Err(ContentError::Field { field: "layer.texture", .. })));
}

#[test]
fn at_most_four_layers() {
    let layers = |n: usize| format!("format = 1\n{}", "[[layer]]\ntexture = \"A.PNG\"\n".repeat(n));
    assert!(parse(&layers(MAX_LAYERS)).is_ok());
    assert!(matches!(parse(&layers(MAX_LAYERS + 1)), Err(ContentError::Field { field: "layer", .. })));
}

#[test]
fn the_profile_path_is_the_rain_splash_folder_of_the_value() {
    assert_eq!(rain_splash_profile_path("mw2012"), "RAINSPLASH/MW2012/profile.toml");
}
