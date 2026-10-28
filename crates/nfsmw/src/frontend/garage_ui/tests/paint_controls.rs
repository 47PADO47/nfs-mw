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

#[test]
#[ignore = "requires installed gauge palette package"]
fn batch12_gauge_entry_and_channel_changes_preserve_unedited_colors() {
    let mut initial = model();
    let catalog = initial.cars[0].parts.as_mut().unwrap();
    catalog.gauge_skins.push(1);
    for (channel, palette) in catalog.color_palettes.iter_mut().enumerate() {
        *palette = (0..2)
            .map(|index| ColorChoice {
                hash: 100 + channel as u32 * 10 + index,
                rgb: [20 + channel as u8 * 10, index as u8 * 80, 120],
                upgrade_level: 0,
            })
            .collect();
    }
    initial.draft.gauges.skin = 1;
    let before = initial.draft.gauges.clone();
    let mut h = Harness::open(Page::GaugeColors { backroom: false }, initial);
    assert_eq!(h.model.draft.gauges, before, "opening colors cannot apply all first swatches");
    for button in [pad::RIGHT, pad::RIGHT, pad::LEFT, pad::LEFT] {
        h.press(button);
        assert_eq!(h.model.draft.gauges, before, "selecting a channel only changes focus");
    }
    h.press(pad::DOWN);
    assert_eq!(h.model.draft.gauges.backing, before.backing);
    assert_eq!(h.model.draft.gauges.digits, before.digits);
    assert_eq!(h.model.draft.gauges.color_parts, [None, Some(141), None]);
    let tree = h.screens.trees().pop().unwrap();
    let gear = tree.nodes.iter().find(|n| n.name_hash == 0xb845_89be).unwrap();
    assert!(gear.visible && gear.world_colour[3] > 0, "the native gear sample remains visible");
    let characters_before = gear.colour;
    h.press(pad::RIGHT);
    h.press(pad::DOWN);
    let tree = h.screens.trees().pop().unwrap();
    let gear = tree.nodes.iter().find(|n| n.name_hash == 0xb845_89be).unwrap();
    assert_ne!(gear.colour[..3], characters_before[..3], "Characters updates the gear sample too");
    assert_eq!(h.model.draft.gauges.color_parts, [None, Some(141), Some(151)]);
    assert!(h.saved.is_empty());
}

#[test]
#[ignore = "requires installed native garage focus scripts"]
fn batch12_garage_idle_focus_clocks_advance_and_new_selection_restarts_its_own_script() {
    use crate::frontend::{garage_ui::GarageUi, logic::Cx};
    use crate::settings::{Partial, Settings, test_caps};
    use crate::ui::{Catalog, SCREEN_FILES, UiAssets};
    use blackbox_feng::ids::PAD_RIGHT;
    use game_install::GameDir;

    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let assets = UiAssets::load(&dir).unwrap();
    let catalog = Catalog::load(&dir, &SCREEN_FILES);
    let mut native_tracks = 0;
    for page in [Page::Paint { color: 1, backroom: false }, Page::Numbers, Page::Manager] {
        let mut initial = fixed_rim_palette();
        initial.custom_slots = vec![("one".into(), "First car".into()), ("two".into(), "Second car".into())];
        initial.custom_slot_cars =
            initial.custom_slots.iter().map(|(id, _)| (id.clone(), initial.draft.car.clone())).collect();
        let ui = GarageUi::new();
        ui.sync(&initial);
        let mut logic = ui.logic(page);
        let mut runtime = crate::ui::runtime();
        let package = runtime.load(catalog.find(page.package()).unwrap().clone());
        runtime.update(2.0);
        let mut settings = Settings::from(Partial::default());
        let mut changed = Partial::default();
        let mut commands = Vec::new();
        let mut memory = Default::default();
        let mut cx = Cx {
            rt: &mut runtime,
            package,
            assets: &assets,
            settings: &mut settings,
            changed: &mut changed,
            caps: test_caps::native(),
            commands: &mut commands,
            memory: &mut memory,
            key_texts: &[],
            world_map: None,
            mods: None,
            name: page.package(),
        };
        logic.start(&mut cx);
        cx.rt.update(1.0 / 60.0);
        let first = cx.rt.current_button(package).expect("native focused garage control");
        cx.rt.take_outgoing();
        for _ in 0..5 {
            logic.tick(&mut cx, 1.0 / 60.0);
            cx.rt.update(1.0 / 60.0);
        }
        assert_eq!(cx.rt.current_button(package), Some(first));
        assert!(
            !cx.rt.take_outgoing().iter().any(|event| matches!(
                event,
                blackbox_feng::runtime::Outgoing::Sound { message: 0xabc0_8912 | 0x55d1_e635, .. }
            )),
            "{page:?}: idle refresh must not emit repeated gain/lose-focus requests"
        );
        let state = cx.rt.object(first).unwrap();
        assert!(state.time > 0, "{page:?}: idle refresh must not restart native focus at tick zero");
        let first_authored = state.scripts.iter().any(|script| script.id == crate::frontend::ids::SCRIPT_HIGHLIGHT);
        if first_authored {
            native_tracks += 1;
            assert_eq!(cx.rt.script_of(first), Some(crate::frontend::ids::SCRIPT_HIGHLIGHT));
        }
        logic.message(&mut cx, PAD_RIGHT);
        let second = cx.rt.current_button(package).unwrap();
        assert_ne!(second, first, "{page:?}: a new selection still changes native focus");
        let second_authored = cx
            .rt
            .object(second)
            .unwrap()
            .scripts
            .iter()
            .any(|script| script.id == crate::frontend::ids::SCRIPT_HIGHLIGHT);
        if second_authored {
            assert_eq!(cx.rt.script_of(second), Some(crate::frontend::ids::SCRIPT_HIGHLIGHT));
            assert_eq!(cx.rt.object(second).unwrap().time, 0, "{page:?}: the new focus starts its own script");
        }
        if first_authored {
            assert_eq!(cx.rt.script_of(first), Some(crate::frontend::ids::SCRIPT_UNHIGHLIGHT));
        }
        cx.rt.update(1.0 / 60.0);
        logic.tick(&mut cx, 1.0 / 60.0);
        cx.rt.update(1.0 / 60.0);
        assert!(cx.rt.object(second).unwrap().time > 0, "{page:?}: the new highlight can animate");
    }
    assert!(native_tracks > 0, "the fixture must exercise authored native highlight tracks");
}
