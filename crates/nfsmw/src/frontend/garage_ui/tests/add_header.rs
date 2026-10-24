//! Native Add Car headings use artwork and suppress duplicate debug strings.

use super::{Harness, Page, model};
use blackbox_feng::{NodeKind, UiTree, fe_hash_upper, runtime::pad};

fn shown(h: &Harness) -> std::sync::Arc<UiTree> {
    h.screens
        .presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard)
        .last()
        .unwrap()
        .tree
        .clone()
}

fn logos(h: &Harness, tree: &UiTree, maker: &str, badge: &str) {
    for (name, texture) in [(0x3e01_ad1d, maker), (0xb05d_d708, badge)] {
        let node = tree.nodes.iter().find(|node| node.name_hash == name).unwrap();
        assert!(node.visible && node.world_colour[3] > 0, "native header {name:08x} must be visible");
        let texture = fe_hash_upper(texture);
        assert!(h.screens.assets().texture(texture).is_some());
        assert!(matches!(node.kind, NodeKind::Image { texture: actual, .. } if actual == texture));
    }
}

fn check(h: &Harness, maker: &str, badge: &str) {
    let tree = shown(h);
    logos(h, &tree, maker, badge);
    let names: Vec<_> = tree.nodes.iter().filter(|node| matches!(node.name_hash, 0xd6d3_2016 | 0x79d6_e45c)).collect();
    assert!(names.len() >= 2, "installed Car_Select includes duplicate car-name strings");
    for node in names {
        assert!(!node.visible, "all native name placeholders must be hidden, GUID {:x}", node.guid);
        assert_eq!(node.text.as_deref(), Some(""));
    }
    assert!(!tree.nodes.iter().any(|node| node.visible && node.text.as_deref() == Some("Debug:")));
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_add_car_wraps_left_right_and_ignores_list_switching_up_down() {
    let mut model = model();
    let mut second = model.cars[0].clone();
    second.folder = "COBALTSS".into();
    second.physics_name = "cobaltss".into();
    second.manufacturer = "Chevrolet".into();
    model.cars.push(second);
    let first = model.draft.car.clone();
    let mut h = Harness::open(Page::AddCar, model);
    for direction in [pad::UP, pad::DOWN] {
        h.press(direction);
        assert_eq!(h.model.draft.car, first, "Customize mode cannot switch the stock-car list");
    }
    h.press(pad::LEFT);
    assert_eq!(h.model.draft.car, "COBALTSS", "left wraps from first to last car");
    h.press(pad::RIGHT);
    assert_eq!(h.model.draft.car, first);
    h.press(pad::RIGHT);
    assert_eq!(h.model.draft.car, "COBALTSS");
    h.press(pad::RIGHT);
    assert_eq!(h.model.draft.car, first, "right wraps from last to first car");
    assert!(h.model.custom_slots.is_empty());
    assert!(h.saved.is_empty());
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_showcase_binds_current_car_and_replaces_native_controller_hints() {
    let mut model = model();
    model.cars[0].manufacturer = "Chevrolet".into();
    model.cars[0].physics_name = "cobaltss".into();
    let h = Harness::open(Page::Showcase, model);
    let tree = shown(&h);
    logos(&h, &tree, "CARSELECT_MANUFACTURER_CHEVROLET", "SECONDARY_LOGO_COBALTSS");
    let control = tree.nodes.iter().position(|node| node.name_hash == 0xd86a_acf8).unwrap();
    assert!(!tree.nodes[control].visible);
    let mut descendants = 0;
    for (index, node) in tree.nodes.iter().enumerate() {
        let mut parent = node.parent;
        while let Some(at) = parent {
            if at == control {
                assert!(!node.visible, "native controller caption or glyph {:x}", node.guid);
                assert!(!tree.draw_order.contains(&index));
                descendants += 1;
                break;
            }
            parent = tree.nodes[at].parent;
        }
    }
    assert!(descendants >= 6, "test must cover the native captions and controller glyphs");
    for text in ["Rotate", "Zoom In", "Zoom Out"] {
        assert!(!tree.nodes.iter().any(|node| node.visible && node.text.as_deref() == Some(text)));
    }
    for text in ["Back", "Zoom", "View Car"] {
        let count = tree.nodes.iter().filter(|node| node.visible && node.text.as_deref() == Some(text)).count();
        assert_eq!(count, 1, "shared Showcase footer must contain one {text} caption");
    }
}

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_garage_add_header_binds_native_manufacturer_and_hides_all_debug_names() {
    let mut model = model();
    let cobalt = &mut model.cars[0];
    cobalt.folder = "COBALTSS".into();
    cobalt.physics_name = "cobaltss".into();
    cobalt.name = "Cobalt SS".into();
    cobalt.manufacturer = "Chevrolet".into();
    let mut ford = cobalt.clone();
    ford.folder = "MUSTANGGT".into();
    ford.physics_name = "mustanggt".into();
    ford.name = "Mustang GT".into();
    ford.manufacturer = "Ford".into();
    model.cars.push(ford);
    model.draft.car = "COBALTSS".into();
    model.applied = model.draft.clone();
    let mut h = Harness::open(Page::AddCar, model);
    check(&h, "CARSELECT_MANUFACTURER_CHEVROLET", "SECONDARY_LOGO_COBALTSS");
    h.press(pad::RIGHT);
    check(&h, "CARSELECT_MANUFACTURER_FORD", "SECONDARY_LOGO_MUSTANGGT");
    h.press(pad::BUTTON0);
    assert_eq!(h.top(), "Showcase.fng");
    h.press(pad::BACK);
    assert_eq!(h.top(), "Car_Select.fng");
    check(&h, "CARSELECT_MANUFACTURER_FORD", "SECONDARY_LOGO_MUSTANGGT");
}
