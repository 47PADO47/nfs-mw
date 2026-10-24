//! Check the actual presented tree, including parent visibility and shared footer replacement.

use super::{Harness, Page, Part, Row, model};
use blackbox_feng::{UiTree, fe_hash_upper, runtime::pad};

fn shown(h: &Harness) -> std::sync::Arc<UiTree> {
    h.screens
        .presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard)
        .last()
        .unwrap()
        .tree
        .clone()
}

fn assert_stats(tree: &UiTree, visible: bool) {
    for hash in [0x97f0_9eee, 0x2460_05c0, 0xbc4d_a0cf, 0x82a7_829e, 0xb148_6ba4, 0x3426_555f, 0xb05d_d708] {
        let node = tree.nodes.iter().find(|node| node.name_hash == hash).unwrap();
        let mut ancestry = Vec::new();
        let mut parent = node.parent;
        for _ in 0..tree.nodes.len() {
            let Some(index) = parent else { break };
            let ancestor = &tree.nodes[index];
            ancestry.push((ancestor.name_hash, ancestor.colour, ancestor.visible));
            parent = ancestor.parent;
        }
        assert_eq!(
            node.visible, visible,
            "native stats object {hash:08x}, world colour {:?}, ancestry {ancestry:?}",
            node.world_colour
        );
        if visible {
            assert_eq!(node.world_colour[3], 255, "native stats must finish appearing");
        }
    }
}

fn assert_one_showcase(tree: &UiTree) {
    let count = tree.nodes.iter().filter(|node| node.visible && node.text.as_deref() == Some("Showcase")).count();
    assert_eq!(count, 1, "only the binding-aware native footer replacement should show Showcase");
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_manager_grid_wraps_rows_and_clamps_short_last_row_with_add_tile() {
    let mut model = model();
    model.custom_slots_enabled = true;
    for index in 0..6 {
        let id = format!("saved{index}");
        model.custom_slots.push((id.clone(), format!("Saved {index}")));
        model.custom_slot_cars.insert(id, model.draft.car.clone());
    }
    model.custom_slot = Some("saved0".into());
    let mut h = Harness::open(Page::Manager, model);
    let label =
        |h: &Harness| shown(h).nodes.iter().find(|node| node.name_hash == 0x5e7b_09c9).unwrap().text.clone().unwrap();
    assert_eq!(label(&h), "Saved 0");
    h.press(pad::LEFT);
    assert_eq!(label(&h), "Saved 4", "left wraps inside the first five-wide row");
    h.press(pad::RIGHT);
    assert_eq!(label(&h), "Saved 0");
    h.press(pad::UP);
    assert_eq!(label(&h), "Saved 0", "up stops at the first row");
    h.press(pad::DOWN);
    assert_eq!(label(&h), "Saved 5");
    h.press(pad::LEFT);
    assert_eq!(label(&h), "Add New Car", "short last row includes the Add tile");
    h.press(pad::RIGHT);
    assert_eq!(label(&h), "Saved 5");
    h.press(pad::UP);
    h.press(pad::RIGHT);
    h.press(pad::RIGHT);
    assert_eq!(label(&h), "Saved 2");
    h.press(pad::DOWN);
    assert_eq!(label(&h), "Add New Car", "down clamps a missing column to the last tile");
    h.press(pad::DOWN);
    assert_eq!(label(&h), "Add New Car", "down stops at the final row");
    assert_eq!(h.model.custom_slots.len(), 6);
    assert!(h.saved.is_empty());
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_manager_stats_badge_finish_appearing_and_add_tile_hides_them() {
    let mut model = model();
    model.custom_slots_enabled = true;
    model.custom_slots.push(("saved".into(), "BMW M3 GTR".into()));
    model.custom_slot_cars.insert("saved".into(), model.draft.car.clone());
    model.custom_slot = Some("saved".into());
    model.cars[0].physics_name = "bmwm3gtre46".into();
    model.cars[0].performance_ratings = Some(nfsmw_data::car::physics::ratings::RatingEndpoints {
        stock: [0.4, 0.5, 0.6],
        maximum: [1.0; 3],
        max_levels: [2; 7],
    });
    let mut h = Harness::open(Page::Manager, model);
    let tree = shown(&h);
    assert_stats(&tree, true);
    let badge = tree.nodes.iter().find(|node| node.name_hash == 0xb05d_d708).unwrap();
    let blackbox_feng::NodeKind::Image { texture, .. } = badge.kind else { panic!("model badge must be an image") };
    assert!(
        h.screens.assets().texture(texture).is_some(),
        "model badge must use an installed texture or the native generic fallback"
    );
    assert_one_showcase(&tree);
    assert_eq!(
        tree.nodes.iter().find(|node| node.name_hash == fe_hash_upper("FILLBAR_TOPSPEED")).unwrap().size.x,
        64.0
    );
    h.run(0, 90);
    assert_stats(&shown(&h), true);
    h.press(pad::RIGHT);
    assert_stats(&shown(&h), false);
    h.press(pad::LEFT);
    assert_stats(&shown(&h), true);
    assert_one_showcase(&shown(&h));
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_body_footer_hides_only_its_legacy_console_captions() {
    let h = Harness::open(Page::Choices { row: Row::Part(Part::Body), backroom: false }, model());
    let tree = shown(&h);
    assert_one_showcase(&tree);
    let legacy = tree.nodes.iter().find(|node| node.guid == 0x120275).unwrap();
    assert!(!legacy.visible);
    assert!(tree.draw_order.iter().all(|index| tree.nodes[*index].guid != legacy.guid));
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_cart_footer_has_one_remove_and_clear_all_caption() {
    let mut h = Harness::open(Page::Choices { row: Row::Part(Part::Body), backroom: false }, model());
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    h.ui.show_cart();
    h.flush();
    h.run(0, 90);
    let tree = shown(&h);
    for caption in ["Remove", "Clear All"] {
        let count = tree.nodes.iter().filter(|node| node.visible && node.text.as_deref() == Some(caption)).count();
        assert_eq!(count, 1, "only the binding-aware cart footer should show {caption}");
    }
    assert!(tree.nodes.iter().filter(|node| node.name_hash == 0xe795_717d).all(|node| !node.visible));
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_cart_localizes_stock_vinyl_and_binds_native_check_images() {
    let mut model = model();
    model.cars[0].parts.as_mut().unwrap().choices[Part::Vinyl as usize].push(nfsmw_data::car::customization::Choice {
        hash: 900,
        label: "Native Vinyl".into(),
        language_hash: None,
        carbon: false,
    });
    model.draft.parts.parts[Part::Vinyl as usize] = Some(900);
    model.applied = model.draft.clone();
    let mut h = Harness::open(Page::Main, model);
    h.ui.preview(Row::Part(Part::Vinyl), 0);
    h.ui.add_to_cart();
    h.ui.show_cart();
    h.flush();
    h.run(0, 90);
    let tree = shown(&h);
    let name = tree.nodes.iter().find(|node| node.name_hash == fe_hash_upper("PART_NAME_1")).unwrap();
    let assets = h.screens.assets();
    let expected = format!(
        "{}: {}",
        crate::ui::Text::Game(0xbfa5_2c55).resolve(assets).unwrap(),
        crate::ui::Text::Game(0x60a6_62f5).resolve(assets).unwrap()
    );
    assert_eq!(name.text.as_deref(), Some(expected.as_str()));
    let check = |tree: &UiTree| {
        let node = tree.nodes.iter().find(|node| node.name_hash == fe_hash_upper("CHECK_ICON_1")).unwrap();
        assert!(node.visible);
        let blackbox_feng::NodeKind::Image { texture, .. } = node.kind else { panic!("check must be a native image") };
        texture
    };
    assert_eq!(check(&tree), 0x696a_e039);
    assert!(assets.texture(0x696a_e039).is_some());
    assert!(assets.texture(0xe719_881c).is_some());
    h.press(pad::BUTTON0 << 4);
    assert_eq!(check(&shown(&h)), 0xe719_881c);
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_choice_captions_use_language_labels_and_preserve_mod_fallbacks() {
    let mut model = model();
    let parts = model.cars[0].parts.as_mut().unwrap();
    parts.choices[Part::Hood as usize][0].language_hash = Some(0x6134_c218);
    let mut h = Harness::open(Page::Choices { row: Row::Part(Part::Hood), backroom: false }, model.clone());
    let caption =
        |h: &Harness, hash| shown(h).nodes.iter().find(|node| node.name_hash == hash).unwrap().text.clone().unwrap();
    let stock = crate::ui::Text::Game(0x60a6_62f5).resolve(h.screens.assets()).unwrap();
    assert_eq!(caption(&h, 0x5e7b_09c9), stock);
    assert!(
        shown(&h).nodes.iter().all(|node| node.name_hash != 0x0dfb_7a2e),
        "the native PC parts package has one caption, without the Manager's separate shadow"
    );
    h.press(pad::RIGHT);
    let localized = crate::ui::Text::Game(0x6134_c218).resolve(h.screens.assets()).unwrap();
    assert_eq!(caption(&h, 0x5e7b_09c9), localized);
    h.run(0, 60);
    assert_eq!(caption(&h, 0x5e7b_09c9), localized, "detail refresh must not restore a diagnostic caption");
    model.cars[0].parts.as_mut().unwrap().choices[Part::Hood as usize][0].language_hash = Some(0xdeaf_beef);
    model.cars[0].parts.as_mut().unwrap().choices[Part::Hood as usize][0].label = "Custom mod hood".into();
    let mut h = Harness::open(Page::Choices { row: Row::Part(Part::Hood), backroom: false }, model);
    h.press(pad::RIGHT);
    assert_eq!(caption(&h, 0x5e7b_09c9), "Custom mod hood");
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_rim_caption_uses_native_name_without_its_size_suffix() {
    let mut model = model();
    let parts = model.cars[0].parts.as_mut().unwrap();
    let hash = parts.choices[Part::Wheels as usize][0].hash;
    parts.rim_metadata.insert(hash, (1, 17));
    parts.native_names.insert(hash, ("MODBRAND_STYLE01_17_25".into(), None));
    let h = Harness::open(Page::Rims { brand: 1, radius: 17, backroom: false }, model);
    let tree = shown(&h);
    let caption = tree.nodes.iter().find(|node| node.name_hash == 0x5e7b_09c9).unwrap();
    assert_eq!(caption.text.as_deref(), Some("MODBRAND_STYLE01"));
    assert!(
        tree.nodes.iter().all(|node| node.name_hash != 0x0dfb_7a2e),
        "the original rim package also has one authored caption"
    );
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_decal_labels_follow_direct_location_routes_and_body_filtered_choices() {
    let mut model = model();
    let parts = model.cars[0].parts.as_mut().unwrap();
    parts.stock[Part::Body as usize] = Some(500);
    let zone = &mut parts.decals[0];
    zone.positions = 1;
    zone.body_positions.insert(500, 1);
    zone.body_choices.insert(500, std::collections::HashSet::from([902]));
    zone.choices = [(901, 0x6134_c218), (902, 0x055d_a70c)]
        .map(|(hash, language)| nfsmw_data::car::customization::Choice {
            hash,
            label: format!("part {hash}"),
            language_hash: Some(language),
            carbon: false,
        })
        .to_vec();
    for hash in [901, 902] {
        zone.variants.insert(hash, nfsmw_data::car::customization::DecalVariant { brand: hash, name: hash });
    }
    let mut h = Harness::open(Page::DecalZones { backroom: false }, model);
    let caption =
        |h: &Harness| shown(h).nodes.iter().find(|node| node.name_hash == 0x5e7b_09c9).unwrap().text.clone().unwrap();
    let windshield = crate::ui::Text::Game(0x301d_edd3).resolve(h.screens.assets()).unwrap();
    assert_eq!(caption(&h), windshield, "a direct texture-choice route still shows its location name");
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "Decals.fng", "single non-door positions bypass the slot selector");
    h.press(pad::RIGHT);
    assert_eq!(h.model.draft.parts.decals[0][0], Some(902));
    let selected = crate::ui::Text::Game(0x055d_a70c).resolve(h.screens.assets()).unwrap();
    assert_eq!(caption(&h), selected, "the caption must describe the body-compatible selected part");
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_add_car_starts_stock_and_cancel_restores_fitted_record_and_preview() {
    let mut model = model();
    model.custom_slots_enabled = true;
    model.custom_slots = vec![("fitted".into(), "Fitted BMW".into())];
    model.custom_slot_cars.insert("fitted".into(), model.draft.car.clone());
    model.custom_slot = Some("fitted".into());
    model.draft.parts.parts[Part::Body as usize] = Some(100);
    model.draft.color = Some([20, 40, 80]);
    model.draft.performance.levels.fill(1);
    model.draft.tuning.profiles[0][0] = 2;
    model.applied = model.draft.clone();
    let fitted = model.draft.clone();
    let mut other = model.cars[0].clone();
    other.folder = "SKYLINE".into();
    other.name = "Skyline".into();
    model.cars.push(other);
    let stock = crate::garage::Selection { car: fitted.car.clone(), ..Default::default() };
    let mut h = Harness::open(Page::Manager, model);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "Car_Select.fng");
    assert_eq!(h.model.draft, stock, "the initially selected car previews stock before navigation");
    assert_eq!(h.slot_selections["fitted"], fitted, "preview cannot mutate the existing record");
    assert_eq!(h.model.custom_slots.len(), 1);
    assert!(h.saved.is_empty());
    h.press(pad::RIGHT);
    assert_eq!(h.model.draft.car, "SKYLINE");
    assert!(h.model.custom_slot.is_none());
    h.press(pad::BACK);
    assert_eq!(h.top(), "MyCarsManager.fng");
    assert_eq!(h.model.custom_slot.as_deref(), Some("fitted"));
    assert_eq!(h.model.draft, fitted);
    assert_eq!(h.ui.0.lock().unwrap().cart, fitted);
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    assert_eq!(h.model.draft, stock);
    h.press(pad::ACCEPT);
    assert_eq!(h.top(), "CustomizeMain.fng");
    assert_eq!(h.model.custom_slots.len(), 2);
    let added = h.model.custom_slot.as_ref().unwrap();
    assert_ne!(added, "fitted");
    assert_eq!(h.slot_selections[added], stock, "confirmation creates an independent stock record");
    assert_eq!(h.slot_selections["fitted"], fitted);
}
