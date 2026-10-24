use super::*;
use crate::garage::{Car, Selection};
use blackbox_hash::{bstring_hash, bstring_hash_continue};
use nfsmw_data::car::customization::{Catalog, Choice};

fn fixture() -> Model {
    let a = bstring_hash("BRAND_A");
    let b = bstring_hash("BRAND_B");
    let mut catalog = Catalog::default();
    catalog.stock[Category::Body as usize] = Some(42);
    catalog.decals[0].body_positions.insert(42, 1);
    for (hash, brand, name) in [
        (101, a, a),
        (102, b, bstring_hash_continue(b, b"_WHITE")),
        (103, a, bstring_hash_continue(a, b"_WHITE")),
        (104, b, b),
        (105, a, bstring_hash("CUSTOM_WHITE_ART")),
    ] {
        catalog.decals[0].choices.push(Choice {
            hash,
            label: "Localized name without color hints".into(),
            language_hash: None,
            carbon: false,
        });
        catalog.decals[0].variants.insert(hash, DecalVariant { brand, name });
    }
    Model {
        cars: vec![Car { folder: "CAR".into(), parts: Some(catalog.into()), ..Default::default() }],
        draft: Selection { car: "CAR".into(), ..Default::default() },
        ..Default::default()
    }
}

fn entries(model: &Model) -> Vec<Item> {
    super::super::super::items(super::super::super::Page::Choices { row: Row::Decal(0, 0), backroom: false }, model)
}

#[test]
fn decal_filter_preserves_brand_and_underlying_row_option_without_using_labels() {
    let model = fixture();
    let row = Row::Decal(0, 0);
    assert!(initial_black(&model, row));
    let a = brand(&model, row, 1).unwrap();
    let mut black = entries(&model);
    assert_eq!(filter(&model, row, true, &mut black, Some(a)), 1);
    assert_eq!(black.iter().map(|item| item.option.unwrap()).collect::<Vec<_>>(), vec![0, 1, 4]);
    let mut white = entries(&model);
    assert_eq!(filter(&model, row, false, &mut white, Some(a)), 2);
    assert_eq!(white.iter().map(|item| item.option.unwrap()).collect::<Vec<_>>(), vec![0, 2, 3]);
    let mut installed_white = model.clone();
    installed_white.draft.parts.decals[0][0] = Some(103);
    assert!(!initial_black(&installed_white, row));
    assert_eq!(brand(&installed_white, row, 3), Some(a));
}

#[test]
fn decal_filter_keeps_removal_and_falls_back_when_opposite_color_is_missing_for_body() {
    let mut model = fixture();
    let row = Row::Decal(0, 0);
    // This body accepts Black A, Black B and White A, but has no White B artwork.
    model.cars[0].parts.as_mut().unwrap().decals[0].body_choices.insert(42, [101, 103, 104].into());
    let b = brand(&model, row, 3).unwrap();
    let mut white = entries(&model);
    assert_eq!(filter(&model, row, false, &mut white, Some(b)), 0);
    assert_eq!(white.iter().map(|item| item.option.unwrap()).collect::<Vec<_>>(), vec![0, 2]);
    assert_eq!(brand(&model, row, 0), None);
    let mut removed = entries(&model);
    assert_eq!(filter(&model, row, true, &mut removed, None), 0);
    assert_eq!(removed[0].option, Some(0));
}
