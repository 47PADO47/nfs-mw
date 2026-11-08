//! Tests of the rider gear catalogue, the kept choice and the process-wide state, on authored data.

use super::catalogue::{Catalogue, STOCK};
use super::choice::{self, Choice, Kept};
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
fn an_id_the_list_lacks_is_the_riders_own_gear() {
    let entries = Catalogue::parse(CATALOGUE.as_bytes()).unwrap().entries(Part::Helmet);
    assert_eq!(position(&entries, "HL_B"), 2);
    assert_eq!(position(&entries, "gone"), 0);
    assert_eq!(position(&entries, ""), 0, "his own gear");
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
    choice::write_kept(&file, &Kept { choice: choice.clone(), owned: Default::default() }).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("helmet") && text.contains("gloves") && !text.contains("suit"), "{text}");
    let back = choice::read_kept(&file).choice;
    assert_eq!(back, choice);
    assert_eq!(back.get(Part::Helmet), "HL_A");
    assert!(!back.is_stock());
    std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
}

#[test]
fn a_missing_or_broken_file_is_the_stock_gear_and_a_value_that_is_no_text_is_skipped() {
    let file = temp("broken");
    assert_eq!(choice::read_kept(&file).choice, Choice::STOCK);
    std::fs::write(&file, "gear = [").unwrap();
    assert!(choice::read_kept(&file).choice.is_stock());
    std::fs::write(&file, "[gear]\nhelmet = 5\nsuit = \"SU_A\"\n").unwrap();
    let back = choice::read_kept(&file).choice;
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

#[test]
fn what_the_shop_sold_is_kept_with_the_choice_and_survives_a_change_of_choice() {
    let file = temp("owned");
    let mut kept = Kept::default();
    kept.choice.set(Part::Helmet, "HL_A");
    kept.owned.insert(choice::owned_key(Part::Helmet, "HL_A"));
    kept.owned.insert(choice::owned_key(Part::Suit, "SU_A"));
    choice::write_kept(&file, &kept).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("helmet:HL_A") && text.contains("suit:SU_A"), "{text}");
    assert_eq!(choice::read_kept(&file), kept);
    // Wearing the rider's own gear again keeps the helmet owned: it is the choice that changed, not the purchase.
    kept.choice = Choice::STOCK;
    choice::write_kept(&file, &kept).unwrap();
    let back = choice::read_kept(&file);
    assert!(back.choice.is_stock() && back.owned.len() == 2);
    // A stock profile writes no `owned` list at all.
    choice::write_kept(&file, &Kept::default()).unwrap();
    assert!(!std::fs::read_to_string(&file).unwrap().contains("owned"));
    std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
}

#[test]
fn gear_costs_the_parts_default_price_unless_it_is_the_riders_own_or_already_owned() {
    use super::price::{changes, cost, default_price};
    let owned = |part: Part, id: &str| part == Part::Helmet && id == "HL_A";
    assert_eq!(cost(Part::Helmet, "", owned), 0, "the rider's own gear is free");
    assert_eq!(cost(Part::Helmet, "HL_A", owned), 0, "a bought item is free to wear again");
    assert_eq!(cost(Part::Helmet, "HL_B", owned), default_price(Part::Helmet));
    assert_eq!(cost(Part::Suit, "SU_A", owned), default_price(Part::Suit));
    let (mut now, mut next) = (Choice::default(), Choice::default());
    now.set(Part::Gloves, "GL_A");
    next.set(Part::Gloves, "GL_A");
    next.set(Part::Helmet, "HL_B");
    next.set(Part::Suit, "SU_A");
    assert_eq!(
        changes(&now, &next, owned),
        [(Part::Helmet, default_price(Part::Helmet)), (Part::Suit, default_price(Part::Suit))],
        "only the parts that differ"
    );
    assert!(changes(&now, &now, owned).is_empty());
    assert!(Part::ALL.iter().all(|part| default_price(*part) > 0), "every part has a price");
}

#[test]
fn the_shop_sells_gear_for_a_bike_from_the_catalogue_only() {
    set_enabled(true);
    let (stock, mut asked) = (Choice::default(), Choice::default());
    assert!(validate_change(false, &stock, &stock).is_ok(), "no change, no check");
    asked.set(Part::Helmet, "HL_A");
    assert!(validate_change(false, &stock, &asked).unwrap_err().to_string().contains("bike"));
    set_enabled(false);
    assert!(validate_change(true, &stock, &asked).unwrap_err().to_string().contains("switched off"));
    set_enabled(true);
    // No catalogue is loaded here: an item nobody lists is refused, and the rider's own gear never is.
    assert!(validate_change(true, &stock, &asked).unwrap_err().to_string().contains("not in the catalogue"));
    assert!(validate_change(true, &asked, &stock).is_ok());
}
