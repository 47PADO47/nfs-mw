//! Transaction boundaries and device navigation use synthetic installed catalogues.

use super::{Car, Model, menu, menu_rows::Row};
use nfsmw_data::car::customization::{Catalog, Category, Choice};

fn model() -> Model {
    let mut catalog = Catalog::default();
    catalog.stock[Category::Body as usize] = Some(10);
    catalog.number_choices = [vec![choice(100, "0")], vec![choice(200, "0")]];
    for zone in [2, 3] {
        catalog.decals[zone].body_number_positions.insert(10, [true; 2]);
    }
    let mut model = Model { shop: true, shop_cash: Some(500), shop_total: Some(0), ..Default::default() };
    model.cars.push(Car {
        folder: "CAR".into(),
        name: "Car".into(),
        paintable: true,
        parts: Some(catalog.into()),
        ..Default::default()
    });
    model.select("CAR".into());
    model.applied = model.draft.clone();
    model
}

fn choice(hash: u32, label: &str) -> Choice {
    Choice { hash, label: label.into(), language_hash: None, carbon: false }
}

#[test]
fn arbitrary_rgb_controls_clear_only_the_edited_native_color_identity() {
    let mut model = model();
    model.draft.parts.vinyl_color_parts = [Some(10), Some(11), Some(12)];
    model.draft.gauges.color_parts = [Some(20), Some(21), Some(22)];
    Row::Color(3).set_color(&mut model, Some([1, 2, 3]));
    Row::Color(6).set_color(&mut model, Some([4, 5, 6]));
    assert_eq!(model.draft.parts.vinyl_color_parts, [Some(10), None, Some(12)]);
    assert_eq!(model.draft.gauges.color_parts, [Some(20), None, Some(22)]);
    assert_eq!(model.draft.parts.vinyl_colors[1], Some([1, 2, 3]));
    assert_eq!(model.draft.gauges.needle, Some([4, 5, 6]));
}

#[test]
fn number_choice_adds_and_removes_a_complete_mirrored_pair() {
    let mut model = model();
    Row::Number(0).set(&mut model, 1);
    for zone in [2, 3] {
        assert_eq!(model.draft.parts.decals[zone][6..8], [Some(100), Some(200)]);
    }
    assert_eq!(model.applied.parts.decals[2], [None; 8], "preview must not change installed record");
    Row::Number(1).set(&mut model, 0);
    assert!(model.draft.parts.decals.iter().flatten().all(Option::is_none));
}

#[test]
fn changing_body_removes_only_decal_positions_without_an_authored_surface() {
    let mut model = model();
    let catalog = model.cars[0].parts.as_mut().unwrap();
    catalog.choices[Category::Body as usize] = vec![choice(11, "Kit 1")];
    catalog.decals[0].body_positions.insert(11, 1);
    model.draft.parts.decals[0] = [Some(20); 8];
    Row::Part(Category::Body).set(&mut model, 1);
    assert_eq!(model.draft.parts.decals[0][0], Some(20));
    assert!(model.draft.parts.decals[0][1..].iter().all(Option::is_none));
}

#[test]
fn checkout_requires_a_valid_affordable_quote_and_a_drivable_car() {
    let mut model = model();
    assert!(menu::can_confirm(&model));
    model.shop_total = Some(501);
    assert!(!menu::can_confirm(&model));
    model.shop_total = Some(500);
    assert!(menu::can_confirm(&model));
    model.shop_error = Some("Locked part".into());
    assert!(!menu::can_confirm(&model));
    model.shop_error = None;
    model.cars[0].unavailable = Some("Missing physics".into());
    assert!(!menu::can_confirm(&model));
}

#[test]
fn career_custom_rgb_chooses_an_installed_finish_without_changing_diagnostic_behavior() {
    let mut model = model();
    model.cars[0].parts.as_mut().unwrap().finishes.push(("Gloss".into(), 300));
    Row::Color(0).set_color(&mut model, Some([10, 20, 30]));
    assert_eq!(model.draft.parts.parts[Category::Paint as usize], Some(300));
    model.shop = false;
    model.draft.parts.parts[Category::Paint as usize] = None;
    Row::Color(0).set_color(&mut model, Some([40, 50, 60]));
    assert_eq!(model.draft.parts.parts[Category::Paint as usize], None);
}

#[test]
fn a_failed_save_can_retry_a_valid_cart_without_reediting_parts() {
    let mut model = model();
    model.shop_cash = Some(1000);
    model.shop_total = Some(100);
    model.message = Some(Err("Filesystem was unavailable".into()));
    assert!(menu::can_confirm(&model), "save errors must allow a confirmed transaction to retry");
    model.shop_error = Some("Part is locked".into());
    assert!(!menu::can_confirm(&model), "a refused quote must still block purchase");
}
