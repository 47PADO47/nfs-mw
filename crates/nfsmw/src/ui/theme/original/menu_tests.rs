//! Tests for the Original menu adapters (`original/menu.rs`). Synthetic menu trees go through the menu cell, the way the
//! frontend plugin fills it. No game data and no GPU are needed.

use std::sync::Arc;

use blackbox_ui::{BuildCx, DrawList, NodeKind, Skin, Theme, UiNode, UiTree};

use crate::ui::theme::ElementId;

use super::{FengSource, FrameCell, OriginalTheme};

/// The menu and career slots, the ones the menu cell feeds (raw 20 and up).
const MENU_SLOTS: [ElementId; 12] = [
    ElementId::MainMenu,
    ElementId::Categories,
    ElementId::PauseMenu,
    ElementId::OptionTable,
    ElementId::TrackSelect,
    ElementId::SafeHouseMenu,
    ElementId::Blacklist,
    ElementId::CarLot,
    ElementId::RapSheet,
    ElementId::RegionUnlock,
    ElementId::MemoryCard,
    ElementId::Sms,
];

fn node(index: usize, name: u32, parent: Option<usize>, kind: NodeKind, z: f32) -> UiNode {
    UiNode { index, name_hash: name, parent, kind, z, visible: true, colour: [255; 4], ..UiNode::default() }
}

fn drawn(node: &UiNode) -> bool {
    node.visible && node.z > 0.0 && !matches!(node.kind, NodeKind::Group | NodeKind::Other)
}

/// A menu tree of three objects: a root group, a text and an image under it. `tag` is the root's name, so two trees
/// built with different tags can be told apart. The children's names are `tag + 1` and `tag + 2`.
fn menu_tree(tag: u32) -> UiTree {
    let text = NodeKind::Text { font: 1, justification: 0, leading: 0, max_width: 0 };
    let image = NodeKind::Image {
        texture: 2,
        uv: [0.0, 0.0, 1.0, 1.0],
        mask: None,
        mask_rotation: [0.5, 0.5, 0.0],
        mask_uv: [0.0, 0.0, 1.0, 1.0],
    };
    let mut list = DrawList::new();
    list.push(node(0, tag, None, NodeKind::Group, 0.0));
    list.push(node(1, tag + 1, Some(0), text, 2.0));
    list.push(node(2, tag + 2, Some(0), image, 1.0));
    list.into_tree(drawn)
}

/// What `slot` emits from `frame`, built the way the presenter's caller builds it.
fn emitted(frame: &FrameCell, slot: ElementId) -> DrawList {
    let theme = OriginalTheme::new(frame.clone());
    let skin = Skin::new();
    let mut out = DrawList::new();
    if let Some(component) = theme.element(slot.slot(), &BuildCx::new(&skin)) {
        component.build(&mut out);
    }
    out
}

fn names(list: &DrawList) -> Vec<u32> {
    list.iter().map(|node| node.name_hash).collect()
}

fn indices(list: &DrawList) -> Vec<usize> {
    list.iter().map(|node| node.index).collect()
}

fn tree_names(tree: &UiTree) -> Vec<u32> {
    tree.nodes.iter().map(|node| node.name_hash).collect()
}

#[test]
fn the_menu_slots_list_every_menu_and_career_slot_of_the_catalogue() {
    let catalogue: Vec<ElementId> = ElementId::ALL.iter().copied().filter(|slot| slot.is_menu()).collect();
    assert_eq!(catalogue.len(), MENU_SLOTS.len());
    for slot in MENU_SLOTS {
        assert!(catalogue.contains(&slot), "{} is not a menu slot", slot.name());
    }
}

#[test]
fn each_menu_slot_emits_the_tree_set_for_it_and_only_that() {
    for (n, slot) in MENU_SLOTS.into_iter().enumerate() {
        let frame = FrameCell::new();
        let tree = menu_tree(0x1000 * (n as u32 + 1));
        frame.set_menu(slot.slot(), Arc::new(tree.clone()));
        let list = emitted(&frame, slot);
        assert_eq!(names(&list), tree_names(&tree), "{} emits its own tree", slot.name());
        assert_eq!(indices(&list), vec![0, 1, 2], "{} keeps the object indices", slot.name());
        for node in list.iter() {
            let original = &tree.nodes[node.index];
            assert_eq!(node.kind, original.kind, "kind of object {} in {}", node.index, slot.name());
            assert_eq!(node.z, original.z, "z of object {} in {}", node.index, slot.name());
        }
        for other in MENU_SLOTS.iter().filter(|other| **other != slot) {
            assert!(emitted(&frame, *other).is_empty(), "{} drew the tree of {}", other.name(), slot.name());
        }
    }
}

#[test]
fn an_empty_menu_slot_emits_nothing() {
    let frame = FrameCell::new();
    for slot in MENU_SLOTS {
        assert!(emitted(&frame, slot).is_empty(), "{} drew with no tree", slot.name());
    }
}

#[test]
fn clearing_a_menu_slot_empties_it_and_leaves_the_others_drawn() {
    let frame = FrameCell::new();
    frame.set_menu(ElementId::MainMenu.slot(), Arc::new(menu_tree(0x1000)));
    frame.set_menu(ElementId::Sms.slot(), Arc::new(menu_tree(0x2000)));
    frame.clear_menu(ElementId::MainMenu.slot());
    assert!(emitted(&frame, ElementId::MainMenu).is_empty(), "a cleared menu slot draws nothing");
    assert_eq!(indices(&emitted(&frame, ElementId::Sms)), vec![0, 1, 2]);
}

#[test]
fn setting_a_menu_slot_again_replaces_its_tree() {
    let frame = FrameCell::new();
    frame.set_menu(ElementId::CarLot.slot(), Arc::new(menu_tree(0x1000)));
    frame.set_menu(ElementId::CarLot.slot(), Arc::new(menu_tree(0x7000)));
    assert_eq!(names(&emitted(&frame, ElementId::CarLot)), tree_names(&menu_tree(0x7000)));
}

#[test]
fn an_adapter_resolved_before_the_tree_reads_the_tree_set_later() {
    let frame = FrameCell::new();
    let theme = OriginalTheme::new(frame.clone());
    let skin = Skin::new();
    let component = theme.element(ElementId::PauseMenu.slot(), &BuildCx::new(&skin)).expect("PauseMenu is filled");
    let mut before = DrawList::new();
    component.build(&mut before);
    assert!(before.is_empty());
    frame.set_menu(ElementId::PauseMenu.slot(), Arc::new(menu_tree(0x3000)));
    let mut after = DrawList::new();
    component.build(&mut after);
    assert_eq!(names(&after), tree_names(&menu_tree(0x3000)));
}

#[test]
fn the_two_cells_do_not_feed_each_other() {
    let frame = FrameCell::new();
    frame.set(FengSource::Hud, menu_tree(0x4000));
    for slot in MENU_SLOTS {
        assert!(emitted(&frame, slot).is_empty(), "{} read the HUD cell", slot.name());
    }
    frame.set_menu(ElementId::MainMenu.slot(), Arc::new(menu_tree(0x5000)));
    assert!(emitted(&frame, ElementId::Speedometer).is_empty(), "a HUD slot read the menu cell");
}

#[test]
fn a_hidden_node_of_a_menu_tree_is_not_emitted() {
    // The footer hints the prompt layer hides (the legacy "Accept" and "Back") are invisible, not just out of the draw
    // order, so the whole-tree copy must leave them out too.
    let mut tree = menu_tree(0x1000);
    tree.nodes[1].visible = false;
    let frame = FrameCell::new();
    frame.set_menu(ElementId::MainMenu.slot(), Arc::new(tree));
    assert_eq!(names(&emitted(&frame, ElementId::MainMenu)), vec![0x1000, 0x1002]);
}

#[test]
fn two_menu_trees_compose_into_one_list_with_their_links_intact() {
    let frame = FrameCell::new();
    frame.set_menu(ElementId::MainMenu.slot(), Arc::new(menu_tree(0x1000)));
    frame.set_menu(ElementId::Categories.slot(), Arc::new(menu_tree(0x2000)));
    let theme = OriginalTheme::new(frame);
    let skin = Skin::new();
    let mut out = DrawList::new();
    for slot in [ElementId::MainMenu, ElementId::Categories] {
        let component = theme.element(slot.slot(), &BuildCx::new(&skin)).expect("menu slot is filled");
        component.build(&mut out);
    }
    assert_eq!(out.len(), 6);
    assert_eq!(indices(&out), vec![0, 1, 2, 0, 1, 2]);
    // Each child points at its own screen's root, which the list holds at the same offset as the first screen's.
    let parents: Vec<Option<usize>> = out.iter().map(|node| node.parent).collect();
    assert_eq!(parents, vec![None, Some(0), Some(0), None, Some(3), Some(3)]);
}

#[test]
fn the_frame_cell_is_send_and_sync() {
    fn check<T: Send + Sync>() {}
    check::<FrameCell>();
}
