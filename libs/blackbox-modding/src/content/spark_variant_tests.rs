use super::*;

const PROFILE: &str = r#"
format = 1

[hit]
closing_speed_full = 22.08
intensity_floor = 0.153

[[hit.layer]]
kind = "streak"
burst = 79.0
life = 0.85
life_variance = 0.7059
speed = 8.0
speed_variance = 0.5
spread = 10.0
gravity = 9.8
drag = 0.0851
bounce = [0.8, 0.9]
friction = [0.01, 0.05]
inherit = 1.0
inherit_variance = 1.0
stretch = 0.125
keys = [0.0, 0.33, 0.66, 0.99]
size = [0.08, 0.0184, 0.0118, 0.0052]
colors = [[255, 104, 39, 255], [255, 104, 39, 255], [255, 112, 36, 230], [255, 125, 28, 0]]
brightness = [12.0, 6.0, 6.0, 5.3]

[[hit.layer]]
texture = "SPARKS/MW2012/E140280_0.PNG"
grid = 4
fps = 30
burst = 3.0

[scrape]
speed_start = 0.73
speed_full = 1.37

[[scrape.layer]]
kind = "streak"
rate = 196.0
life = 0.4
"#;

#[test]
fn layers_parse_with_their_units_and_defaults() {
    let profile = parse(PROFILE).unwrap();
    assert_eq!((profile.hit.items.len(), profile.scrape.items.len()), (2, 1));
    let streak = &profile.hit.items[0];
    assert_eq!((streak.kind, streak.texture.as_deref(), streak.additive), (SparkKind::Streak, None, true));
    assert_eq!((streak.count, streak.life, streak.speed, streak.spread), (79.0, 0.85, 8.0, 10.0));
    assert_eq!((streak.gravity, streak.stretch), (9.8, 0.125));
    assert_eq!((streak.bounce, streak.friction), (Some([0.8, 0.9]), Some([0.01, 0.05])));
    assert_eq!(streak.keys.len(), 4);
    assert_eq!(streak.colors[3], [255, 125, 28, 0]);
    assert_eq!(streak.brightness, vec![12.0, 6.0, 6.0, 5.3]);
    let sprite = &profile.hit.items[1];
    assert_eq!((sprite.kind, sprite.grid, sprite.fps), (SparkKind::Sprite, 4, 30));
    assert_eq!(sprite.texture.as_deref(), Some("SPARKS/MW2012/E140280_0.PNG"));
    assert_eq!(sprite.keys.len(), 4, "the default keys");
    assert_eq!(sprite.brightness, vec![1.0; 4], "no overbright unless the profile says so");
    assert_eq!(sprite.bounce, None);
    let rate = &profile.scrape.items[0];
    assert_eq!((rate.count, rate.life), (196.0, 0.4));
}

#[test]
fn the_gates_come_with_the_channels() {
    let profile = parse(PROFILE).unwrap();
    let hit = profile.hit.gate.unwrap();
    assert_eq!((hit.closing_speed_full, hit.intensity_floor), (22.08, 0.153));
    let scrape = profile.scrape.gate.unwrap();
    assert_eq!((scrape.speed_start, scrape.speed_full), (0.73, 1.37));
    assert!((profile.hit.strength(22.08).unwrap() - 1.0).abs() < 1e-6);
    assert!((profile.hit.strength(0.0).unwrap() - 0.153).abs() < 1e-6);
    assert_eq!(profile.scrape.strength(0.5), Some(0.0));
}

#[test]
fn a_channel_with_no_layer_keeps_the_installs_sparks() {
    let only_hit = "format = 1\n[hit]\nclosing_speed_full = 10.0\nintensity_floor = 0.1\n[[hit.layer]]\nkind = \"streak\"\nburst = 5.0";
    let profile = parse(only_hit).unwrap();
    assert!(profile.scrape.items.is_empty() && profile.scrape.gate.is_none());
    assert_eq!(profile.scrape.strength(5.0), None);
    let gate_only = format!("{only_hit}\n[scrape]\nspeed_start = 1.0\nspeed_full = 2.0");
    let kept = parse(&gate_only).unwrap();
    assert!(kept.scrape.items.is_empty());
    assert_eq!(kept.scrape.strength(5.0), None, "a gate with nothing to play is no channel");
}

#[test]
fn a_profile_needs_the_format_and_a_layer() {
    assert!(matches!(parse("format = 2\n[hit]\n"), Err(ContentError::Field { field: "format", .. })));
    assert!(matches!(parse("format = 1"), Err(ContentError::Field { field: "layer", .. })));
    assert!(matches!(parse("[hit]\nclosing_speed_full = 1.0"), Err(ContentError::Parse(_))), "format is required");
    let empty = "format = 1\n[hit]\nclosing_speed_full = 10.0\nintensity_floor = 0.1";
    assert!(matches!(parse(empty), Err(ContentError::Field { field: "layer", .. })), "gates alone play nothing");
}

#[test]
fn a_channel_with_layers_needs_its_gate() {
    let layer = "[[hit.layer]]\nkind = \"streak\"\nburst = 5.0";
    let no_gate = format!("format = 1\n[hit]\n{layer}");
    assert!(matches!(parse(&no_gate), Err(ContentError::Field { field: "hit.closing_speed_full", .. })));
    let half = format!("format = 1\n[hit]\nclosing_speed_full = 5.0\n{layer}");
    assert!(matches!(parse(&half), Err(ContentError::Field { field: "hit.intensity_floor", .. })));
    let scrape = "format = 1\n[scrape]\nspeed_start = 1.0\n[[scrape.layer]]\nkind = \"streak\"\nrate = 5.0";
    assert!(matches!(parse(scrape), Err(ContentError::Field { field: "scrape.speed_full", .. })));
}

#[test]
fn an_impact_layer_has_a_burst_and_a_scrape_layer_a_rate() {
    let hit = |extra: &str| {
        let head =
            "format = 1\n[hit]\nclosing_speed_full = 5.0\nintensity_floor = 0.1\n[[hit.layer]]\nkind = \"streak\"\n";
        parse(&format!("{head}{extra}"))
    };
    assert!(hit("burst = 5.0").is_ok());
    assert!(matches!(hit("rate = 5.0"), Err(ContentError::Field { field: "layer.burst", .. })));
    assert!(
        matches!(hit("burst = 5.0\nrate = 5.0"), Err(ContentError::Field { field: "layer.burst", .. })),
        "not both"
    );
    assert!(matches!(hit(""), Err(ContentError::Field { field: "layer.burst", .. })));
    let scrape = |extra: &str| {
        let head = "format = 1\n[scrape]\nspeed_start = 1.0\nspeed_full = 2.0\n[[scrape.layer]]\nkind = \"streak\"\n";
        parse(&format!("{head}{extra}"))
    };
    assert!(scrape("rate = 50.0").is_ok());
    assert!(matches!(scrape("burst = 5.0"), Err(ContentError::Field { field: "layer.rate", .. })));
    assert!(matches!(scrape("rate = 5000.0"), Err(ContentError::Field { field: "layer.count", .. })));
}

#[test]
fn layer_numbers_are_checked() {
    let bad = |extra: &str| {
        let head = "format = 1\n[hit]\nclosing_speed_full = 5.0\nintensity_floor = 0.1\n[[hit.layer]]\nburst = 5.0\n";
        parse(&format!("{head}texture = \"A.PNG\"\n{extra}"))
    };
    assert!(bad("").is_ok());
    assert!(matches!(bad("kind = \"x\""), Err(ContentError::Field { field: "layer.kind", .. })));
    assert!(matches!(bad("blend = \"x\""), Err(ContentError::Field { field: "layer.blend", .. })));
    assert!(matches!(bad("grid = 3"), Err(ContentError::Field { field: "layer.grid", .. })));
    assert!(bad("grid = 8").is_ok());
    assert!(matches!(bad("fps = 61"), Err(ContentError::Field { field: "layer.fps", .. })));
    assert!(matches!(bad("life = 0.0"), Err(ContentError::Field { field: "layer.life", .. })));
    assert!(matches!(bad("speed = 500.0"), Err(ContentError::Field { field: "layer.speed", .. })));
    assert!(matches!(bad("spread = 190.0"), Err(ContentError::Field { field: "layer.spread", .. })));
    assert!(matches!(bad("gravity = 60.0"), Err(ContentError::Field { field: "layer.gravity", .. })));
    assert!(matches!(bad("drag = -1.0"), Err(ContentError::Field { field: "layer.drag", .. })));
    assert!(matches!(bad("inherit = 1.5"), Err(ContentError::Field { field: "layer.inherit", .. })));
    assert!(matches!(bad("stretch = 6.0"), Err(ContentError::Field { field: "layer.stretch", .. })));
    assert!(matches!(bad("bounce = [0.9, 0.8]"), Err(ContentError::Field { field: "layer.bounce", .. })));
    assert!(matches!(bad("friction = [0.0, 2.0]"), Err(ContentError::Field { field: "layer.friction", .. })));
    assert!(matches!(bad("size = [0.1, 0.1, 0.1, 11.0]"), Err(ContentError::Field { field: "layer.size", .. })));
    assert!(matches!(
        bad("brightness = [1.0, 65.0, 1.0, 1.0]"),
        Err(ContentError::Field { field: "layer.brightness", .. })
    ));
    assert!(matches!(bad("colors = [[1,2,3,4]]"), Err(ContentError::Field { field: "layer.colors", .. })));
    assert!(matches!(bad("what = 1"), Err(ContentError::Parse(_))));
}

#[test]
fn keys_must_rise_and_every_list_has_one_value_for_each() {
    let keyed = |keys: &str, size: &str| {
        let head = "format = 1\n[hit]\nclosing_speed_full = 5.0\nintensity_floor = 0.1\n[[hit.layer]]\nkind = \"streak\"\nburst = 5.0\n";
        parse(&format!("{head}keys = {keys}\nsize = {size}"))
    };
    assert!(keyed("[0.0, 0.5, 1.0]", "[0.1, 0.1, 0.1]").is_ok());
    assert!(keyed("[0.0, 1.0]", "[0.1, 0.1]").is_ok());
    assert!(matches!(
        keyed("[0.0, 0.5, 0.5]", "[0.1, 0.1, 0.1]"),
        Err(ContentError::Field { field: "layer.keys", .. })
    ));
    assert!(
        matches!(keyed("[0.5]", "[0.1]"), Err(ContentError::Field { field: "layer.keys", .. })),
        "one key is no track"
    );
    assert!(matches!(keyed("[0.0, 1.5]", "[0.1, 0.1]"), Err(ContentError::Field { field: "layer.keys", .. })));
    assert!(matches!(keyed("[0.0, 1.0]", "[0.1]"), Err(ContentError::Field { field: "layer.size", .. })));
}

#[test]
fn a_sprite_needs_a_png_in_the_mods_files() {
    let sprite = |extra: &str| {
        parse(&format!(
            "format = 1\n[hit]\nclosing_speed_full = 5.0\nintensity_floor = 0.1\n[[hit.layer]]\nburst = 1.0\n{extra}"
        ))
    };
    assert!(
        matches!(sprite(""), Err(ContentError::Field { field: "layer.texture", .. })),
        "a sprite without a picture"
    );
    assert!(sprite("texture = \"SPARKS/A/B.PNG\"").is_ok());
    assert!(matches!(sprite("texture = \"../B.PNG\""), Err(ContentError::Field { field: "layer.texture", .. })));
    assert!(matches!(sprite("texture = \"B.WAV\""), Err(ContentError::Field { field: "layer.texture", .. })));
    let streak = "format = 1\n[hit]\nclosing_speed_full = 5.0\nintensity_floor = 0.1\n[[hit.layer]]\nkind = \"streak\"\nburst = 1.0\ntexture = \"x y.PNG\"";
    assert!(
        matches!(parse(streak), Err(ContentError::Field { field: "layer.texture", .. })),
        "a streak's picture is checked too"
    );
}

#[test]
fn at_most_eight_layers_a_channel() {
    let layers = |n: usize| {
        let head = "format = 1\n[hit]\nclosing_speed_full = 5.0\nintensity_floor = 0.1\n";
        format!("{head}{}", "[[hit.layer]]\nkind = \"streak\"\nburst = 1.0\n".repeat(n))
    };
    assert!(parse(&layers(MAX_SPARK_LAYERS)).is_ok());
    assert!(matches!(parse(&layers(MAX_SPARK_LAYERS + 1)), Err(ContentError::Field { field: "layer", .. })));
}

#[test]
fn the_profile_path_is_the_sparks_folder_of_the_value() {
    assert_eq!(sparks_profile_path("mw2012"), "SPARKS/MW2012/profile.toml");
    assert_eq!(SPARKS_FOLDER, "SPARKS");
}
