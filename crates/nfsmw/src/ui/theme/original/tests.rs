//! Tests for the Original HUD adapters. A synthetic tree carries the name hashes of the real HUD roots (`hud/roots.rs`)
//! and goes through the frame cell, the way the plugin will fill it. No game data and no GPU are needed.

use blackbox_feng::fe_hash_upper;
use blackbox_ui::{BuildCx, DrawList, LayoutCx, NodeKind, Rect, Skin, SlotId, Theme, UiNode, UiTree};

use crate::hud::roots;
use crate::ui::theme::ElementId;

use super::{FengSource, FrameCell, OriginalTheme};

/// The HUD slots, and the menu slots, which T8 fills. The Custom HUD's slots (13 to 19) are here too: the synthetic
/// package below has none of their objects, so they emit nothing from it (their objects are tested in `compose_tests`).
const HUD_SLOTS: [ElementId; 17] = [
    ElementId::Tachometer,
    ElementId::Speedometer,
    ElementId::GearIndicator,
    ElementId::Minimap,
    ElementId::Blips,
    ElementId::PursuitMeter,
    ElementId::Nitrous,
    ElementId::Turbo,
    ElementId::Speedbreaker,
    ElementId::PoliceRadar,
    ElementId::RaceStatus,
    ElementId::Leaderboard,
    ElementId::BountyRow,
    ElementId::Banner,
    ElementId::Notifications,
    ElementId::WorldMarkers,
    ElementId::VehicleCondition,
];

#[test]
fn the_local_slot_list_is_the_composers() {
    assert_eq!(HUD_SLOTS, super::HUD_SLOTS);
}

fn h(name: &str) -> u32 {
    fe_hash_upper(name)
}

/// The names of the synthetic HUD objects, as the package uses them. The tachometer and speedometer groups, the
/// minimap group, the pursuit meter and cool-down, the nitrous and turbo groups and the speed breaker group are the
/// same hashes `hud/elements.rs` and `hud/bind.rs` use.
fn tach() -> u32 {
    0x5164_D4EA
}
fn speedo() -> u32 {
    0x941F_FF09
}
fn meter() -> u32 {
    0x7B42_2BA3
}
fn cooldown() -> u32 {
    0x84A2_26EC
}
fn nos_group() -> u32 {
    0x87C3_8E97
}
fn nos_bar() -> u32 {
    0xEDFB_6D37
}
fn radar() -> u32 {
    0x839E_7D77
}

fn obj(index: usize, name: u32, parent: Option<usize>, kind: NodeKind, z: f32) -> UiNode {
    UiNode { index, name_hash: name, parent, kind, z, visible: true, colour: [255; 4], ..UiNode::default() }
}

fn group(index: usize, name: u32, parent: Option<usize>) -> UiNode {
    obj(index, name, parent, NodeKind::Group, 0.0)
}

fn text(index: usize, name: u32, parent: Option<usize>, z: f32) -> UiNode {
    obj(index, name, parent, NodeKind::Text { font: 1, justification: 0, leading: 0, max_width: 0 }, z)
}

fn image(index: usize, name: u32, parent: Option<usize>, z: f32) -> UiNode {
    let kind = NodeKind::Image {
        texture: 2,
        uv: [0.0, 0.0, 1.0, 1.0],
        mask: None,
        mask_rotation: [0.5, 0.5, 0.0],
        mask_uv: [0.0, 0.0, 1.0, 1.0],
    };
    obj(index, name, parent, kind, z)
}

/// The tree the producer builds from `nodes`: `Runtime::tree` puts the visible, drawable nodes in front of the screen
/// into `draw_order`, farthest first. Here `index` equals the position, as in the producer's output.
fn tree(nodes: Vec<UiNode>) -> UiTree {
    let mut list = DrawList::new();
    for node in nodes {
        list.push(node);
    }
    list.into_tree(drawn)
}

fn drawn(node: &UiNode) -> bool {
    node.visible && node.z > 0.0 && !matches!(node.kind, NodeKind::Group | NodeKind::Other)
}

/// A HUD-like package. Objects are listed with their index equal to their position:
///
/// - 0 tachometer group, 1 needle (child), 2 gear digit (child of the tachometer).
/// - 3 speedometer group, 4 digit (child).
/// - 5 track-map group, 6 piece (child), 7 blip icon (child of the track map, as the game places it).
/// - 8 pursuit meter (own root), 9 cool-down group, 10 cool-down track (child).
/// - 11 nitrous group, 12 nitrous bar (child).
/// - 13 turbo group, 14 turbo needle (child).
/// - 15 speed breaker group, 16 speed breaker bar (child).
/// - 17 radar group, 18 radar child. The police radar slot claims them.
fn hud_nodes() -> Vec<UiNode> {
    let blip = roots::blip_icons()[0];
    vec![
        group(0, tach(), None),
        image(1, h("3rdPersonNeedle"), Some(0), 2.0),
        text(2, h("3rdPersonGear"), Some(0), 3.0),
        group(3, speedo(), None),
        text(4, h("SPEED_DIGIT_1"), Some(3), 4.0),
        group(5, h("TRACK_MAP"), None),
        image(6, h("TRACK_MAP1"), Some(5), 1.0),
        image(7, blip, Some(5), 1.5),
        image(8, meter(), None, 6.0),
        group(9, cooldown(), None),
        image(10, h("Cooldown_Track"), Some(9), 6.5),
        group(11, nos_group(), None),
        image(12, nos_bar(), Some(11), 2.0),
        group(13, h("TURBO_GROUP"), None),
        image(14, h("3rdperson_TurboDial"), Some(13), 2.0),
        group(15, h("Speedbreaker_Meter_Group"), None),
        image(16, h("Speedbreaker_Meter_Multi_Image"), Some(15), 2.0),
        group(17, radar(), None),
        image(18, h("RadarChild"), Some(17), 2.0),
    ]
}

fn hud_frame(nodes: Vec<UiNode>) -> FrameCell {
    let frame = FrameCell::new();
    frame.set(FengSource::Hud, tree(nodes));
    frame
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

/// The source parent (object index) of the object `index`.
fn source_parent(source: &UiTree, index: usize) -> Option<usize> {
    source.nodes.iter().find(|s| s.index == index).and_then(|s| s.parent)
}

/// Every parent link in `list` points at an earlier node of `list` that is the same object as in the source.
fn assert_links(list: &DrawList, source: &UiTree) {
    let emitted: Vec<&UiNode> = list.iter().collect();
    for (pos, node) in emitted.iter().enumerate() {
        let Some(at) = node.parent else { continue };
        let parent = source_parent(source, node.index);
        assert!(at < pos, "parent of node {} must come before it", node.index);
        assert_eq!(Some(emitted[at].index), parent, "node {} points at the wrong parent", node.index);
    }
}

/// For the output of one adapter: a node whose source parent lies outside its subtree has no parent in `list`, and
/// the parent object is not in `list`. (A composed list holds the parents of other adapters, so this is for one slot.)
fn assert_roots_detached(list: &DrawList, source: &UiTree) {
    for node in list.iter() {
        let Some(outside) = source_parent(source, node.index) else { continue };
        if node.parent.is_some() {
            continue;
        }
        assert!(
            !list.iter().any(|n| n.index == outside),
            "node {} lost its parent {outside} although that parent is in the list",
            node.index
        );
    }
}

/// The draw order of `list` alone: its nodes' object indices, farthest first, as `into_tree` gives them.
fn drawn_order(list: &DrawList) -> Vec<usize> {
    let tree = DrawList::clone(list).into_tree(drawn);
    tree.draw_order.iter().map(|&k| tree.nodes[k].index).collect()
}

/// The draw order of the source tree, limited to the objects in `keep`.
fn expected_order(source: &UiTree, keep: &[usize]) -> Vec<usize> {
    source.draw_order.iter().map(|&k| source.nodes[k].index).filter(|i| keep.contains(i)).collect()
}

#[test]
fn tachometer_emits_its_group_and_not_the_gear() {
    let frame = hud_frame(hud_nodes());
    let list = emitted(&frame, ElementId::Tachometer);
    assert_eq!(names(&list), vec![tach(), h("3rdPersonNeedle")]);
}

#[test]
fn gear_emits_only_the_gear_digit() {
    let frame = hud_frame(hud_nodes());
    let list = emitted(&frame, ElementId::GearIndicator);
    assert_eq!(names(&list), vec![h("3rdPersonGear")]);
    assert!(matches!(list.iter().next().map(|n| &n.kind), Some(NodeKind::Text { .. })));
}

#[test]
fn speedometer_emits_its_group_and_digits() {
    let frame = hud_frame(hud_nodes());
    let list = emitted(&frame, ElementId::Speedometer);
    assert_eq!(indices(&list), vec![3, 4]);
    let kinds: Vec<&NodeKind> = list.iter().map(|n| &n.kind).collect();
    assert!(matches!(kinds[0], NodeKind::Group));
    assert!(matches!(kinds[1], NodeKind::Text { .. }));
}

#[test]
fn minimap_emits_the_track_map_but_not_the_blip() {
    let frame = hud_frame(hud_nodes());
    let list = emitted(&frame, ElementId::Minimap);
    assert_eq!(indices(&list), vec![5, 6]);
}

#[test]
fn blips_emit_only_the_blip_icons() {
    let frame = hud_frame(hud_nodes());
    let list = emitted(&frame, ElementId::Blips);
    assert_eq!(indices(&list), vec![7]);
}

#[test]
fn pursuit_meter_emits_the_meter_and_the_cool_down_subtree() {
    let frame = hud_frame(hud_nodes());
    let list = emitted(&frame, ElementId::PursuitMeter);
    assert_eq!(indices(&list), vec![8, 9, 10]);
}

#[test]
fn nitrous_turbo_and_speedbreaker_each_emit_their_own_root() {
    let frame = hud_frame(hud_nodes());
    assert_eq!(indices(&emitted(&frame, ElementId::Nitrous)), vec![11, 12]);
    assert_eq!(indices(&emitted(&frame, ElementId::Turbo)), vec![13, 14]);
    assert_eq!(indices(&emitted(&frame, ElementId::Speedbreaker)), vec![15, 16]);
}

#[test]
fn copied_nodes_keep_their_fields_and_the_draw_order_of_their_slot() {
    let source = tree(hud_nodes());
    let frame = FrameCell::new();
    frame.set(FengSource::Hud, source.clone());
    for slot in HUD_SLOTS {
        let list = emitted(&frame, slot);
        let members = indices(&list);
        assert_eq!(drawn_order(&list), expected_order(&source, &members), "draw order of {}", slot.name());
        for node in list.iter() {
            let original = &source.nodes[node.index];
            assert_eq!(node.kind, original.kind, "kind of object {}", node.index);
            assert_eq!(node.z, original.z, "z of object {}", node.index);
            assert_eq!(node.colour, original.colour, "colour of object {}", node.index);
        }
    }
}

#[test]
fn parent_links_point_into_the_draw_list() {
    let source = tree(hud_nodes());
    let frame = FrameCell::new();
    frame.set(FengSource::Hud, source.clone());
    for slot in HUD_SLOTS {
        let list = emitted(&frame, slot);
        assert_links(&list, &source);
        assert_roots_detached(&list, &source);
    }
    // The needle's parent is the tachometer, which is the first node of the list.
    let list = emitted(&frame, ElementId::Tachometer);
    assert_eq!(list.iter().nth(1).and_then(|n| n.parent), Some(0));
}

#[test]
fn slots_composed_into_one_list_repeat_no_node_and_keep_their_links() {
    let source = tree(hud_nodes());
    let frame = FrameCell::new();
    frame.set(FengSource::Hud, source.clone());
    let theme = OriginalTheme::new(frame.clone());
    let skin = Skin::new();
    let mut out = DrawList::new();
    for slot in HUD_SLOTS {
        let component = theme.element(slot.slot(), &BuildCx::new(&skin)).expect("HUD slot is filled");
        component.build(&mut out);
    }
    let all = indices(&out);
    let mut unique = all.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), all.len(), "a node was emitted by two slots");
    // The radar is the police radar slot's, so a slot emits it.
    assert!(all.contains(&17) && all.contains(&18), "the radar was not emitted");
    // Every HUD object appears exactly once.
    assert_eq!(unique, (0..19).collect::<Vec<usize>>());
    assert_links(&out, &source);
}

#[test]
fn adapter_appends_after_nodes_already_in_the_list() {
    let source = tree(hud_nodes());
    let frame = FrameCell::new();
    frame.set(FengSource::Hud, source.clone());
    let theme = OriginalTheme::new(frame);
    let skin = Skin::new();
    let mut out = DrawList::new();
    out.push(UiNode { index: 99, ..UiNode::default() });
    let component = theme.element(ElementId::Tachometer.slot(), &BuildCx::new(&skin)).expect("tachometer");
    component.build(&mut out);
    assert_eq!(out.len(), 3, "the existing node stays and two nodes are appended");
    let needle = out.iter().nth(2).expect("needle");
    assert_eq!(needle.parent, Some(1), "the needle's parent is the group, placed after the existing node");
    assert_eq!(out.iter().nth(1).map(|n| n.index), Some(0));
}

#[test]
fn an_absent_root_emits_nothing_and_the_others_still_do() {
    let nodes: Vec<UiNode> = hud_nodes().into_iter().filter(|n| n.name_hash != speedo()).collect();
    let frame = FrameCell::new();
    frame.set(FengSource::Hud, tree(nodes));
    assert!(emitted(&frame, ElementId::Speedometer).is_empty());
    // The digit's parent is gone, so the digit is not in the speedometer; it has no root of its own.
    assert_eq!(indices(&emitted(&frame, ElementId::Tachometer)), vec![0, 1]);
}

#[test]
fn a_hidden_root_emits_nothing_even_when_its_children_are_visible() {
    // The producer hides a child along with its hidden root, so the gear (the gear's own root) is hidden too.
    let mut nodes = hud_nodes();
    nodes[0].visible = false;
    nodes[2].visible = false;
    let frame = hud_frame(nodes.clone());
    assert!(emitted(&frame, ElementId::Tachometer).is_empty());
    assert!(emitted(&frame, ElementId::GearIndicator).is_empty());
    // The needle is a visible child of the hidden tachometer. The root's rule still hides it.
    assert!(nodes[1].visible, "the needle stays visible; only its root is hidden");
    // The other roots are untouched.
    assert_eq!(indices(&emitted(&frame, ElementId::Speedometer)), vec![3, 4]);
}

#[test]
fn an_empty_cell_draws_nothing_for_any_slot() {
    let frame = FrameCell::new();
    for slot in HUD_SLOTS {
        assert!(emitted(&frame, slot).is_empty(), "{} drew with no tree", slot.name());
    }
    assert!(emitted(&frame, ElementId::RadioCard).is_empty());
    assert!(emitted(&frame, ElementId::BustedOverlay).is_empty());
}

#[test]
fn radio_card_and_busted_overlay_emit_their_whole_package() {
    let frame = FrameCell::new();
    let chyron = vec![group(0, h("EA_TRAX_ROOT"), None), text(1, h("TITLE"), Some(0), 2.0)];
    frame.set(FengSource::Chyron, tree(chyron));
    assert_eq!(indices(&emitted(&frame, ElementId::RadioCard)), vec![0, 1]);
    assert!(emitted(&frame, ElementId::BustedOverlay).is_empty(), "the overlay is not set");
    let overlay = vec![group(0, h("BustedOverlay"), None)];
    frame.set(FengSource::BustedOverlay, tree(overlay));
    assert_eq!(indices(&emitted(&frame, ElementId::BustedOverlay)), vec![0]);
    frame.clear(FengSource::Chyron);
    assert!(emitted(&frame, ElementId::RadioCard).is_empty(), "a cleared chyron draws nothing");
}

#[test]
fn clearing_one_source_leaves_the_others_drawn() {
    let frame = hud_frame(hud_nodes());
    frame.set(FengSource::Chyron, tree(vec![group(0, h("EA_TRAX_ROOT"), None)]));
    frame.clear(FengSource::Hud);
    assert!(emitted(&frame, ElementId::Tachometer).is_empty());
    assert_eq!(indices(&emitted(&frame, ElementId::RadioCard)), vec![0]);
}

/// The every-slot invariant (T8, `docs/specs/ui-themes.md` section 7): the Original theme returns a component for every
/// slot of the MW catalogue, HUD, menu and career alike. An empty cell is enough: the adapters exist whether or not a
/// tree is set. `build_themes` runs the same check at start-up.
#[test]
fn every_slot_has_an_original_adapter() {
    let frame = FrameCell::new();
    let theme = OriginalTheme::new(frame);
    let skin = Skin::new();
    for element in ElementId::ALL {
        let filled = theme.element(element.slot(), &BuildCx::new(&skin)).is_some();
        assert!(filled, "the Original theme has no adapter for {}", element.name());
    }
}

#[test]
fn a_slot_outside_the_catalogue_has_no_adapter() {
    let theme = OriginalTheme::new(FrameCell::new());
    let skin = Skin::new();
    assert!(theme.element(SlotId::new(999), &BuildCx::new(&skin)).is_none());
}

#[test]
fn layout_changes_nothing_and_building_twice_gives_the_same_nodes() {
    let frame = hud_frame(hud_nodes());
    let theme = OriginalTheme::new(frame);
    let skin = Skin::new();
    let mut component = theme.element(ElementId::Speedometer.slot(), &BuildCx::new(&skin)).expect("speedometer");
    let before = {
        let mut out = DrawList::new();
        component.build(&mut out);
        indices(&out)
    };
    component.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    component.build(&mut out);
    assert_eq!(indices(&out), before);
    assert_eq!(before, vec![3, 4]);
}
