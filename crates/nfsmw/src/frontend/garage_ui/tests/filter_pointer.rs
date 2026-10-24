//! The lower native filter arrows operate independently of the top part carousel.

use super::{Harness, Page, Part, model};
use crate::ui::Text;
use blackbox_feng::runtime::pad;
use nfsmw_data::car::customization::Choice;

fn choice(hash: u32) -> Choice {
    Choice { hash, label: format!("Part {hash}"), language_hash: None, carbon: false }
}

fn filter_art(h: &Harness, names: &[u32]) {
    let tree = h
        .screens
        .presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard)
        .last()
        .unwrap()
        .tree
        .clone();
    for name in names {
        let at = tree.nodes.iter().position(|node| node.name_hash == *name).unwrap();
        let node = &tree.nodes[at];
        assert!(
            node.visible && node.world_colour[3] > 0,
            "authored filter artwork {name:08x} must return after Showcase"
        );
        let artwork: Vec<_> = tree
            .nodes
            .iter()
            .filter(|node| node.parent == Some(at) && matches!(node.name_hash, 0xc6af_dd7e | 0xd2c2_1a9c))
            .collect();
        assert_eq!(artwork.len(), 2, "the native filter has a persistent frame and stripe");
        for node in artwork {
            assert!(
                node.visible && node.world_colour[3] > 0,
                "actual filter artwork {:08x} must return",
                node.name_hash
            );
        }
    }
    if let Some(flash) = tree.nodes.iter().find(|node| node.name_hash == 0x2c3c_c2d3) {
        assert_eq!(flash.world_colour[3], 0, "the authored entrance flash ends at alpha zero");
        assert!(!flash.visible);
    }
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_rim_pointer_arrows_change_size_and_preserve_style_in_both_directions() {
    let mut model = model();
    let catalog = model.cars[0].parts.as_mut().unwrap();
    catalog.choices[Part::Wheels as usize] = (3000..3004).map(choice).collect();
    for index in 0..4 {
        catalog.rim_metadata.insert(3000 + index, (42, 17 + index / 2));
    }
    model.draft.parts.parts[Part::Wheels as usize] = Some(3001);
    let mut h = Harness::open(Page::Rims { brand: 42, radius: 17, backroom: false }, model);
    h.press(pad::BUTTON0);
    h.press(pad::BACK);
    filter_art(&h, &[0x4bf6_87bf]);
    h.click_named(0x7935_4351);
    assert_eq!(h.model.draft.parts.parts[Part::Wheels as usize], Some(3003));
    h.click_named(0x6b36_4f8b);
    assert_eq!(h.model.draft.parts.parts[Part::Wheels as usize], Some(3001));
    assert!(h.saved.is_empty());
}

fn spoiler_group(h: &Harness, group: usize) {
    let tree = h
        .screens
        .presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard)
        .last()
        .unwrap()
        .tree
        .clone();
    let caption = tree.nodes.iter().find(|node| node.name_hash == 0x7800_8599).unwrap();
    assert!(caption.visible && caption.world_colour[3] > 0);
    let label = [0x01f0_e2b2, 0x0205_b328, 0x0991_2746, 0x0e74_16fc][group];
    assert_eq!(caption.text, Text::Game(label).resolve(h.screens.assets()));
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_spoiler_pointer_arrows_wrap_groups_and_restore_each_group_cursor() {
    let mut model = model();
    let catalog = model.cars[0].parts.as_mut().unwrap();
    catalog.choices[Part::Spoiler as usize] = (4000..4008).map(choice).collect();
    for index in 0..8 {
        catalog.part_groups.insert(4000 + index, (index / 2) as u8);
    }
    model.draft.parts.parts[Part::Spoiler as usize] = Some(4001);
    let mut h = Harness::open(Page::Spoilers { group: 0, backroom: false }, model);
    h.press(pad::BUTTON0);
    h.press(pad::BACK);
    filter_art(&h, &[0x5363_9a10]);
    spoiler_group(&h, 0);
    h.click_named(0x7935_4351);
    spoiler_group(&h, 1);
    h.press(pad::RIGHT);
    h.press(pad::RIGHT);
    assert_eq!(h.model.draft.parts.parts[Part::Spoiler as usize], Some(4003));
    h.click_named(0x6b36_4f8b);
    spoiler_group(&h, 0);
    assert_eq!(h.model.draft.parts.parts[Part::Spoiler as usize], Some(4001));
    h.click_named(0x6b36_4f8b);
    spoiler_group(&h, 3);
    h.click_named(0x7935_4351);
    spoiler_group(&h, 0);
    assert_eq!(h.model.draft.parts.parts[Part::Spoiler as usize], Some(4001));
    assert!(h.saved.is_empty());
}
