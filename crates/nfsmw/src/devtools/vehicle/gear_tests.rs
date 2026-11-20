use super::*;
use crate::garage::rider_gear::catalogue::{Helmet, Suit};

fn catalogue() -> Catalogue {
    Catalogue {
        version: 1,
        helmets: vec![
            Helmet {
                id: "HL_A".into(),
                name: "AGV Corsa Black".into(),
                mesh: String::new(),
                color: String::new(),
                visor: None,
            },
            Helmet {
                id: "HL_B".into(),
                name: "AGV Corsa White".into(),
                mesh: String::new(),
                color: String::new(),
                visor: None,
            },
        ],
        suits: vec![Suit { id: "SU_A".into(), name: "Alpine Suit".into(), color: String::new() }],
        ..Catalogue::default()
    }
}

#[test]
fn only_the_parts_with_something_to_choose_are_listed() {
    let rows = rows(&catalogue(), &Choice::default());
    let parts: Vec<Part> = rows.iter().map(|row| row.part).collect();
    assert_eq!(parts, [Part::Helmet, Part::Suit]);
    assert_eq!(rows[0].entries.len(), 3, "the rider's own gear and two helmets");
    assert_eq!(rows[0].worn, 0);
}

#[test]
fn what_is_worn_is_read_from_the_choice() {
    let mut choice = Choice::default();
    choice.set(Part::Helmet, "HL_B");
    assert_eq!(rows(&catalogue(), &choice)[0].worn, 2);
    choice.set(Part::Helmet, "gone");
    assert_eq!(rows(&catalogue(), &choice)[0].worn, 0, "an item the catalogue lost is the rider's own gear");
}

#[test]
fn a_gear_word_is_stock_an_id_a_number_from_one_or_a_name() {
    let entries = catalogue().entries(Part::Helmet);
    assert_eq!(find(&entries, "stock").unwrap(), "");
    assert_eq!(find(&entries, "NONE").unwrap(), "");
    assert_eq!(find(&entries, "hl_b").unwrap(), "HL_B");
    assert_eq!(find(&entries, "1").unwrap(), "", "the first entry is the rider's own gear");
    assert_eq!(find(&entries, "2").unwrap(), "HL_A");
    assert_eq!(find(&entries, "white").unwrap(), "HL_B");
    assert!(find(&entries, "4").is_err() && find(&entries, "0").is_err());
    assert!(find(&entries, "agv").unwrap_err().contains("several"));
    assert!(find(&entries, "ducati").is_err());
}

#[test]
fn the_listing_numbers_the_entries_and_stars_the_worn_one() {
    let mut choice = Choice::default();
    choice.set(Part::Helmet, "HL_A");
    let text = text(&rows(&catalogue(), &choice));
    assert!(text.contains("Helmet (helmet): 1: Stock, 2: AGV Corsa Black*, 3: AGV Corsa White"), "{text}");
    assert!(text.contains("Suit (suit): 1: Stock*, 2: Alpine Suit"), "{text}");
}
