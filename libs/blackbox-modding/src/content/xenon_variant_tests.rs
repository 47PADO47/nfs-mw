use super::*;

const PROFILE: &str = r#"
format = 1

[hit]
closing_speed_full = 22.08
intensity_floor = 0.153

[[hit.spark]]
color = [1.0, 0.407, 0.153, 1.0]
life = 0.55
life_variance = 0.0
count = 79.0
count_variance = 0.0
center = [0.0, 0.0, 0.0]
extent = [0.5, 0.5, 0.0]
velocity = [0.53, 0.53, 6.0]
variation = [1.0, 1.0, 0.333]
inherit = [0.55, 0.55, 0.0]
gravity = -4.9
gravity_delta = 0.0
length = 255.0
length_delta = 0.0
height = 59.0
elasticity = 217.0

[[hit.spark]]
color = [1.0, 0.8, 0.5, 1.0]
life = 0.3
count = 10.0

[scrape]
speed_start = 0.73
speed_full = 1.37

[[scrape.spark]]
color = [1.0, 0.5, 0.2, 1.0]
life = 0.4
count = 196.0
"#;

#[test]
fn sparks_parse_in_the_restorations_units_with_defaults() {
    let profile = parse(PROFILE).unwrap();
    assert_eq!((profile.hit.items.len(), profile.scrape.items.len()), (2, 1));
    let first = &profile.hit.items[0];
    assert_eq!((first.life, first.count, first.gravity, first.height), (0.55, 79.0, -4.9, 59.0));
    assert_eq!((first.velocity, first.variation), ([0.53, 0.53, 6.0], [1.0, 1.0, 0.333]));
    assert_eq!((first.length, first.elasticity), (255.0, Some(217.0)));
    let second = &profile.hit.items[1];
    assert_eq!((second.velocity, second.extent, second.center), ([0.0; 3], [0.0; 3], [0.0; 3]));
    assert_eq!((second.life_variance, second.elasticity), (0.0, None), "the style's own elasticity stays");
    assert_eq!(profile.scrape.items[0].count, 196.0);
}

#[test]
fn the_gates_come_with_the_channels() {
    let profile = parse(PROFILE).unwrap();
    assert!((profile.hit.strength(22.08).unwrap() - 1.0).abs() < 1e-6);
    assert!((profile.hit.strength(0.0).unwrap() - 0.153).abs() < 1e-6);
    assert_eq!(profile.scrape.strength(0.0), Some(0.0));
    assert_eq!(profile.scrape.strength(5.0), Some(1.0));
}

#[test]
fn a_profile_needs_the_format_a_spark_and_the_gate_of_a_channel_with_sparks() {
    assert!(matches!(parse("format = 2\n[hit]\n"), Err(ContentError::Field { field: "format", .. })));
    assert!(matches!(parse("format = 1"), Err(ContentError::Field { field: "spark", .. })));
    let spark = "[[hit.spark]]\ncolor = [1.0, 1.0, 1.0, 1.0]\nlife = 0.5\ncount = 5.0";
    assert!(matches!(
        parse(&format!("format = 1\n[hit]\n{spark}")),
        Err(ContentError::Field { field: "hit.closing_speed_full", .. })
    ));
    let gated = format!("format = 1\n[hit]\nclosing_speed_full = 10.0\nintensity_floor = 0.2\n{spark}");
    assert!(parse(&gated).is_ok());
    let only_scrape = "format = 1\n[scrape]\nspeed_start = 1.0\nspeed_full = 2.0\n[[scrape.spark]]\ncolor = [1.0, 1.0, 1.0, 1.0]\nlife = 0.5\ncount = 5.0";
    let profile = parse(only_scrape).unwrap();
    assert!(profile.hit.items.is_empty());
    assert_eq!(profile.hit.strength(10.0), None, "the install's impact sparks stay");
}

#[test]
fn a_spark_needs_a_colour_a_life_and_a_count() {
    let head = "format = 1\n[hit]\nclosing_speed_full = 10.0\nintensity_floor = 0.2\n[[hit.spark]]\n";
    assert!(matches!(parse(&format!("{head}life = 0.5\ncount = 5.0")), Err(ContentError::Parse(_))));
    assert!(matches!(parse(&format!("{head}color = [1.0, 1.0, 1.0, 1.0]\ncount = 5.0")), Err(ContentError::Parse(_))));
    assert!(matches!(parse(&format!("{head}color = [1.0, 1.0, 1.0, 1.0]\nlife = 0.5")), Err(ContentError::Parse(_))));
}

#[test]
fn spark_numbers_are_checked_against_the_restorations_ranges() {
    let bad = |extra: &str| {
        let head = "format = 1\n[hit]\nclosing_speed_full = 10.0\nintensity_floor = 0.2\n[[hit.spark]]\ncolor = [1.0, 1.0, 1.0, 1.0]\nlife = 0.5\ncount = 5.0\n";
        parse(&format!("{head}{extra}"))
    };
    assert!(bad("").is_ok());
    assert!(matches!(bad("life = 0.5\n"), Err(ContentError::Parse(_))), "a key twice is no key");
    assert!(matches!(bad("count_variance = 0.5"), Err(ContentError::Field { field: "spark.count_variance", .. })));
    assert!(matches!(bad("life_variance = 1.0"), Err(ContentError::Field { field: "spark.life_variance", .. })));
    assert!(matches!(bad("extent = [0.5, -0.5, 0.0]"), Err(ContentError::Field { field: "spark.extent", .. })));
    assert!(matches!(bad("variation = [1.0, -1.0, 0.0]"), Err(ContentError::Field { field: "spark.variation", .. })));
    assert!(matches!(bad("velocity = [0.0, 0.0, 2000.0]"), Err(ContentError::Field { field: "spark.velocity", .. })));
    assert!(matches!(bad("length = 256.0"), Err(ContentError::Field { field: "spark.length", .. })));
    assert!(matches!(bad("height = -1.0"), Err(ContentError::Field { field: "spark.height", .. })));
    assert!(matches!(bad("elasticity = 300.0"), Err(ContentError::Field { field: "spark.elasticity", .. })));
    assert!(matches!(bad("gravity = 5000.0"), Err(ContentError::Field { field: "spark.gravity", .. })));
    assert!(matches!(bad("what = 1"), Err(ContentError::Parse(_))));
    let tall = "format = 1\n[hit]\nclosing_speed_full = 10.0\nintensity_floor = 0.2\n[[hit.spark]]\ncolor = [1.0, 1.5, 1.0, 1.0]\nlife = 0.5\ncount = 5.0";
    assert!(matches!(parse(tall), Err(ContentError::Field { field: "spark.color", .. })));
    let long = "format = 1\n[hit]\nclosing_speed_full = 10.0\nintensity_floor = 0.2\n[[hit.spark]]\ncolor = [1.0, 1.0, 1.0, 1.0]\nlife = 0.0\ncount = 5.0";
    assert!(matches!(parse(long), Err(ContentError::Field { field: "spark.life", .. })), "a spark that lives no time");
    let many = "format = 1\n[hit]\nclosing_speed_full = 10.0\nintensity_floor = 0.2\n[[hit.spark]]\ncolor = [1.0, 1.0, 1.0, 1.0]\nlife = 0.5\ncount = 999.0";
    assert!(matches!(parse(many), Err(ContentError::Field { field: "spark.count", .. })));
}

#[test]
fn at_most_two_sparks_a_channel() {
    let sparks = |n: usize| {
        let head = "format = 1\n[hit]\nclosing_speed_full = 10.0\nintensity_floor = 0.2\n";
        format!("{head}{}", "[[hit.spark]]\ncolor = [1.0, 1.0, 1.0, 1.0]\nlife = 0.5\ncount = 5.0\n".repeat(n))
    };
    assert!(parse(&sparks(MAX_XENON_SPARKS)).is_ok());
    assert!(matches!(parse(&sparks(MAX_XENON_SPARKS + 1)), Err(ContentError::Field { field: "spark", .. })));
}

#[test]
fn there_is_no_trail_and_no_streak_table() {
    let with_trail = format!("{PROFILE}\n[trail]\nspeed = 1.0");
    assert!(matches!(parse(&with_trail), Err(ContentError::Parse(_))), "the contrail has no variant");
    assert!(matches!(parse(&format!("{PROFILE}\n[streaks]\n")), Err(ContentError::Parse(_))));
}

#[test]
fn the_profile_path_is_the_xenon_folder_of_the_value() {
    assert_eq!(xenon_profile_path("mw2012"), "XENON/MW2012/profile.toml");
    assert_eq!(XENON_FOLDER, "XENON");
}
