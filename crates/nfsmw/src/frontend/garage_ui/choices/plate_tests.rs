use super::*;
use nfsmw_data::car::customization::PlateDesignChoice;

fn labels(catalog: Option<&Catalog>) -> Vec<String> {
    plate_pages(catalog).into_iter().map(|item| item.label).collect()
}

fn design(id: &str) -> PlateDesignChoice {
    PlateDesignChoice { id: id.into(), name: format!("DESIGN {id}"), group: None }
}

#[test]
fn a_car_without_a_plate_table_lists_neither_entry() {
    assert!(labels(None).is_empty(), "no catalogue at all");
    assert!(labels(Some(&Catalog::default())).is_empty(), "a stock car");
}

#[test]
fn a_plate_without_designs_lists_only_the_text_entry() {
    let catalog = Catalog { plate: true, ..Catalog::default() };
    assert_eq!(labels(Some(&catalog)), ["License Plate"]);
}

#[test]
fn a_plate_with_designs_lists_both_entries() {
    let catalog = Catalog { plate: true, plate_designs: vec![design("1"), design("2")], ..Catalog::default() };
    assert_eq!(labels(Some(&catalog)), ["License Plate", "Plate Design"]);
}

#[test]
fn designs_without_a_plate_list_nothing() {
    let catalog = Catalog { plate_designs: vec![design("1")], ..Catalog::default() };
    assert!(labels(Some(&catalog)).is_empty());
}
