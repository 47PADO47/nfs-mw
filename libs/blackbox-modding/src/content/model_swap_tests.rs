use super::*;

const PROFILE: &str = r#"
format = 1
geometry = "TREES/MYMOD/GEOMETRY.BIN"
textures = "TREES/MYMOD/TEXTURES.BIN"

[[swap]]
solid = "TALL_TREE_1B"
model = "TALL_TREE_L0"

[[swap]]
solid = "TREE_ROW_1B"
model = "TREE_L1"
fit = "row"
"#;

#[test]
fn swaps_parse_in_file_order_and_fit_defaults_to_height() {
    let profile = parse(PROFILE).unwrap();
    assert_eq!(profile.geometry, "TREES/MYMOD/GEOMETRY.BIN");
    assert_eq!(profile.textures, "TREES/MYMOD/TEXTURES.BIN");
    assert_eq!(profile.swaps.len(), 2);
    assert_eq!(profile.swaps[0], Swap { solid: "TALL_TREE_1B".into(), model: "TALL_TREE_L0".into(), fit: Fit::Height });
    assert_eq!(profile.swaps[1].fit, Fit::Row);
}

#[test]
fn the_profile_lives_in_the_trees_folder() {
    assert_eq!(model_swap_profile_path("mymod"), "TREES/MYMOD/profile.toml");
    assert_eq!(TREES_FOLDER, "TREES");
}

#[test]
fn a_profile_needs_the_format_and_a_swap() {
    let wrong = PROFILE.replace("format = 1", "format = 2");
    assert!(matches!(parse(&wrong), Err(ContentError::Field { field: "format", .. })));
    let none = "format = 1\ngeometry = \"A.BIN\"\ntextures = \"B.BIN\"\n";
    assert!(matches!(parse(none), Err(ContentError::Field { field: "swap", .. })));
    assert!(matches!(parse("geometry = \"A.BIN\"\ntextures = \"B.BIN\""), Err(ContentError::Parse(_))));
}

#[test]
fn bad_packs_names_and_keys_are_refused() {
    let with = |geometry: &str, solid: &str, extra: &str| {
        format!(
            "format = 1\ngeometry = \"{geometry}\"\ntextures = \"T.BIN\"\n[[swap]]\nsolid = \"{solid}\"\nmodel = \"M\"\n{extra}"
        )
    };
    assert!(parse(&with("G.BIN", "S", "")).is_ok());
    for (what, text) in [
        ("a pack that is not a BIN", with("G.PNG", "S", "")),
        ("a pack path with ..", with("../G.BIN", "S", "")),
        ("a solid name with a space", with("G.BIN", "A B", "")),
        ("an empty solid name", with("G.BIN", "", "")),
        ("a fit the schema does not know", with("G.BIN", "S", "fit = \"grid\"")),
        ("a key the schema does not know", with("G.BIN", "S", "yaw = 1.0")),
    ] {
        assert!(parse(&text).is_err(), "{what}");
    }
}

#[test]
fn a_solid_is_replaced_once() {
    let two = format!(
        "{}\n[[swap]]\nsolid = \"TALL_TREE_1B\"\nmodel = \"OTHER\"\n",
        PROFILE.replace("fit = \"row\"", "fit = \"height\"")
    );
    assert!(matches!(parse(&two), Err(ContentError::Field { field: "swap.solid", .. })));
}

#[test]
fn more_than_4096_swaps_are_too_many() {
    let head = "format = 1\ngeometry = \"G.BIN\"\ntextures = \"T.BIN\"\n";
    let many =
        |count: usize| (0..count).map(|n| format!("[[swap]]\nsolid = \"S{n}\"\nmodel = \"M\"\n")).collect::<String>();
    assert!(parse(&format!("{head}{}", many(MAX_SWAPS))).is_ok());
    assert!(parse(&format!("{head}{}", many(MAX_SWAPS + 1))).is_err());
}

#[test]
fn the_fit_limits_default_and_are_read_from_the_fit_table() {
    assert_eq!(parse(PROFILE).unwrap().limits, FitLimits::default());
    let tuned = PROFILE.replacen("[[swap]]", "[fit]\nspread = 1.5\nsqueeze = 0.4\nmin_height = 0.6\n\n[[swap]]", 1);
    let limits = parse(&tuned).unwrap().limits;
    assert_eq!(limits, FitLimits { spread: 1.5, squeeze: 0.4, min_height: 0.6 });
    let partial = PROFILE.replacen("[[swap]]", "[fit]\nspread = 2.0\n\n[[swap]]", 1);
    let limits = parse(&partial).unwrap().limits;
    assert_eq!(limits, FitLimits { spread: 2.0, ..FitLimits::default() }, "a key left out keeps its default");
}

#[test]
fn fit_limits_out_of_range_or_unknown_are_refused() {
    let with = |line: &str| PROFILE.replacen("[[swap]]", &format!("[fit]\n{line}\n\n[[swap]]"), 1);
    for line in
        ["spread = 0.9", "spread = 5.0", "squeeze = 0.0", "squeeze = 1.5", "min_height = -0.1", "min_height = 1.1"]
    {
        assert!(matches!(parse(&with(line)), Err(ContentError::Field { .. })), "{line}");
    }
    assert!(parse(&with("grow = 2.0")).is_err(), "a key the schema does not know");
}
