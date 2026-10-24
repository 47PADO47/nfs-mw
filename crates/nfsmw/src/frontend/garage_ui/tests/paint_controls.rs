//! Rim paint keeps its native fixed palette independent of body-paint finishes.

use super::{Harness, Page, Part, model};
use blackbox_feng::runtime::pad;
use nfsmw_data::car::customization::{Choice, ColorChoice};

fn fixed_rim_palette() -> crate::garage::Model {
    let mut model = model();
    let catalog = model.cars[0].parts.as_mut().unwrap();
    let wheel = catalog.choices[Part::Wheels as usize][0].hash;
    catalog.paintable_wheels.insert(wheel);
    model.draft.parts.parts[Part::Wheels as usize] = Some(wheel);
    catalog.finishes = vec![("Gloss".into(), 10), ("Pearl".into(), 11)];
    catalog.paint_families.extend([(10, "GLOSS".into()), (11, "PEARL".into()), (20, "SILVER".into())]);
    catalog.stock[Part::RimPaint as usize] = Some(20);
    catalog.colors.extend([(20, [160; 3]), (21, [255, 0, 0]), (22, [0, 0, 255])]);
    catalog.choices[Part::RimPaint as usize] = [21, 22]
        .into_iter()
        .map(|hash| Choice { hash, label: format!("Rim {hash}"), language_hash: None, carbon: false })
        .collect();
    model.applied = model.draft.clone();
    model
}

fn check_controls(h: &Harness) {
    let shown = h.screens.presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard);
    let tree = &shown.last().unwrap().tree;
    for hash in [0x2c52_6172, 0x7800_8599] {
        let control = tree.nodes.iter().find(|node| node.name_hash == hash).unwrap();
        assert!(!control.visible, "rim-paint finish control {hash:08x} must stay hidden");
    }
    let count = tree.nodes.iter().find(|node| node.name_hash == 0x2e48_1779).unwrap();
    assert_eq!(count.text.as_deref(), Some("3"), "stock rim color must not depend on body finish");
}

fn finish_label(h: &Harness) -> String {
    let shown = h.screens.presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard);
    let label = shown.last().unwrap().tree.nodes.iter().find(|node| node.name_hash == 0x7800_8599).unwrap();
    assert!(label.visible && label.world_colour[3] > 0, "the current body finish must remain visible");
    label.text.clone().unwrap()
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_rim_paint_has_one_fixed_palette_without_finish_controls() {
    for backroom in [false, true] {
        let mut h = Harness::open(Page::Paint { color: 1, backroom }, fixed_rim_palette());
        check_controls(&h);
        let entry = h.model.draft.clone();
        let events = h.events.len();
        h.press(pad::RTRIGGER);
        h.press(pad::LTRIGGER);
        assert_eq!(h.model.draft, entry);
        assert_eq!(h.events.len(), events, "filter triggers cannot create rim-paint previews");
        h.press(pad::RIGHT);
        assert_eq!(h.model.draft.parts.parts[Part::RimPaint as usize], Some(21));
        let chosen = h.model.draft.clone();
        h.press(pad::RTRIGGER);
        assert_eq!(h.model.draft, chosen);
        h.press(pad::BUTTON0);
        h.press(pad::BACK);
        check_controls(&h);
        h.press(pad::BACK);
        assert_eq!(h.model.draft, entry);
        assert!(h.saved.is_empty());
    }
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_body_paint_remembers_finish_swatches_and_uses_native_filter_pointer_bounds() {
    let mut model = fixed_rim_palette();
    let catalog = model.cars[0].parts.as_mut().unwrap();
    catalog.stock[Part::Paint as usize] = None;
    catalog.choices[Part::Paint as usize] = [1001, 1002, 1101, 1102]
        .into_iter()
        .map(|hash| Choice { hash, label: format!("Paint {hash}"), language_hash: None, carbon: false })
        .collect();
    for (hash, family, rgb) in [
        (1001, "GLOSS", [20, 40, 60]),
        (1002, "GLOSS", [40, 60, 80]),
        (1101, "PEARL", [80, 100, 120]),
        (1102, "PEARL", [100, 120, 140]),
    ] {
        catalog.paint_families.insert(hash, family.into());
        catalog.colors.insert(hash, rgb);
        let label = if family == "GLOSS" { 0xb676_3cde } else { 0xb715_070a };
        catalog.paint_names.insert(hash, (label, 0));
    }
    let mut h = Harness::open(Page::Paint { color: 0, backroom: false }, model);
    assert_eq!(finish_label(&h), "Gloss");
    h.press(pad::RIGHT);
    assert_eq!(h.model.draft.parts.parts[Part::Paint as usize], Some(1002));
    h.press(pad::RTRIGGER);
    assert_eq!(finish_label(&h), "Custom");
    h.press(pad::RIGHT);
    assert_eq!(h.model.draft.parts.parts[Part::Paint as usize], Some(1102));
    h.press(pad::LTRIGGER);
    assert_eq!(finish_label(&h), "Gloss");
    assert_eq!(h.model.draft.parts.parts[Part::Paint as usize], Some(1002));
    h.press(pad::BUTTON0);
    h.press(pad::BACK);
    assert_eq!(finish_label(&h), "Gloss");
    h.click_named(0x7935_4351);
    assert_eq!(h.model.draft.parts.parts[Part::Paint as usize], Some(1102));
    h.click_named(0x6b36_4f8b);
    assert_eq!(h.model.draft.parts.parts[Part::Paint as usize], Some(1002));
    h.press(pad::RIGHT);
    assert_eq!(h.model.draft.parts.parts[Part::Paint as usize], Some(1001), "a partial row wraps within itself");
    h.press(pad::LEFT);
    assert_eq!(h.model.draft.parts.parts[Part::Paint as usize], Some(1002));
    assert!(h.saved.is_empty());
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_gauge_color_list_wraps_and_retains_each_layer_cursor() {
    let mut model = model();
    let catalog = model.cars[0].parts.as_mut().unwrap();
    catalog.gauge_skins.push(1);
    for (channel, palette) in catalog.color_palettes.iter_mut().enumerate() {
        *palette = (0..2)
            .map(|index| ColorChoice {
                hash: 100 + channel as u32 * 10 + index,
                rgb: [40 + index as u8 * 40, 80, 120],
                upgrade_level: 0,
            })
            .collect();
    }
    model.draft.gauges.skin = 1;
    let mut h = Harness::open(Page::GaugeColors { backroom: false }, model);
    h.press(pad::UP);
    assert_eq!(h.model.draft.gauges.color_parts[1], Some(141));
    h.press(pad::DOWN);
    assert_eq!(h.model.draft.gauges.color_parts[1], Some(140));
    h.press(pad::UP);
    h.press(pad::RIGHT);
    h.press(pad::LEFT);
    assert_eq!(h.model.draft.gauges.color_parts[1], Some(141));
    h.press(pad::BUTTON0);
    h.press(pad::BACK);
    for (arrows, expected) in [([0x611d_6eae, 0xc6ea_c384], 140), ([0x204b_882c, 0x0603_c1d1], 141)] {
        let name = h
            .screens
            .presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard)
            .last()
            .unwrap()
            .tree
            .nodes
            .iter()
            .find(|node| arrows.contains(&node.name_hash) && node.visible)
            .unwrap()
            .name_hash;
        h.click_named(name);
        assert_eq!(h.model.draft.gauges.color_parts[1], Some(expected));
    }
    assert!(h.saved.is_empty());
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_paint_horizontal_navigation_wraps_each_native_twenty_column_row() {
    let mut model = model();
    let catalog = model.cars[0].parts.as_mut().unwrap();
    catalog.stock[Part::Paint as usize] = None;
    catalog.finishes = vec![("Gloss".into(), 2000)];
    catalog.choices[Part::Paint as usize] =
        (2000..2023).map(|hash| Choice { hash, label: hash.to_string(), language_hash: None, carbon: false }).collect();
    for hash in 2000..2023 {
        catalog.paint_families.insert(hash, "GLOSS".into());
        catalog.colors.insert(hash, [hash as u8, 80, 120]);
    }
    let mut h = Harness::open(Page::Paint { color: 0, backroom: false }, model);
    for (button, expected) in [
        (pad::LEFT, 2019),
        (pad::DOWN, 2022),
        (pad::DOWN, 2022),
        (pad::UP, 2002),
        (pad::LEFT, 2001),
        (pad::LEFT, 2000),
        (pad::DOWN, 2020),
        (pad::LEFT, 2022),
        (pad::RIGHT, 2020),
        (pad::UP, 2000),
    ] {
        h.press(button);
        assert_eq!(h.model.draft.parts.parts[Part::Paint as usize], Some(expected));
    }
}
