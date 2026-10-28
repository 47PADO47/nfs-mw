//! Normal garage controls against the actual installed packages, without save I/O.

mod add_header;
mod animation_input;
mod decal_filter;
mod filter_pointer;
mod footer_layout;
mod harness;
mod manager_visual;
mod manager_window;
mod paint_controls;
mod return_state;
mod samples;
mod specialized;
mod tuning_help;
mod tuning_prompts;

use super::{Page, Section};
use crate::garage::{MenuEvent, menu_rows::Row};
use blackbox_feng::runtime::pad;
use harness::{Harness, model};
use nfsmw_data::car::customization::Category as Part;

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_packages_consume_held_accept_and_cancel_transient_part_preview() {
    let mut h = Harness::open(Page::Main, model());
    h.run(pad::ACCEPT, 120);
    assert_eq!(h.top(), "CustomizeCategory.fng", "held accept must not select two pages");
    h.run(0, 3);
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "CustomizeParts.fng");
    h.press(pad::RIGHT);
    assert_eq!(h.model.draft.parts.parts[Part::Body as usize], Some(100));
    h.press(pad::BACK);
    assert_eq!(h.top(), "CustomizeCategory.fng");
    assert_eq!(h.model.draft.parts.parts[Part::Body as usize], None);
    assert!(h.saved.is_empty(), "browsing and cancelling must not save");
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_cart_keeps_preview_out_until_added_and_can_uncheck_without_purchase() {
    let mut h = Harness::open(Page::Main, model());
    h.press(pad::ACCEPT);
    h.press(pad::ACCEPT);
    h.press(pad::RIGHT);
    h.press(pad::START);
    assert_eq!(h.top(), "ShoppingCart_QR.fng");
    assert_eq!(h.model.draft.parts.parts[Part::Body as usize], None, "View Cart excludes unadded preview");
    h.press(pad::BACK);
    assert_eq!(h.top(), "CustomizeParts.fng");
    assert_eq!(h.model.draft.parts.parts[Part::Body as usize], Some(100));
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "CustomizeCategory.fng");
    h.press(pad::BACK);
    assert_eq!(h.top(), "ShoppingCart_QR.fng", "Done with active items opens cart");
    h.press(pad::BUTTON0 << 4);
    assert_eq!(h.model.draft.parts.parts[Part::Body as usize], None);
    h.press(pad::BACK);
    assert_eq!(h.top(), "CustomizeCategory.fng");
    h.press(pad::BACK);
    assert_eq!(h.top(), "CustomizeMain.fng");
    h.press(pad::BACK);
    assert!(h.events.iter().any(|event| matches!(event, MenuEvent::Cancel)));
    assert!(h.saved.is_empty());
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_cart_checkout_defaults_to_no_and_retries_failed_save() {
    let mut h = Harness::open(Page::Main, model());
    h.ui.preview(Row::Part(Part::Body), 1);
    h.ui.add_to_cart();
    h.flush();
    h.ui.show_cart();
    h.flush();
    h.run(0, 90);
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "Dialog.fng");
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "ShoppingCart_QR.fng");
    assert!(h.saved.is_empty(), "default No cannot save");
    h.fail_save = true;
    h.press(pad::ACCEPT);
    h.press(pad::LEFT);
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "ShoppingCart_QR.fng");
    assert_eq!(h.model.draft.parts.parts[Part::Body as usize], Some(100));
    assert!(h.saved.is_empty());
    h.fail_save = false;
    h.press(pad::ACCEPT);
    h.press(pad::LEFT);
    h.press(pad::ACCEPT);
    assert_eq!(h.saved.len(), 1);
    assert_eq!(h.saved[0].parts.parts[Part::Body as usize], Some(100));
    assert_eq!(h.top(), "CustomizeMain.fng", "checkout remains in garage");
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_manager_adds_only_after_car_confirmation_and_deletes_only_after_yes() {
    let mut model = model();
    model.custom_slots_enabled = true;
    let mut h = Harness::open(Page::Manager, model);
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "Car_Select.fng");
    assert!(h.model.custom_slots.is_empty(), "opening Add cannot duplicate a car");
    h.press(pad::ACCEPT);
    assert_eq!(h.model.custom_slots.len(), 1);
    assert_eq!(h.top(), "CustomizeMain.fng");
    h.press(pad::BACK);
    assert_eq!(h.top(), "MyCarsManager.fng");
    h.press(pad::BUTTON0 << 5);
    assert_eq!(h.top(), "Dialog.fng");
    h.press(pad::ACCEPT);
    assert_eq!(h.model.custom_slots.len(), 1, "delete defaults to No");
    h.press(pad::BUTTON0 << 5);
    h.press(pad::LEFT);
    h.press(pad::ACCEPT);
    assert!(h.model.custom_slots.is_empty());
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_backroom_keeps_main_then_uses_original_backroom_subcategory() {
    let mut model = model();
    model.shop = true;
    model.lock_car = true;
    model.shop_marker_counts.fill(1);
    for category in &mut model.cars[0].performance.categories {
        category.junkman = true;
        category.stock_present = true;
    }
    let mut h = Harness::open(Page::Main, model);
    h.press(pad::BUTTON0 << 4);
    assert_eq!(h.top(), "CustomizeMain.fng");
    assert!(h.model.backroom);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    assert_eq!(h.top().to_ascii_lowercase(), "customizecategory_backroom.fng");
    h.press(pad::BACK);
    assert_eq!(h.top(), "CustomizeMain.fng");
    h.press(pad::BACK);
    assert!(!h.model.backroom, "Done returns from backroom to front room first");
    assert_eq!(h.top(), "CustomizeMain.fng");
}

#[test]
fn original_performance_categories_use_the_original_order() {
    let entries = super::choices::items(Page::Categories { section: Section::Performance, backroom: false }, &model());
    let names: Vec<_> = entries.iter().map(|entry| entry.label.as_str()).collect();
    assert_eq!(names, ["Engine", "Transmission", "Suspension", "Nitrous", "Tires", "Brakes", "Turbo / supercharger"]);
}

#[test]
fn original_hero_eligibility_hides_hardware_and_rim_paint_even_when_mod_parts_exist() {
    let mut model = model();
    model.cars[0].hero = true;
    let main = super::choices::items(Page::Main, &model);
    assert_eq!(main.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), ["Visual"]);
    let visual = super::choices::items(Page::Categories { section: Section::Visual, backroom: false }, &model);
    assert!(!visual.iter().any(|item| item.label == "Rim Paint"));
    model.cars[0].hero = false;
    assert_eq!(
        super::choices::items(Page::Main, &model).len(),
        3,
        "mod cars retain their own customization eligibility"
    );
}

#[test]
fn original_performance_carousel_has_regular_packages_without_an_equipment_entry() {
    use nfsmw_data::car::physics::PerformanceCategory;
    let entries = super::choices::items(
        Page::Choices { row: Row::Performance(PerformanceCategory::Engine), backroom: false },
        &model(),
    );
    assert_eq!(entries.iter().map(|item| item.option).collect::<Vec<_>>(), [Some(1), Some(2)]);
}

#[test]
fn original_backroom_hides_categories_without_the_matching_marker() {
    let mut model = model();
    model.shop = true;
    model.backroom = true;
    assert!(super::choices::items(Page::Main, &model).is_empty());
    model.shop_marker_counts[8] = 1;
    let main = super::choices::items(Page::Main, &model);
    assert_eq!(main.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), ["Parts"]);
    let parts = super::choices::items(Page::Categories { section: Section::Parts, backroom: true }, &model);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].next, Some(Page::Choices { row: Row::Part(Part::Body), backroom: true }));
    model.shop_marker_counts[2] = 1;
    let car = &mut model.cars[0];
    use nfsmw_data::car::physics::PerformanceCategory;
    car.performance.categories[PerformanceCategory::Engine as usize].junkman = true;
    car.performance.categories[PerformanceCategory::Engine as usize].stock_present = true;
    assert_eq!(super::choices::items(Page::Main, &model).len(), 2);
    let performance = super::choices::items(Page::Categories { section: Section::Performance, backroom: true }, &model);
    assert_eq!(performance.len(), 1);
    assert_eq!(performance[0].label, "Engine");
}

#[test]
fn performance_camera_names_follow_original_categories_independent_of_storage_order() {
    use nfsmw_data::car::physics::PerformanceCategory;
    assert_eq!(
        Page::Choices { row: Row::Performance(PerformanceCategory::Engine), backroom: false }.camera(),
        "customize_customizeperformance_513"
    );
    assert_eq!(
        Page::Choices { row: Row::Performance(PerformanceCategory::Tires), backroom: true }.camera(),
        "customize_customizeperformance_backroom_517"
    );
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_locked_item_can_preview_but_cannot_enter_cart() {
    let mut model = model();
    model.selection_locked = true;
    let mut h = Harness::open(Page::Choices { row: Row::Part(Part::Body), backroom: false }, model);
    h.press(pad::RIGHT);
    assert_eq!(h.model.draft.parts.parts[Part::Body as usize], Some(100));
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "CustomizeParts.fng");
    assert_eq!(h.ui.0.lock().unwrap().cart.parts.parts[Part::Body as usize], None);
    assert!(h.saved.is_empty());
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_full_pages_hide_parents_and_dialogs_keep_the_current_page() {
    let mut h = Harness::open(Page::Main, model());
    h.press(pad::ACCEPT);
    let visible = |h: &Harness| {
        h.screens
            .presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard)
            .into_iter()
            .map(|page| page.name)
            .collect::<Vec<_>>()
    };
    assert_eq!(visible(&h), ["CustomizeCategory.fng"]);
    h.ui.request(super::Request::Push(Page::Checkout { backroom: false, quick: true }));
    h.flush();
    assert_eq!(visible(&h), ["CustomizeCategory.fng", "Dialog.fng"]);
    h.run(0, 90);
    h.press(pad::BACK);
    assert_eq!(visible(&h), ["CustomizeCategory.fng"]);
    h.press(pad::BACK);
    assert_eq!(visible(&h), ["CustomizeMain.fng"]);
}
