use super::*;
use crate::garage::rider_gear::{Catalogue, Part};

fn catalogue() -> Catalogue {
    Catalogue::parse(
        br#"{
            "version": 1,
            "bodies": [{ "id": "b", "name": "Male 1", "gender": "male", "mesh": "m" }],
            "helmets": [{ "id": "h", "name": "H", "mesh": "m", "color": "c" }],
            "suits": [], "gloves": [{ "id": "g", "name": "G", "rgb": [1, 2, 3] }], "boots": []
        }"#,
    )
    .unwrap()
}

fn labels(offered: bool, catalogue: Option<&Catalogue>) -> Vec<String> {
    gear_pages(offered, catalogue).into_iter().map(|item| item.label).collect()
}

#[test]
fn a_bike_lists_a_page_for_each_part_that_has_items() {
    assert_eq!(labels(true, Some(&catalogue())), ["Rider Body", "Helmet", "Gloves"]);
}

#[test]
fn a_car_or_the_setting_off_or_no_catalogue_lists_nothing() {
    assert!(labels(false, Some(&catalogue())).is_empty(), "not a bike, or the setting is off");
    assert!(labels(true, None).is_empty(), "no mod offers gear");
}

#[test]
fn the_pages_are_the_riders_pages_of_their_part() {
    let items = gear_pages(true, Some(&catalogue()));
    assert_eq!(items[1].next, Some(Page::Rider { part: Part::Helmet }));
    assert!(items.iter().all(|item| item.enabled && item.option.is_none() && item.row.is_none()));
}
