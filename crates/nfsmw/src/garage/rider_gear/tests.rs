//! Tests of the rider gear catalogue, the kept choice and the process-wide state, on authored data.

use super::catalogue::{Catalogue, STOCK};
use super::choice::{self, Choice};
use super::*;

const CATALOGUE: &str = r#"{
    "version": 1,
    "stock": { "body": ["suit001_male"], "helmet": ["helmetRacing_default"], "visor": ["visorClosedRacing_default"], "boots": ["boots001"] },
    "bodies": [{ "id": "suit001_male", "name": "Male 1", "gender": "male", "mesh": "meshes/b.bbsk" }],
    "helmets": [
        { "id": "HL_A", "name": "A one", "mesh": "meshes/h.bbsk", "color": "tex/h.r6tx", "visor": { "mesh": "meshes/v.bbsk", "color": "tex/v.r6tx" } },
        { "id": "HL_B", "name": "B two", "mesh": "meshes/h2.bbsk", "color": "tex/h2.r6tx" }
    ],
    "suits": [{ "id": "SU_A", "name": "Suit A", "color": "tex/s.r6tx" }],
    "gloves": [{ "id": "GL_A", "name": "Glove A", "rgb": [10, 20, 30] }],
    "boots": []
}"#;

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-rider-gear-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("rider-gear.toml")
}

#[test]
fn the_catalogue_lists_each_part_with_the_stock_gear_first() {
    let catalogue = Catalogue::parse(CATALOGUE.as_bytes()).unwrap();
    let helmets = catalogue.entries(Part::Helmet);
    assert_eq!(helmets.len(), 3);
    assert_eq!((helmets[0].id.as_str(), helmets[0].label.as_str()), ("", STOCK));
    assert_eq!(helmets[1].id, "HL_A");
    assert_eq!(helmets[2].label, "B two");
    assert_eq!(catalogue.entries(Part::Boots).len(), 1, "a part with nothing to pick has only the stock entry");
    assert_eq!(catalogue.entries(Part::Gloves)[1].label, "Glove A");
    assert!(!catalogue.is_empty());
    assert!(Catalogue::parse(br#"{ "version": 1 }"#).unwrap().is_empty());
    assert!(Catalogue::parse(br#"{ "version": 2 }"#).is_err());
    assert!(Catalogue::parse(b"[").is_err());
}

#[test]
fn a_part_is_found_by_its_key() {
    assert_eq!(Part::of_key("gloves"), Some(Part::Gloves));
    assert_eq!(Part::of_key("glove"), None);
    assert!(Part::ALL.iter().all(|part| Part::of_key(part.key()) == Some(*part)));
}

#[test]
fn a_choice_round_trips_through_its_file_and_stock_parts_are_not_written() {
    let file = temp("round-trip");
    let mut choice = Choice::default();
    assert!(choice.is_stock());
    choice.set(Part::Helmet, "HL_A");
    choice.set(Part::Gloves, "GL_A");
    choice::write(&file, &choice).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("helmet") && text.contains("gloves") && !text.contains("suit"), "{text}");
    let back = choice::read(&file);
    assert_eq!(back, choice);
    assert_eq!(back.get(Part::Helmet), "HL_A");
    assert!(!back.is_stock());
    std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
}

#[test]
fn a_missing_or_broken_file_is_the_stock_gear_and_a_value_that_is_no_text_is_skipped() {
    let file = temp("broken");
    assert_eq!(choice::read(&file), Choice::STOCK);
    std::fs::write(&file, "gear = [").unwrap();
    assert!(choice::read(&file).is_stock());
    std::fs::write(&file, "[gear]\nhelmet = 5\nsuit = \"SU_A\"\n").unwrap();
    let back = choice::read(&file);
    assert_eq!((back.helmet.as_str(), back.suit.as_str()), ("", "SU_A"));
    std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
}

/// The state is one for the process, so its checks are one test: they must not interleave.
#[test]
fn the_setting_and_the_preview_decide_what_is_current_and_move_the_generation() {
    let mut shown = Choice::default();
    shown.set(Part::Suit, "SU_A");
    set_enabled(true);
    set_preview(None);
    let before = generation();
    assert!(enabled());
    assert!(current().is_stock());
    set_preview(Some(shown.clone()));
    assert_ne!(generation(), before, "a preview moves it");
    assert_eq!(current(), shown, "the garage shows the preview");
    assert!(committed().is_stock(), "what the world wears is not the preview");
    set_enabled(false);
    assert!(current().is_stock() && committed().is_stock(), "off: the rider's own gear");
    let off = generation();
    set_enabled(false);
    assert_eq!(generation(), off, "no change, no move");
    set_enabled(true);
    set_preview(None);
    assert!(current() == committed());
}
