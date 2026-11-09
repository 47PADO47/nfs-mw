use super::*;
use crate::content::{ContentError, SwapFit, parse_model_swap_profile};

const SET: &str = r#"
format = 1

[set]
kind = "trees"
kind_label = "Trees"
value = "mw2012"
value_label = "NFS Most Wanted 2012"
priority = 5
fit = "stand"

[files]
geometry = "TREES/MW2012/GEOMETRY.BIN"
textures = "TREES/MW2012/TEXTURES.BIN"

[fit]
spread = 1.5

[[model]]
targets = ["XT_CEDAR_1B", "XT_CEDAR_1A"]
model = "MW12_CONIFER_L0"

[[model]]
target = "XT_ROW_1B"
model = "MW12_ROW"
fit = "row"

[[model]]
target_hash = 0x1234ABCD
model = "MW12_OTHER"

[[texture]]
target = "ROAD_ASPHALT_01"
file = "TEXTURES/ROAD.png"
max_size = 1024

[[texture]]
name = "MW12_SIGN"
texture = "MW12_SIGN"
format = "bc3"

[[add]]
model = "MW12_BENCH"
pos = [10.0, 20.0, 3.0]
yaw = 90.0
"#;

#[test]
fn a_set_parses_with_its_models_textures_and_objects() {
    let set = parse("trees-mw2012", SET).unwrap();
    assert_eq!((set.kind.as_str(), set.value.as_str(), set.priority), ("trees", "mw2012", 5));
    assert_eq!(set.option().as_deref(), Some("trees_variant"));
    assert_eq!(set.kind_label.as_deref(), Some("Trees"));
    assert_eq!(set.limits.spread, 1.5);
    assert_eq!(set.models.len(), 3);
    assert_eq!(set.model_targets(), 4);
    assert_eq!(set.models[0].fit, SwapFit::Stand, "the set's default");
    assert_eq!(set.models[1].fit, SwapFit::Row);
    assert_eq!(set.models[2].targets, [Target::Hash(0x1234ABCD)]);
    assert_eq!(set.texture_entries.len(), 2);
    assert_eq!(set.texture_entries[0].replaces, Some(Target::Name("ROAD_ASPHALT_01".into())));
    assert_eq!(set.texture_entries[0].max_size, Some(1024));
    assert!(set.texture_entries[1].replaces.is_none(), "a name adds");
    assert_eq!(set.adds.len(), 1);
}

#[test]
fn the_value_defaults_to_the_file_name_and_an_always_set_needs_no_kind() {
    let text =
        "format = 1\n[set]\nalways = true\n[files]\ngeometry = \"A.BIN\"\n[[model]]\ntarget = \"S\"\nmodel = \"M\"\n";
    let set = parse("lamp-fix", text).unwrap();
    assert!(set.always && set.kind.is_empty() && set.option().is_none());
    assert_eq!((set.value.as_str(), set.value_label.as_str()), ("lamp-fix", "lamp-fix"));
    let without = text.replace("always = true", "priority = 1");
    assert!(matches!(parse("lamp-fix", &without), Err(ContentError::Field { field: "set.kind", .. })));
}

#[test]
fn bad_sets_are_refused_with_a_reason() {
    let with = |from: &str, to: &str| SET.replace(from, to);
    for (what, text) in [
        ("another format", with("format = 1", "format = 2")),
        ("a kind that is not an id", with("kind = \"trees\"", "kind = \"Big Trees\"")),
        ("a priority out of range", with("priority = 5", "priority = 5000")),
        ("a fit the schema does not know", with("fit = \"stand\"", "fit = \"grid\"")),
        ("a pack that is not a BIN", with("TREES/MW2012/GEOMETRY.BIN", "TREES/G.PNG")),
        ("a path with ..", with("TREES/MW2012/GEOMETRY.BIN", "../G.BIN")),
        ("a target listed twice", with("target = \"XT_ROW_1B\"", "target = \"XT_CEDAR_1B\"")),
        ("two kinds of target", with("target = \"XT_ROW_1B\"", "target = \"XT_ROW_1B\"\ntargets = [\"X\"]")),
        ("a texture file that is not a picture", with("TEXTURES/ROAD.png", "TEXTURES/ROAD.exe")),
        ("a texture size out of range", with("max_size = 1024", "max_size = 2")),
        ("a key the schema does not know", with("priority = 5", "priority = 5\nspeed = 1")),
        ("a texture that is neither replaced nor added", with("target = \"ROAD_ASPHALT_01\"\n", "")),
    ] {
        assert!(parse("s", &text).is_err(), "{what}");
    }
    assert!(parse("Not An Id", SET).is_err(), "a file name that is not an id");
}

#[test]
fn models_and_objects_need_a_geometry_file_and_a_set_needs_something_to_do() {
    let no_geometry = "format = 1\n[set]\nkind = \"k\"\n[[model]]\ntarget = \"S\"\nmodel = \"M\"\n";
    assert!(matches!(parse("s", no_geometry), Err(ContentError::Field { field: "files.geometry", .. })));
    let empty = "format = 1\n[set]\nkind = \"k\"\n";
    assert!(matches!(parse("s", empty), Err(ContentError::Field { field: "set", .. })));
    let textures_only = "format = 1\n[set]\nkind = \"k\"\n[[texture]]\ntarget = \"T\"\nfile = \"A.dds\"\n";
    assert!(parse("s", textures_only).is_ok(), "a set of textures needs no geometry");
    let from_pack = textures_only.replace("file = \"A.dds\"", "texture = \"A\"");
    assert!(parse("s", &from_pack).is_err(), "a pack texture needs the pack");
}

#[test]
fn the_old_trees_profile_is_the_same_set() {
    let profile = parse_model_swap_profile(
        "format = 1\ngeometry = \"TREES/X/GEOMETRY.BIN\"\ntextures = \"TREES/X/TEXTURES.BIN\"\n[[swap]]\nsolid = \"S1\"\nmodel = \"M1\"\n[[swap]]\nsolid = \"S2\"\nmodel = \"M2\"\nfit = \"row\"\n",
    )
    .unwrap();
    let set = from_model_swap("x", &profile);
    assert_eq!((set.kind.as_str(), set.value.as_str()), ("trees", "x"));
    assert_eq!(set.option().as_deref(), Some("trees_variant"));
    assert_eq!(set.geometry.as_deref(), Some("TREES/X/GEOMETRY.BIN"));
    assert_eq!(set.models.len(), 2);
    assert_eq!(set.models[1].fit, SwapFit::Row);
    assert_eq!(set.models[0].targets, [Target::Name("S1".into())]);
    assert_eq!(set.limits, profile.limits);
}
