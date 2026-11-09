use super::*;

const PROFILE: &str = r#"
format = 1
[[set]]
surface = "paved"
slip = "SKIDMARKS/MW2012/PAVED_SLIP.PNG"
spin = "SKIDMARKS/MW2012/PAVED_SPIN.PNG"
slip_tile = 4.0
width = 1.2
[[set]]
surface = "paved"
wet = true
spin = "SKIDMARKS/MW2012/WET_SPIN.PNG"
opacity = 0.5
[[set]]
surface = "loose"
slip = "SKIDMARKS/MW2012/LOOSE_SLIP.PNG"
life = 90.0
"#;

#[test]
fn sets_parse_in_file_order_with_defaults() {
    let profile = parse(PROFILE).unwrap();
    assert_eq!(profile.sets.len(), 3);
    let dry = &profile.sets[0];
    assert_eq!(
        (dry.surface, dry.wet, dry.slip_tile, dry.spin_tile, dry.width),
        (MarkSurface::Paved, false, 4.0, 2.0, 1.2)
    );
    assert_eq!((dry.opacity, dry.life), (1.0, 45.0));
    assert_eq!(profile.sets[1].slip, None);
    assert_eq!(profile.sets[2].life, 90.0);
}

#[test]
fn the_wet_set_is_for_a_wet_road_and_a_surface_falls_back_to_its_dry_set() {
    let profile = parse(PROFILE).unwrap();
    assert!(profile.set_for(MarkSurface::Paved, true).unwrap().wet);
    assert!(!profile.set_for(MarkSurface::Paved, false).unwrap().wet);
    assert!(!profile.set_for(MarkSurface::Loose, true).unwrap().wet, "no wet set: the dry one");
    let paved_only = parse("format = 1\n[[set]]\nsurface = \"paved\"\nslip = \"A.PNG\"").unwrap();
    assert!(paved_only.set_for(MarkSurface::Loose, false).is_none(), "no set: the game's own mark");
}

#[test]
fn a_profile_needs_the_format_and_a_set_with_a_picture() {
    let wrong = parse("format = 2\n[[set]]\nsurface = \"paved\"\nslip = \"A.PNG\"");
    assert!(matches!(wrong, Err(ContentError::Field { field: "format", .. })));
    assert!(matches!(parse("format = 1"), Err(ContentError::Field { field: "set", .. })));
    assert!(matches!(parse("format = 1\n[[set]]\nsurface = \"paved\""), Err(ContentError::Field { field: "set", .. })));
    assert!(matches!(parse("[[set]]\nsurface = \"paved\"\nslip = \"A.PNG\""), Err(ContentError::Parse(_))));
}

#[test]
fn numbers_paths_and_repeats_are_checked() {
    let set = |extra: &str| parse(&format!("format = 1\n[[set]]\nsurface = \"paved\"\nslip = \"A.PNG\"\n{extra}"));
    assert!(set("").is_ok());
    assert!(matches!(set("width = 9.0"), Err(ContentError::Field { field: "set.width", .. })));
    assert!(matches!(set("opacity = 1.5"), Err(ContentError::Field { field: "set.opacity", .. })));
    assert!(matches!(set("life = 1.0"), Err(ContentError::Field { field: "set.life", .. })));
    assert!(matches!(set("spin_tile = 0.0"), Err(ContentError::Field { field: "set.spin_tile", .. })));
    assert!(matches!(set("what = 1"), Err(ContentError::Parse(_))));
    assert!(matches!(set("spin = \"../B.PNG\""), Err(ContentError::Field { field: "set.spin", .. })));
    let twice =
        "format = 1\n[[set]]\nsurface = \"paved\"\nslip = \"A.PNG\"\n[[set]]\nsurface = \"paved\"\nspin = \"B.PNG\"";
    assert!(matches!(parse(twice), Err(ContentError::Field { field: "set", .. })));
}

#[test]
fn the_profile_path_is_the_skid_mark_folder_of_the_value() {
    assert_eq!(skid_marks_profile_path("mw2012"), "SKIDMARKS/MW2012/profile.toml");
    assert_eq!(SKIDMARKS_FOLDER, "SKIDMARKS");
}
