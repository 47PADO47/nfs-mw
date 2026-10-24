//! Native specialized widgets, including multi-part atomic selections.

use super::{Harness, Page, Part, Row, model};
use blackbox_feng::{fe_hash_upper, runtime::pad};
use nfsmw_data::car::customization::{Choice, ColorChoice};

fn colors() -> crate::garage::Model {
    let mut model = model();
    let catalog = model.cars[0].parts.as_mut().unwrap();
    catalog.choices[Part::Vinyl as usize].push(Choice {
        hash: 900,
        label: "Flames 1".into(),
        language_hash: None,
        carbon: false,
    });
    let vinyl = catalog.choices[Part::Vinyl as usize][0].hash;
    catalog.vinyl_remaps.insert(vinyl, 3);
    catalog.gauge_skins.push(1);
    for channel in 0..6 {
        catalog.color_palettes[channel] = (0..2)
            .map(|index| ColorChoice {
                hash: 1000 + channel as u32 * 10 + index,
                rgb: [channel as u8 * 20, 80, 160],
                upgrade_level: 0,
            })
            .collect();
    }
    model
}

fn text(h: &Harness, hash: u32) -> Option<String> {
    h.screens
        .presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard)
        .last()?
        .tree
        .nodes
        .iter()
        .find(|node| node.name_hash == hash)?
        .text
        .clone()
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_vinyl_remaps_keep_native_identity_and_commit_all_channels_atomically() {
    let mut h = Harness::open(Page::Vinyls { group: 0, backroom: false }, colors());
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "Paint.fng");
    assert_eq!(h.ui.0.lock().unwrap().cart.parts.parts[Part::Vinyl as usize], None);
    h.press(pad::RIGHT);
    assert_eq!(h.model.draft.parts.vinyl_color_parts[0], Some(1001), "identical RGB does not collapse native identity");
    h.press(pad::RTRIGGER);
    h.press(pad::RIGHT);
    h.press(pad::RTRIGGER);
    h.press(pad::RIGHT);
    h.press(pad::BACK);
    assert_eq!(h.top(), "CustomizeParts.fng");
    assert_eq!(
        h.ui.0.lock().unwrap().cart.parts.parts[Part::Vinyl as usize],
        None,
        "cancelling colors cannot add vinyl"
    );
    h.press(pad::ACCEPT);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    let cart = h.ui.0.lock().unwrap().cart.clone();
    assert!(cart.parts.parts[Part::Vinyl as usize].is_some());
    assert_eq!(cart.parts.vinyl_color_parts, [Some(1001), Some(1010), Some(1020)]);
    assert!(h.saved.is_empty());
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_gauge_colors_use_the_native_layer_grid_and_cancel_without_adding_style() {
    let mut h = Harness::open(Page::Choices { row: Row::Gauge, backroom: false }, colors());
    h.press(pad::RIGHT);
    assert_eq!(h.model.draft.gauges.skin, 1, "native gauge carousel previews selected skin");
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "CustomHUDColor.fng");
    assert_eq!(h.ui.0.lock().unwrap().cart.gauges.skin, 0);
    h.press(pad::DOWN);
    assert_eq!(h.model.draft.gauges.color_parts[1], Some(1041), "first layer edits needle");
    h.press(pad::RIGHT);
    h.press(pad::DOWN);
    assert_eq!(h.model.draft.gauges.color_parts[2], Some(1051), "second layer edits characters");
    h.press(pad::BACK);
    assert_eq!(h.top(), "CustomHUD.fng");
    assert_eq!(h.model.draft.gauges.skin, 1, "cancelled colors return to selected style");
    assert_eq!(h.ui.0.lock().unwrap().cart.gauges.skin, 0);
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "CustomHUDColor.fng", "the same style can reopen its native color grid");
    h.press(pad::ACCEPT);
    let cart = h.ui.0.lock().unwrap().cart.clone();
    assert_eq!(cart.gauges.skin, 1);
    assert_eq!(cart.gauges.color_parts, [Some(1030), Some(1040), Some(1050)]);
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_rim_size_keeps_selected_style_and_uses_native_diameter_caption() {
    let mut model = model();
    let catalog = model.cars[0].parts.as_mut().unwrap();
    catalog.choices[Part::Wheels as usize] = ["A17", "B17", "A18", "B18"]
        .into_iter()
        .enumerate()
        .map(|(index, label)| Choice {
            hash: 3000 + index as u32,
            label: label.into(),
            language_hash: None,
            carbon: false,
        })
        .collect();
    for index in 0..4 {
        catalog.rim_metadata.insert(3000 + index, (42, 17 + index / 2));
    }
    model.draft.parts.parts[Part::Wheels as usize] = Some(3001);
    let mut h = Harness::open(Page::Rims { brand: 42, radius: 17, backroom: false }, model);
    h.press(pad::RTRIGGER);
    assert_eq!(h.model.draft.parts.parts[Part::Wheels as usize], Some(3003));
    let pages = h.screens.presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard);
    assert!(pages.last().unwrap().tree.nodes.iter().any(|node| node.text.as_deref() == Some("18\"")));
    h.press(pad::LTRIGGER);
    assert_eq!(h.model.draft.parts.parts[Part::Wheels as usize], Some(3001));
    assert!(h.saved.is_empty());
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_numbers_edit_and_remove_both_door_pairs_as_one_cart_item() {
    let mut model = model();
    let catalog = model.cars[0].parts.as_mut().unwrap();
    catalog.stock[Part::Body as usize] = Some(1);
    for zone in [2, 3] {
        catalog.decals[zone].body_positions.insert(1, 6);
        catalog.decals[zone].body_number_positions.insert(1, [true; 2]);
    }
    for digit in 0..2 {
        catalog.number_choices[digit] = (0..10)
            .map(|value| Choice {
                hash: 2000 + digit as u32 * 100 + value,
                label: value.to_string(),
                language_hash: None,
                carbon: false,
            })
            .collect();
    }
    let mut h = Harness::open(Page::Numbers, model);
    h.press(pad::UP);
    h.press(pad::RIGHT);
    h.press(pad::UP);
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "Numbers.fng", "Add stays in Numbers");
    for zone in [2, 3] {
        assert_eq!(h.model.draft.parts.decals[zone][6..8], [Some(2000), Some(2101)]);
    }
    h.press(pad::START);
    assert_eq!(h.top(), "ShoppingCart_QR.fng");
    assert!(text(&h, fe_hash_upper("PART_NAME_1")).unwrap().contains("01"));
    h.press(pad::BUTTON0 << 4);
    for zone in [2, 3] {
        assert_eq!(h.model.draft.parts.decals[zone][6..8], [None; 2]);
    }
    h.press(pad::BACK);
    h.press(pad::BUTTON0 << 4);
    assert_eq!(h.ui.0.lock().unwrap().cart.parts.decals[3][6..8], [None; 2]);
    assert!(h.saved.is_empty());
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_tuning_help_and_discard_use_native_packages_and_do_not_save() {
    let mut model = model();
    model.draft.performance.levels.fill(1);
    let mut h = Harness::open(Page::Tuning, model);
    h.press(pad::BUTTON0 << 5);
    assert!(!text(&h, 0x46be_6be4).unwrap_or_default().is_empty());
    assert!(!text(&h, 0x46be_6be4).unwrap().contains("jubberish"));
    h.press(pad::BACK);
    assert_eq!(h.top(), "Pause_Performance_Tuning.fng");
    h.press(pad::RIGHT);
    assert_eq!(h.model.draft.tuning.profiles[0][0], 1);
    h.press(pad::BACK);
    assert_eq!(h.top(), "InGameDialog.fng");
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "Pause_Performance_Tuning.fng", "discard defaults to No");
    h.press(pad::BACK);
    h.press(pad::LEFT);
    h.press(pad::ACCEPT);
    assert!(h.events.iter().any(|event| matches!(event, crate::garage::MenuEvent::Cancel)));
    assert!(h.saved.is_empty());
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_footer_shows_rebound_keycaps_and_explicit_cart_shortcut() {
    let h = Harness::open(Page::Choices { row: Row::Part(Part::Body), backroom: false }, model());
    let pages = h.screens.presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard);
    let nodes = &pages.last().unwrap().tree.nodes;
    assert!(
        nodes.iter().any(|node| node.visible && node.text.as_deref() == Some("4")),
        "View Cart has a visible keyboard keycap"
    );
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_insufficient_funds_uses_native_alert_and_preserves_the_cart_for_retry() {
    let mut h = Harness::open(Page::Main, model());
    h.ui.preview(Row::Part(Part::Body), 1);
    h.ui.add_to_cart();
    h.flush();
    h.model.shop = true;
    h.model.shop_cash = Some(0);
    h.model.shop_total = Some(100);
    h.model.shop_error = Some("Not enough cash".into());
    h.ui.sync(&h.model);
    h.ui.show_cart();
    h.flush();
    h.run(0, 90);
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "Dialog.fng");
    let question = text(&h, crate::frontend::confirm::MESSAGE).unwrap();
    assert!(!question.is_empty());
    assert_eq!(text(&h, crate::frontend::confirm::YES_BUTTON).as_deref(), Some("Continue"));
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "ShoppingCart.fng");
    assert_eq!(h.ui.0.lock().unwrap().cart.parts.parts[Part::Body as usize], Some(100));
    assert!(h.saved.is_empty());
    h.model.shop_cash = Some(100);
    h.model.shop_error = None;
    h.ui.sync(&h.model);
    h.press(pad::ACCEPT);
    h.press(pad::LEFT);
    h.press(pad::ACCEPT);
    assert_eq!(h.saved.len(), 1);
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_failed_tuning_save_can_retry_or_discard_after_the_native_error_alert() {
    for retry in [true, false] {
        let mut model = model();
        model.draft.performance.levels.fill(1);
        let mut h = Harness::open(Page::Tuning, model);
        h.press(pad::RIGHT);
        h.fail_save = true;
        h.press(pad::ACCEPT);
        assert_eq!(h.top(), "InGameDialog.fng");
        assert_eq!(text(&h, 0xe2ee_606b).as_deref(), Some("synthetic save failure"));
        h.press(pad::ACCEPT);
        assert_eq!(h.top(), "Pause_Performance_Tuning.fng");
        if retry {
            h.fail_save = false;
            h.press(pad::ACCEPT);
            assert_eq!(h.saved.len(), 1);
            assert_eq!(h.saved[0].tuning.profiles[0][0], 1);
            continue;
        }
        h.press(pad::BACK);
        h.press(pad::LEFT);
        h.press(pad::ACCEPT);
        assert!(h.saved.is_empty());
        assert!(h.events.iter().any(|event| matches!(event, crate::garage::MenuEvent::Cancel)));
    }
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_failed_slot_write_stays_on_car_selection_and_allows_retry() {
    let mut model = model();
    model.custom_slots_enabled = true;
    let mut h = Harness::open(Page::Manager, model);
    h.press(pad::ACCEPT);
    h.fail_slot = true;
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "Car_Select.fng");
    assert!(h.model.custom_slots.is_empty());
    h.fail_slot = false;
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "CustomizeMain.fng");
    assert_eq!(h.model.custom_slots.len(), 1);
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_zero_markers_prevent_entry_but_do_not_block_return_from_backroom() {
    let mut model = model();
    model.shop = true;
    let mut h = Harness::open(Page::Main, model);
    h.press(pad::BUTTON0 << 4);
    assert!(!h.model.backroom);
    assert_eq!(h.top(), "CustomizeMain.fng");
    h.model.shop_marker_counts[8] = 1;
    h.ui.sync(&h.model);
    h.press(pad::BUTTON0 << 4);
    assert!(h.model.backroom);
    h.model.shop_marker_counts.fill(0);
    h.ui.sync(&h.model);
    h.press(pad::BACK);
    assert!(!h.model.backroom);
    assert_eq!(h.top(), "CustomizeMain.fng");
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_tuning_fills_bind_native_images_and_keep_signed_center_anchors() {
    for value in [-5, -2, 0, 2, 5] {
        let mut model = model();
        model.draft.performance.levels.fill(1);
        model.draft.tuning.profiles[0][0] = value;
        let h = Harness::open(Page::Tuning, model);
        let pages = h.screens.presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard);
        let nodes = &pages.last().unwrap().tree.nodes;
        for (side, direction) in [("LEFT", -1.0), ("RIGHT", 1.0)] {
            let fill_name = fe_hash_upper(&format!("FILLBAR_{side}_SLIDER_1"));
            let base_name = fe_hash_upper(&format!("BASE_{side}_SLIDER_1"));
            let fill = nodes.iter().find(|node| node.name_hash == fill_name).unwrap();
            let base = nodes.iter().find(|node| node.name_hash == base_name).unwrap();
            assert_eq!(base.size.x, direction * 82.0, "native base range stays unchanged");
            let fraction = (value as f32 * direction / 5.0).max(0.0);
            let width = base.size.x * fraction;
            assert!((fill.size.x - width).abs() < 0.0001, "{side} value {value}");
            assert_eq!(fill.size.y, 32.0);
            let anchor = base.local_position.truncate() - base.size.truncate() * 0.5 + glam::Vec2::new(2.0, -12.0);
            assert!((fill.local_position.x - width * 0.5 - anchor.x).abs() < 0.0001);
            assert!((fill.local_position.y - fill.size.y * 0.5 - anchor.y).abs() < 0.0001);
            let blackbox_feng::NodeKind::Image { texture, uv, .. } = &fill.kind else {
                panic!("native fill is an image")
            };
            let texture_width = h.screens.assets().texture_size(*texture).unwrap().0 as f32;
            assert!((uv[2] - width / texture_width).abs() < 0.0001);
            assert_eq!([uv[0], uv[1], uv[3]], [0.0, 0.0, 1.0]);
            assert_eq!(fill.visible, fraction > 0.0);
        }
        assert_eq!(h.model.draft.tuning.profiles[0][0], value, "rendering does not change the stored tuning");
        assert!(
            nodes.iter().all(|node| node.name_hash != fe_hash_upper("OPTION_DATA_1")),
            "the original PC package has no numeric data widget"
        );
    }
}
