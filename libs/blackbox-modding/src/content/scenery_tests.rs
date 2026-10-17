use super::parse_scenery as parse;
use super::*;

const PACK: &str = r#"
[pack]
file = "SCENERY/MONUMENT.BUN"
section = 1100
anchor = [1200.0, -900.0, 40.0, 180.0]
source_game = "nfsmw"

[[place]]
model = "MON_ARCH_A"
pos = [1200.0, -900.0, 0.0]
yaw = 90.0
coarse = "MON_ARCH_C"
draw_m = 900.0
lod_m = 300.0
collision = "static"
"#;

/// The manifest text with one line replaced: `from` is found in the valid pack and swapped for `to`.
fn with(from: &str, to: &str) -> String {
    assert!(PACK.contains(from), "the test text has {from:?}");
    PACK.replace(from, to)
}

fn refused(text: &str) -> ContentError {
    parse("monument", text).expect_err("the manifest is refused")
}

#[test]
fn a_valid_pack_parses_with_its_defaults() {
    let pack = parse("monument", PACK).unwrap();
    assert_eq!(pack.key, "monument");
    assert_eq!(pack.file, "SCENERY/MONUMENT.BUN");
    assert_eq!(pack.section, 1100);
    assert_eq!(pack.anchor, [1200.0, -900.0, 40.0, 180.0]);
    assert_eq!(pack.source_game.as_deref(), Some("nfsmw"));
    let place = &pack.places[0];
    assert_eq!(place.model, "MON_ARCH_A");
    assert_eq!(place.orientation, Orientation::Yaw(90.0));
    assert_eq!(place.scale, 1.0, "the scale defaults to 1");
    assert_eq!(place.coarse.as_deref(), Some("MON_ARCH_C"));
    assert_eq!(place.mirror, None);
    assert_eq!(place.collision, Collision::Static);
    assert_eq!(place.exclude, None, "the game's default is applied by the game, not here");
    assert!(!place.boost);
}

#[test]
fn a_rotation_matrix_is_read_in_place_of_a_yaw() {
    let text = with("yaw = 90.0", "rotation = [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]");
    let pack = parse("monument", &text).unwrap();
    assert_eq!(pack.places[0].orientation, Orientation::Rows([[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]));
}

#[test]
fn yaw_and_rotation_together_are_refused() {
    let text = with("yaw = 90.0", "yaw = 90.0\nrotation = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]");
    assert!(matches!(refused(&text), ContentError::Place { at: 1, reason: "yaw and rotation are both given" }));
}

#[test]
fn a_rotation_component_beyond_the_format_is_refused() {
    let text = with("yaw = 90.0", "rotation = [[5.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]");
    assert!(matches!(refused(&text), ContentError::Place { at: 1, .. }));
}

#[test]
fn scale_must_be_above_zero_and_at_most_four() {
    assert!(parse("monument", &with("yaw = 90.0", "yaw = 90.0\nscale = 2.0")).is_ok());
    assert!(matches!(refused(&with("yaw = 90.0", "yaw = 90.0\nscale = 0.0")), ContentError::Place { .. }));
    assert!(matches!(refused(&with("yaw = 90.0", "yaw = 90.0\nscale = 4.5")), ContentError::Place { .. }));
}

#[test]
fn a_draw_distance_must_not_be_less_than_its_lod_distance() {
    let text = with("draw_m = 900.0", "draw_m = 100.0");
    assert!(matches!(refused(&text), ContentError::Place { reason: "lod_m is beyond draw_m", .. }));
}

#[test]
fn an_unknown_collision_mode_is_refused() {
    let text = with("collision = \"static\"", "collision = \"mesh\"");
    assert!(matches!(refused(&text), ContentError::Place { reason: "collision must be none, prop or static", .. }));
}

#[test]
fn the_pack_file_must_stay_under_files() {
    // The backslash is written as a TOML escape, so the file text holds one backslash.
    for file in
        ["../SCENERY/X.BUN", "/SCENERY/X.BUN", "C:/X.BUN", "SCENERY\\\\X.BUN", "SCENERY/X.TXT", "SCENERY//X.BUN"]
    {
        let text = with("SCENERY/MONUMENT.BUN", file);
        assert!(matches!(refused(&text), ContentError::InvalidPackFile(_)), "{file} is refused");
    }
}

#[test]
fn section_zero_and_numbers_past_the_index_type_are_refused() {
    assert!(matches!(refused(&with("section = 1100", "section = 0")), ContentError::InvalidSection(0)));
    assert!(matches!(refused(&with("section = 1100", "section = 70000")), ContentError::Parse(_)));
    assert!(matches!(refused(&with("section = 1100", "section = 32768")), ContentError::InvalidSection(32768)));
}

#[test]
fn only_the_tile_route_is_accepted() {
    let text = with("source_game = \"nfsmw\"", "source_game = \"nfsmw\"\nroute = \"shared\"");
    assert!(matches!(refused(&text), ContentError::RouteNotInV1(route) if route == "shared"));
    assert!(parse("monument", &with("source_game = \"nfsmw\"", "source_game = \"nfsmw\"\nroute = \"tile\"")).is_ok());
}

#[test]
fn a_pack_with_no_place_is_refused() {
    let text = PACK.split("[[place]]").next().unwrap();
    assert_eq!(refused(text), ContentError::NoPlace);
}

#[test]
fn an_unknown_key_refuses_the_pack_with_the_key_named() {
    let text = with("collision = \"static\"", "collision = \"static\"\nmesh = \"x\"");
    let error = refused(&text).to_string();
    assert!(error.contains("mesh"), "the reason names the key: {error}");
}

#[test]
fn a_pack_key_must_match_the_file_name() {
    let text = with("[pack]", "[pack]\nkey = \"other\"");
    assert!(matches!(refused(&text), ContentError::KeyMismatch { .. }));
    assert!(parse("monument", &with("[pack]", "[pack]\nkey = \"monument\"")).is_ok());
}

#[test]
fn a_model_name_must_be_a_plain_name() {
    let text = with("model = \"MON_ARCH_A\"", "model = \"MON ARCH\"");
    assert!(matches!(refused(&text), ContentError::InvalidModel(_)));
}

#[test]
fn a_bad_file_name_is_refused_before_its_contents() {
    assert!(matches!(parse("bad name", PACK), Err(ContentError::InvalidId(_))));
}

#[test]
fn the_anchor_needs_a_positive_radius() {
    assert_eq!(refused(&with("180.0]", "0.0]")), ContentError::InvalidAnchor);
}
