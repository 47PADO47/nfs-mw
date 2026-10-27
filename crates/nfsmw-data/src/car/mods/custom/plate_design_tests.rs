use std::path::PathBuf;

use blackbox_tpk::{PixelFormat, TextureSource, write_tpk};

use super::*;

const INDEX: &str = "PLATES/SET/DESIGNS.TOML";

fn plate() -> Plate {
    Plate {
        text: None,
        picture: "CAR_PLATE".into(),
        glyphs: "ABCDEFGHIJKLMNOPQRSTUVWXYZ 123456789".chars().collect(),
        columns: 9,
        rows: 4,
        slots: 8,
        aliases: Vec::new(),
        designs: Some(INDEX.into()),
        default_design: None,
    }
}

/// One DXT1 block of a single colour (RGB565 `color`, both end points the same).
fn solid(color: u16) -> Vec<u8> {
    let end = color.to_le_bytes();
    vec![end[0], end[1], end[0], end[1], 0, 0, 0, 0]
}

fn picture(name: &str, color: u16) -> TextureSource {
    TextureSource::new(name, 4, 4, PixelFormat::Dxt1, 1, solid(color))
}

/// A scratch install with an index and a pack of `pictures`, and the folder to remove.
fn install(name: &str, index: &str, pictures: &[TextureSource]) -> (PathBuf, GameDir) {
    let root = std::env::temp_dir().join(format!("nfsmw-plate-designs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let folder = root.join("install/PLATES/SET");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("DESIGNS.TOML"), index).unwrap();
    std::fs::write(folder.join("DESIGNS.BIN"), write_tpk("designs", pictures).unwrap()).unwrap();
    let dir = GameDir::open(root.join("install")).unwrap();
    (root, dir)
}

const TWO: &str = r#"
pack = "DESIGNS.BIN"

[[design]]
id = "1"
name = "START ME UP"
picture = "PD1"
group = "Base game"

[[design]]
id = "2"
name = "NO LIMITS"
picture = "PD2"
ink = [1, 2, 3]
"#;

#[test]
fn an_index_lists_the_designs_and_puts_its_pack_beside_it() {
    let found = parse_index(INDEX, TWO).unwrap();
    assert_eq!(found.pack, "PLATES/SET/DESIGNS.BIN");
    let ids: Vec<&str> = found.designs.iter().map(|design| design.id.as_str()).collect();
    assert_eq!(ids, ["1", "2"]);
    assert_eq!(found.designs[0].group.as_deref(), Some("Base game"));
    assert_eq!(found.designs[1].ink, Some([1, 2, 3]));
    assert_eq!(found.designs[1].glyphs, None);
    assert_eq!(parse_index("DESIGNS.TOML", TWO).unwrap().pack, "DESIGNS.BIN", "an index at the root");
}

#[test]
fn a_malformed_or_repeated_design_is_left_out_and_the_rest_stay() {
    let text = format!(
        "{TWO}\n[[design]]\nid = \"1\"\nname = \"AGAIN\"\npicture = \"PD3\"\n\n[[design]]\nid = \"has space\"\nname = \"X\"\npicture = \"PD4\"\n\n[[design]]\nid = \"5\"\nname = \"\"\npicture = \"PD5\"\n"
    );
    let found = parse_index(INDEX, &text).unwrap();
    assert_eq!(found.designs.len(), 2, "the repeated id, the id with a space and the empty name are out");
}

#[test]
fn an_index_that_is_not_toml_or_names_a_pack_outside_its_folder_is_an_error() {
    assert!(parse_index(INDEX, "pack = [").is_err());
    assert!(parse_index(INDEX, "design = []").is_err(), "no pack");
    assert!(parse_index(INDEX, "pack = \"../DESIGNS.BIN\"").is_err());
    assert!(parse_index(INDEX, "pack = \"A/DESIGNS.BIN\"").is_err());
    assert!(parse_index(INDEX, "pack = \"DESIGNS.BIN\"\nsurprise = 1").is_err(), "unknown keys are mistakes");
}

#[test]
fn only_the_designs_whose_picture_is_in_the_pack_are_offered() {
    let (root, dir) = install("offered", TWO, &[picture("PD1", 0xF800)]);
    let offered = offered(&dir, "CAR", &plate());
    assert_eq!(offered.len(), 1, "PD2 is not in the pack");
    assert_eq!((offered[0].id.as_str(), offered[0].name.as_str()), ("1", "START ME UP"));
    assert_eq!(offered[0].group.as_deref(), Some("Base game"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_plate_without_an_index_or_with_a_missing_one_offers_nothing() {
    let (root, dir) = install("none", TWO, &[picture("PD1", 0xF800)]);
    assert!(offered(&dir, "CAR", &Plate { designs: None, ..plate() }).is_empty());
    assert!(offered(&dir, "CAR", &Plate { designs: Some("PLATES/SET/MISSING.TOML".into()), ..plate() }).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_design_is_drawn_from_its_picture_with_its_own_ink_or_one_chosen_by_brightness() {
    let (root, dir) = install("ground", TWO, &[picture("PD1", 0xFFE0), picture("PD2", 0xF800)]);
    let yellow = ground(&dir, &plate(), "1").unwrap();
    assert_eq!(yellow.ink, [20, 20, 24], "dark ink on a light picture");
    let red = ground(&dir, &plate(), "2").unwrap();
    assert_eq!(red.ink, [1, 2, 3], "the design names its ink");
    assert!(ground(&dir, &plate(), "9").is_err(), "no such design");
    assert!(ground(&dir, &Plate { designs: None, ..plate() }, "1").is_err(), "no index");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_design_whose_picture_is_missing_from_the_pack_cannot_be_drawn() {
    let (root, dir) = install("missing", TWO, &[picture("PD1", 0xFFE0)]);
    assert!(ground(&dir, &plate(), "2").is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_index_is_read_once_while_the_file_is_unchanged() {
    let (root, dir) = install("memo", TWO, &[picture("PD1", 0xFFE0)]);
    let first = set(&dir, INDEX).unwrap();
    assert!(Arc::ptr_eq(&first, &set(&dir, INDEX).unwrap()));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_levels_of_a_sheet_need_contrast_between_ground_and_ink() {
    // 100 pixels: 90 light (200) and 10 dark (20).
    let mut rgba = Vec::new();
    for n in 0..100 {
        let value = if n < 10 { 20 } else { 200 };
        rgba.extend([value, value, value, 255]);
    }
    let (ground, ink) = levels(&rgba).unwrap();
    assert!((ground - 200.0).abs() < 2.0 && (ink - 20.0).abs() < 2.0, "{ground} {ink}");
    assert_eq!(levels(&[128u8; 400]), None, "a flat sheet");
}
