//! The rider's gear rows: which parts get one, where they sit in the Gameplay page and when they are dimmed.

use super::super::category_rows::gameplay;
use super::*;
use crate::settings::{Partial, Settings};

const CATALOGUE: &str = r#"{
    "version": 1,
    "bodies": [{ "id": "B1", "name": "Body one", "gender": "male", "mesh": "m.bbsk" }],
    "helmets": [{ "id": "H1", "name": "Helmet one", "mesh": "h.bbsk", "color": "h.r6tx" }],
    "suits": [], "gloves": [], "boots": []
}"#;

fn catalogue() -> Catalogue {
    Catalogue::parse(CATALOGUE.as_bytes()).unwrap()
}

fn settings() -> Settings {
    Settings::from(Partial::default())
}

#[test]
fn a_part_with_something_to_pick_gets_a_row_and_nothing_else_does() {
    let rows = rows(Some(&catalogue()));
    let settings: Vec<Setting> = rows.iter().map(|row| row.setting).collect();
    assert_eq!(settings, [Setting::Gear(Part::Body), Setting::Gear(Part::Helmet)]);
    assert_eq!(rows[1].title, Title::Key("options-rider-helmet"));
    assert!(super::rows(None).is_empty(), "no mod, no rows");
}

#[test]
fn every_part_has_its_own_title() {
    let titles: Vec<Title> = Part::ALL.into_iter().map(title).collect();
    for (at, title) in titles.iter().enumerate() {
        assert_eq!(titles.iter().filter(|other| *other == title).count(), 1, "part {at} shares its title");
        let Title::Key(id) = title else { panic!("a catalogue message") };
        assert!(!title.text().is_empty() && title.text() != *id, "{id} has no message");
    }
}

#[test]
fn the_gear_follows_the_switch_in_the_gameplay_page() {
    let page = gameplay(rows(Some(&catalogue())));
    let at = |setting: Setting| page.iter().position(|row| row.setting == setting).unwrap();
    assert_eq!(at(Setting::Gear(Part::Body)), at(Setting::RiderGear) + 1);
    assert_eq!(at(Setting::Gear(Part::Helmet)), at(Setting::RiderGear) + 2);
    assert_eq!(at(Setting::RidingStyle), at(Setting::RiderGear) + 3);
    assert_eq!(gameplay(Vec::new()).len(), page.len() - 2, "without gear the page is as it was");
}

#[test]
fn the_rows_are_dimmed_while_the_switch_is_off() {
    let mut off = settings();
    off.rider_gear = false;
    assert!(Setting::Gear(Part::Suit).enabled(&settings()));
    assert!(!Setting::Gear(Part::Suit).enabled(&off));
}

#[test]
fn an_id_the_list_lacks_is_the_riders_own_gear() {
    let entries = catalogue().entries(Part::Helmet);
    assert_eq!(position(&entries, "H1"), 1);
    assert_eq!(position(&entries, "gone"), 0);
    assert_eq!(position(&entries, ""), 0);
}
