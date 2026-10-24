//! Native decal color filtering retains underlying part identities and the selected brand.

use super::{Harness, Page, Part, Row, model};
use crate::garage::Model;
use crate::ui::Text;
use blackbox_feng::runtime::pad;
use blackbox_hash::bstring_hash_continue;
use nfsmw_data::car::customization::{Choice, DecalVariant};

fn fixture(initial: Option<u32>) -> Model {
    let mut model = model();
    let catalog = model.cars[0].parts.as_mut().unwrap();
    catalog.stock[Part::Body as usize] = Some(42);
    let zone = &mut catalog.decals[2];
    zone.body_positions.insert(42, 6);
    for (hash, brand, black) in
        [(101, 900, true), (102, 900, false), (103, 901, true), (104, 901, false), (105, 902, true)]
    {
        zone.choices.push(Choice { hash, label: format!("Native {hash}"), language_hash: None, carbon: false });
        zone.variants.insert(
            hash,
            DecalVariant {
                brand,
                name: match black {
                    true => brand,
                    false => bstring_hash_continue(brand, b"_WHITE"),
                },
            },
        );
    }
    model.draft.parts.decals[2][0] = initial;
    model.applied = model.draft.clone();
    model
}

fn check(h: &Harness, part: Option<u32>, black: bool, count: usize) {
    assert_eq!(h.model.draft.parts.decals[2][0], part);
    let tree = h
        .screens
        .presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard)
        .last()
        .unwrap()
        .tree
        .clone();
    let dir =
        game_install::GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let catalog = crate::ui::Catalog::load(&dir, &crate::ui::SCREEN_FILES);
    let package = catalog.find("Decals.fng").unwrap();
    let is_button = |node: &blackbox_feng::UiNode| {
        package.objects[node.index].flags & blackbox_feng::package::flags::IS_BUTTON != 0
    };
    let captions: Vec<_> =
        tree.nodes.iter().filter(|node| node.name_hash == 0x889b_acb6 && node.visible && !is_button(node)).collect();
    let diagnostics: Vec<_> = tree
        .nodes
        .iter()
        .filter(|node| matches!(node.name_hash, 0x889b_acb6 | 0x5363_9a10))
        .map(|node| (node.guid, node.name_hash, node.visible, node.world_colour, node.text.as_deref()))
        .collect();
    assert_eq!(captions.len(), 1, "exactly one native color caption must be visible: {diagnostics:?}");
    let caption = captions[0];
    assert!(caption.world_colour[3] > 0);
    assert!(caption.position.x.abs() < 320.0 && caption.position.y.abs() < 240.0);
    let artwork: Vec<_> = tree
        .nodes
        .iter()
        .filter(|node| node.parent == caption.parent && matches!(node.name_hash, 0xc6af_dd7e | 0xd2c2_1a9c))
        .collect();
    assert_eq!(artwork.len(), 2, "the native caption has a persistent frame and stripe");
    for node in artwork {
        assert!(
            node.visible && node.world_colour[3] > 0,
            "actual filter artwork {:08x} must remain visible",
            node.name_hash
        );
    }
    let flash = tree.nodes.iter().find(|node| node.name_hash == 0x2c3c_c2d3).unwrap();
    assert_eq!(flash.world_colour[3], 0, "the authored entrance flash ends at alpha zero");
    assert!(!flash.visible);
    let label = match black {
        true => 0x41f0_a3a5,
        false => 0x436a_98e9,
    };
    assert_eq!(caption.text, Text::Game(label).resolve(h.screens.assets()));
    let buttons: Vec<_> = tree.nodes.iter().filter(|node| node.name_hash == 0x889b_acb6 && is_button(node)).collect();
    assert!(!buttons.is_empty(), "the native offscreen Toggle Colour button shares the caption hash");
    for button in buttons {
        assert!(button.parent.is_none() && button.position.y > 240.0, "Toggle Colour stays below the native viewport");
        let authored = &package.objects[button.index];
        assert_ne!(authored.flags & 2, 0, "this native event button uses its literal text, not its language key");
        assert_eq!(
            button.text.as_deref(),
            Some(authored.string.as_ref().unwrap().text.as_str()),
            "caption binding must not rewrite the button"
        );
    }
    let total = tree.nodes.iter().find(|node| node.name_hash == 0x2e48_1779).unwrap();
    assert_eq!(total.text.as_deref(), Some(count.to_string().as_str()));
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_decal_filter_keeps_brand_and_color_across_triggers_pointer_and_showcase() {
    let page = Page::Choices { row: Row::Decal(2, 0), backroom: false };
    let mut h = Harness::open(page, fixture(Some(104)));
    check(&h, Some(104), false, 3);
    h.press(pad::LTRIGGER);
    check(&h, Some(103), true, 4);
    h.press(pad::RTRIGGER);
    check(&h, Some(104), false, 3);
    h.press(pad::BUTTON0);
    assert_eq!(h.top(), "Showcase.fng");
    h.press(pad::BACK);
    check(&h, Some(104), false, 3);
    h.click_named(0x7935_4351);
    check(&h, Some(103), true, 4);
    h.click_named(0x6b36_4f8b);
    check(&h, Some(104), false, 3);
    h.press(pad::LEFT);
    check(&h, Some(102), false, 3);
    h.click_named(0xd2c2_1a9c);
    check(&h, Some(101), true, 4);
    h.press(pad::RIGHT);
    check(&h, Some(103), true, 4);
    h.press(pad::ACCEPT);
    assert_eq!(h.ui.0.lock().unwrap().cart.parts.decals[2][0], Some(103));
    assert!(h.saved.is_empty());
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_decal_filter_defaults_black_and_missing_opposite_color_selects_removal() {
    let page = Page::Choices { row: Row::Decal(2, 0), backroom: false };
    let mut empty = Harness::open(page, fixture(None));
    check(&empty, None, true, 4);
    empty.press(pad::LTRIGGER);
    check(&empty, None, false, 3);
    let mut h = Harness::open(page, fixture(Some(105)));
    check(&h, Some(105), true, 4);
    h.press(pad::RTRIGGER);
    check(&h, None, false, 3);
    h.press(pad::BACK);
    assert_eq!(h.model.draft.parts.decals[2][0], Some(105), "Cancel restores the installed part");
}
