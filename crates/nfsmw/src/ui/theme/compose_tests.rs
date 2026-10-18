//! Tests for the composer (`compose.rs`): the parity gate and the layers. The Original theme must draw the trees the
//! plugins stored, node for node and in the same draw order. The tests use synthetic trees, so they run with no game
//! data and no GPU.
//!
//! The HUD parity has one documented difference. A slot's root cannot keep its `parent` link when the root's parent lies
//! outside the slot (the gear's root sits inside the tachometer), so its composed `parent` is `None`. Nothing draws from
//! that link after composition (`docs/specs/ui-themes-compose.md`), and the test expects exactly that.

use std::sync::Arc;

use blackbox_feng::tree::is_drawn;
use blackbox_modding::host::{UiPrim, UiRect};
use blackbox_ui::{DrawList, NodeKind, ThemeId, UiNode, UiTree};
use glam::{Mat4, Vec3};

use super::modded::ModTheme;
use super::original::{FengSource, HUD_SLOTS};
use super::{ElementId, UiThemes, hud_layer, slot_layer};
use crate::hud::roots;
use crate::settings::UiStyle;

/// The rear-view mirror's bezel and the radar's group: nodes no HUD slot takes (`hud_slots` module docs).
const MIRROR_BEZEL: u32 = 0x7FAC_287B;
const RADAR_GROUP: u32 = 0x839E_7D77;

/// A node of a synthetic tree. Its world transform carries its index, so a moved node shows in the comparison.
fn node(index: usize, name: u32, parent: Option<usize>, kind: NodeKind, z: f32, visible: bool) -> UiNode {
    UiNode {
        index,
        name_hash: name,
        parent,
        kind,
        z,
        visible,
        colour: [255; 4],
        world_colour: [255; 4],
        world: Mat4::from_translation(Vec3::new(index as f32, 1.0, 0.0)),
        position: Vec3::new(index as f32, 0.0, 0.0),
        ..UiNode::default()
    }
}

fn group(index: usize, name: u32, parent: Option<usize>, z: f32, visible: bool) -> UiNode {
    node(index, name, parent, NodeKind::Group, z, visible)
}

fn image(index: usize, name: u32, parent: Option<usize>, z: f32, visible: bool) -> UiNode {
    let kind = NodeKind::Image {
        texture: index as u32,
        uv: [0.0, 0.0, 1.0, 1.0],
        mask: None,
        mask_rotation: [0.5, 0.5, 0.0],
        mask_uv: [0.0, 0.0, 1.0, 1.0],
    };
    node(index, name, parent, kind, z, visible)
}

fn text(index: usize, name: u32, parent: Option<usize>, z: f32) -> UiNode {
    let kind = NodeKind::Text { font: 1, justification: 0, leading: 0, max_width: 0 };
    node(index, name, parent, kind, z, true)
}

/// Builds a tree the way the runtime does: nodes in object order, draw order by `is_drawn`.
fn tree_of(nodes: Vec<UiNode>) -> UiTree {
    let mut list = DrawList::new();
    for node in nodes {
        list.push(node);
    }
    list.into_tree(is_drawn)
}

/// The HUD of the tests, in object order. The index of each node is its position.
///
/// - 0 tachometer root, 1 needle, 4 face. 2 gear root (inside the tachometer), 3 its digit.
/// - 5 the mirror (no slot), 6 its bezel.
/// - 7 speedometer root, 8 its needle. 9 the radar (no slot), 10 its picture.
/// - 11 minimap root, 13 its track. 12 a blip icon under the minimap (the blips slot takes it).
/// - 14 nitrous root, hidden, 15 its picture: hidden with its root, so no slot draws it.
/// - 16 an object the runtime does not draw (no slot).
fn synthetic_hud() -> UiTree {
    let tach = roots::tachometer().roots[0];
    let gear = roots::gear().roots[0];
    let speedo = roots::speedometer().roots[0];
    let minimap = roots::minimap().roots[0];
    let blip = roots::blip_icons().first().copied().expect("the blip pool has icons");
    let nitrous = roots::nitrous().roots[0];
    tree_of(vec![
        group(0, tach, None, 0.0, true),
        image(1, 0x1111_0001, Some(0), 3.0, true),
        group(2, gear, Some(0), 0.0, true),
        text(3, 0x1111_0003, Some(2), 4.0),
        image(4, 0x1111_0004, Some(0), 2.0, true),
        group(5, 0x7FAC_0000, None, 0.0, true),
        image(6, MIRROR_BEZEL, Some(5), 1.0, true),
        group(7, speedo, None, 0.0, true),
        image(8, 0x1111_0008, Some(7), 2.0, true),
        group(9, RADAR_GROUP, None, 0.0, true),
        image(10, 0x1111_000A, Some(9), 5.0, true),
        group(11, minimap, None, 0.0, true),
        image(12, blip, Some(11), 2.0, true),
        image(13, 0x1111_000D, Some(11), 1.0, true),
        group(14, nitrous, None, 0.0, false),
        image(15, 0x1111_000F, Some(14), 2.0, false),
        node(16, 0x1111_0010, None, NodeKind::Other, 9.0, true),
    ])
}

/// The slot roots whose parent lies outside their slot: the gear's root (inside the tachometer) and the blip icon (inside
/// the minimap). Their composed `parent` is `None`, the one documented difference of the composed tree.
const GEAR_ROOT: usize = 2;
const BLIP_ROOT: usize = 12;

/// Asserts that `actual` holds the same nodes as `expected`, in the same order, with the same draw order.
fn assert_same(actual: &UiTree, expected: &UiTree) {
    assert_eq!(actual.nodes.len(), expected.nodes.len(), "node count");
    for (at, (a, e)) in actual.nodes.iter().zip(&expected.nodes).enumerate() {
        assert_eq!(format!("{a:?}"), format!("{e:?}"), "node at {at}");
    }
    assert_eq!(actual.draw_order, expected.draw_order, "draw order");
}

/// The input with the documented parent difference applied: the slot roots whose parent lies outside them have none.
fn expected_hud() -> UiTree {
    let mut expected = synthetic_hud();
    for root in [GEAR_ROOT, BLIP_ROOT] {
        expected.nodes[root].parent = None;
    }
    expected
}

fn has_name(tree: &UiTree, name: u32) -> bool {
    tree.nodes.iter().any(|node| node.name_hash == name)
}

#[test]
fn original_hud_layer_is_the_hud_tree_node_for_node() {
    let themes = UiThemes::build();
    themes.frame().set(FengSource::Hud, synthetic_hud());
    assert_same(&hud_layer(&themes, 0.0), &expected_hud());
}

#[test]
fn the_hud_slots_claim_each_node_at_most_once() {
    let counts = super::original::claim_counts(&synthetic_hud());
    assert!(counts.iter().all(|count| *count <= 1), "a node is claimed twice: {counts:?}");
    // The gear digit is claimed by the gear slot alone, not by the tachometer that holds it.
    assert_eq!(counts[3], 1);
    // The mirror, the radar and the object the runtime does not draw are claimed by no slot.
    assert_eq!(counts[5], 0);
    assert_eq!(counts[9], 0);
    assert_eq!(counts[16], 0);
}

#[test]
fn the_composed_hud_draws_the_nodes_no_slot_takes() {
    let themes = UiThemes::build();
    themes.frame().set(FengSource::Hud, synthetic_hud());
    let composed = hud_layer(&themes, 0.0);
    assert!(has_name(&composed, MIRROR_BEZEL), "the mirror bezel is lost");
    assert!(has_name(&composed, RADAR_GROUP), "the radar group is lost");
    let drawn: Vec<u32> = composed.draw_order.iter().map(|&i| composed.nodes[i].name_hash).collect();
    assert!(drawn.contains(&MIRROR_BEZEL), "the mirror bezel is not drawn");
    assert!(drawn.contains(&0x1111_000A), "the radar picture is not drawn");
}

#[test]
fn a_hidden_root_keeps_its_nodes_but_draws_none_of_them() {
    let themes = UiThemes::build();
    themes.frame().set(FengSource::Hud, synthetic_hud());
    let composed = hud_layer(&themes, 0.0);
    assert_eq!(composed.nodes.len(), synthetic_hud().nodes.len(), "a node is dropped");
    assert!(!composed.draw_order.iter().any(|&i| composed.nodes[i].name_hash == 0x1111_000F));
}

#[test]
fn the_custom_theme_fills_the_drive_readouts_and_keeps_the_other_hud_slots_original() {
    let mut themes = UiThemes::build();
    themes.select(UiStyle::Custom);
    themes.frame().set(FengSource::Hud, synthetic_hud());
    assert_eq!(themes.provided_by(ElementId::Tachometer), Some(ThemeId::Custom));
    assert_eq!(themes.provided_by(ElementId::Speedometer), Some(ThemeId::Custom));
    assert_eq!(themes.provided_by(ElementId::GearIndicator), Some(ThemeId::Custom));
    for element in super::original::HUD_SLOTS {
        if matches!(
            element,
            ElementId::Tachometer
                | ElementId::Speedometer
                | ElementId::GearIndicator
                | ElementId::Nitrous
                | ElementId::Turbo
                | ElementId::Speedbreaker
        ) {
            continue;
        }
        assert_eq!(themes.provided_by(element), Some(ThemeId::Original), "{}", element.name());
    }
    // The Original tachometer and speedometer nodes are gone: the Custom readouts replace them.
    let composed = hud_layer(&themes, 0.0);
    assert!(!has_name(&composed, roots::tachometer().roots[0]), "the Original tachometer is still drawn");
    assert!(!has_name(&composed, roots::speedometer().roots[0]), "the Original speedometer is still drawn");
}

#[test]
fn a_mod_theme_replaces_only_the_slot_it_names() {
    let tach_panel =
        UiPrim::Panel { rect: UiRect { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }, colour: [10, 20, 30, 255], texture: None };
    let mut theme = ModTheme::new(ThemeId::Mod(7), "tach", "Tach");
    theme.set_slot(ElementId::Tachometer.slot(), vec![tach_panel]);
    let mut themes = UiThemes::build();
    themes.set_mod_themes(vec![theme]);
    themes.select_mod(ThemeId::Mod(7)).expect("the mod theme is registered");
    themes.frame().set(FengSource::Hud, synthetic_hud());

    let composed = hud_layer(&themes, 0.0);
    assert_eq!(themes.provided_by(ElementId::Tachometer), Some(ThemeId::Mod(7)));
    // The tachometer's own nodes are gone, the mod's panel is there, and the gear and the rest of the HUD remain.
    assert!(!has_name(&composed, roots::tachometer().roots[0]), "the tachometer root is still drawn");
    assert!(!has_name(&composed, 0x1111_0001), "the tachometer needle is still drawn");
    assert!(composed.nodes.iter().any(|n| n.colour == [10, 20, 30, 255]), "the mod's panel is not drawn");
    assert!(has_name(&composed, roots::gear().roots[0]), "the gear was dropped with the tachometer");
    assert!(has_name(&composed, MIRROR_BEZEL), "the mirror was dropped with the tachometer");
}

#[test]
fn the_original_radio_layer_is_the_chyron_package() {
    let themes = UiThemes::build();
    let chyron = tree_of(vec![group(0, 0x2222_0000, None, 0.0, true), image(1, 0x2222_0001, Some(0), 1.0, true)]);
    themes.frame().set(FengSource::Chyron, chyron.clone());
    assert_eq!(themes.provided_by(ElementId::RadioCard), Some(ThemeId::Original));
    assert_same(&slot_layer(&themes, ElementId::RadioCard, 0.0), &chyron);
}

#[test]
fn the_custom_radio_layer_draws_no_chyron() {
    let mut themes = UiThemes::build();
    themes.select(UiStyle::Custom);
    let chyron = tree_of(vec![image(0, 0x2222_0001, None, 1.0, true)]);
    themes.frame().set(FengSource::Chyron, chyron);
    // The Custom card replaces the slot, so the chyron is not what the layer draws, and an idle card draws nothing.
    assert_eq!(themes.provided_by(ElementId::RadioCard), Some(ThemeId::Custom));
    assert!(slot_layer(&themes, ElementId::RadioCard, 0.0).nodes.is_empty());
}

#[test]
fn the_busted_overlay_layer_is_the_overlay_tree() {
    let themes = UiThemes::build();
    let overlay = tree_of(vec![group(0, 0x3333_0000, None, 0.0, true), image(1, 0x3333_0001, Some(0), 1.0, true)]);
    themes.frame().set(FengSource::BustedOverlay, overlay.clone());
    assert_same(&slot_layer(&themes, ElementId::BustedOverlay, 0.0), &overlay);
}

/// A menu screen's tree: a root group with a text and an image under it.
fn menu_tree(tag: u32) -> UiTree {
    tree_of(vec![group(0, tag, None, 0.0, true), text(1, tag + 1, Some(0), 2.0), image(2, tag + 2, Some(0), 1.0, true)])
}

#[test]
fn each_menu_slot_is_its_screen_tree_node_for_node() {
    let themes = UiThemes::build();
    for (at, slot) in ElementId::ALL.iter().copied().filter(|slot| slot.is_menu()).enumerate() {
        let tree = menu_tree(0x4000_0000 + 0x100 * at as u32);
        themes.frame().set_menu(slot.slot(), Arc::new(tree.clone()));
        assert_same(&slot_layer(&themes, slot, 0.0), &tree);
    }
}

#[test]
fn a_cleared_menu_slot_draws_nothing() {
    let themes = UiThemes::build();
    themes.frame().set_menu(ElementId::MainMenu.slot(), Arc::new(menu_tree(0x4000_0000)));
    themes.frame().clear_menu(ElementId::MainMenu.slot());
    assert!(slot_layer(&themes, ElementId::MainMenu, 0.0).nodes.is_empty());
}

#[test]
fn an_empty_hud_cell_draws_no_hud() {
    let themes = UiThemes::build();
    assert!(hud_layer(&themes, 0.0).nodes.is_empty());
}

#[test]
fn the_hud_layer_slots_are_hud_slots_of_the_catalogue() {
    // The composer builds these in order; each must be a HUD slot, not a menu or a radio slot.
    for slot in HUD_SLOTS {
        assert!(!slot.is_menu(), "{} is a menu slot", slot.name());
        assert!(!matches!(slot, ElementId::RadioCard | ElementId::BustedOverlay));
    }
}
